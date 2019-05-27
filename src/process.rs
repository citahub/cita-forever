use crate::config::{ForeverConfig, ProcessConfig};
use channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::default::Default;
use std::fs::remove_file;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::io::{BufReader, Read};
use std::process::Child;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use util::Mutex;

#[derive(Debug, Default)]
pub struct Processes {
    pub processcfg: ProcessConfig,
    pub processhandle: Option<Child>,
    pub children: HashMap<String, Arc<Mutex<Processes>>>,
}

impl Processes {
    pub fn new(foreverconfig: ForeverConfig) -> Self {
        let parentcfg = ProcessConfig {
            name: foreverconfig.name.clone(),
            command: foreverconfig.command.clone(),
            args: foreverconfig.args.clone(),
            pidfile: foreverconfig.pidfile.clone(),
            ..Default::default()
        };

        let processcfg = foreverconfig.process.unwrap();
        let mut children_processes = HashMap::new();

        for cfg in &processcfg {
            let child_name = cfg.name.clone().unwrap();
            let child_cfg = cfg.clone();

            let child_inner_process = HashMap::new();
            let child_handle = None;
            let child_process = Processes {
                processcfg: child_cfg,
                processhandle: child_handle,
                children: child_inner_process,
            };
            children_processes.insert(child_name, Arc::new(Mutex::new(child_process)));
        }

        Processes {
            processcfg: parentcfg,
            processhandle: None,
            children: children_processes,
        }
    }

    // find child process
    pub fn find_process(&mut self) -> Option<u32> {
        if self.processcfg.pidfile == None {
            let name = self.processcfg.name.clone().unwrap();
            warn!("{} pidfile path is null", name);
            return None;
        }

        let pidfile_clone = self.processcfg.pidfile.clone();
        let pidfile = pidfile_clone.unwrap();
        check_process(pidfile)
    }

    // start parent process
    pub fn start(&mut self) {
        let command = self.processcfg.command.clone().unwrap();
        let arg_null: Vec<String> = Vec::new();
        let args = self.processcfg.args.clone().unwrap_or(arg_null);
        let child = Command::new(command)
            .args(args)
            .spawn()
            .expect("failed to execute child");

        info!("process id: {}", child.id());

        self.processcfg.pid = Some(child.id());

        // record pid
        let pid = child.id();
        let pidfile = self.processcfg.pidfile.clone().unwrap();
        write_pid(pidfile, pid);

        // record process handle
        self.processhandle = Some(child);

        // record process status
        let name = self.processcfg.name.clone().unwrap();
        info!("{} started", name);
    }

    // run all child processes
    pub fn start_all(self) {
        let (tx, rx): (Sender<String>, Receiver<String>) = unbounded();

        for (_, child_process) in self.children {
            let tx = tx.clone();
            run_process(child_process, tx);
        }

        loop {
            let ret = rx.recv().unwrap();
            warn!("{}", ret);
        }
    }

    // stop process
    pub fn stop(&mut self) {
        let name = self.processcfg.name.clone().unwrap();
        let pidfile = self.processcfg.pidfile.clone().unwrap();
        match self.find_process() {
            Some(pid) => {
                let pid_str = pid.to_string();
                let args = vec!["-9", &pid_str];
                let status = Command::new("kill").args(args).status();
                match status {
                    Ok(exit_status) if exit_status.success() => {
                        info!("kill {} {} ok", name, &pid_str);
                    }
                    _ => info!("kill {} {} failed", name, &pid_str),
                }
            }
            None => {
                warn!("{} not started", name);
            }
        }
        delete_pidfile(pidfile);
    }

    // stop all processes
    pub fn stop_all(mut self) {
        // stop parent process
        self.stop();

        // stop all child process
        for (_, child_process) in self.children {
            let mut process = child_process.lock();
            process.stop();
        }
    }

    // all child processes logrotate
    pub fn logrotate(self) {
        for (_, child_process) in self.children {
            let mut process = child_process.lock();
            let name = process.processcfg.name.clone().unwrap();
            match process.find_process() {
                Some(pid) => {
                    let pid_str = pid.to_string();
                    //send signal(SIGUSR1) to child processes
                    let args = vec!["-10", &pid_str];
                    let status = Command::new("kill").args(args).status();
                    match status {
                        Ok(exit_status) if exit_status.success() => {
                            info!("logrotate {} {} ok", name, &pid_str);
                        }
                        _ => info!("logrotate {} {} failed", name, &pid_str),
                    }
                }
                None => {
                    warn!("{} not started", name);
                }
            }
        }
    }
}

// run child process
pub fn run_process(child_process: Arc<Mutex<Processes>>, tx: Sender<String>) {
    thread::spawn(move || {
        loop {
            {
                // wait until process exit,then restart process
                let process_wait = child_process.clone();
                let mut process = process_wait.lock();

                let name = process.processcfg.name.clone().unwrap();
                let pidfile = process.processcfg.pidfile.clone().unwrap();

                // check process exsit
                if let Some(pid) = process.find_process() {
                    warn!("{} already started,pid is {}", name, pid);
                    return;
                }

                // start child process
                process.start();

                match process.processhandle {
                    // wait here, except error occurs
                    Some(ref mut child) => match child.wait() {
                        Ok(_status) => {
                            warn!("{} exit status is {:?}", name, _status);
                            delete_pidfile(pidfile);
                        }
                        Err(e) => {
                            warn!("{} processhandle error {}", name, e);
                            delete_pidfile(pidfile);
                            return;
                        }
                    },
                    None => {
                        // almost never happen
                        delete_pidfile(pidfile);
                        warn!("{} processHandle is None", name);
                        return;
                    }
                }
            }

            if !change_status(&child_process) {
                // inform cita-forever error occurs.
                let name = child_process.lock().processcfg.name.clone().unwrap();
                tx.send(format!("==>: Child process {} exited unexpectedly!", name))
                    .unwrap();
                return;
            }
        }
    });
}

// change child status..
pub fn change_status(child_process: &Arc<Mutex<Processes>>) -> bool {
    let process_temp = child_process.clone();
    let mut process = process_temp.lock();

    let process_name = process.processcfg.name.clone().unwrap();

    // repawns++
    process.processcfg.respawns = process.processcfg.respawns.unwrap_or(0).checked_add(1);

    // reach max respawn times,default:3 times
    if process.processcfg.respawns.unwrap() > process.processcfg.respawn.unwrap_or(3) {
        warn!("{} reach max respawn limit", process_name);
        return false;
    }
    true
}

// write pid to the path file
pub fn write_pid(path: String, pid: u32) {
    let mut pid_file: File = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("pid file path error");
    pid_file
        .write_fmt(format_args!("{}", pid))
        .expect("write pid failed");
}
// read pid from the path file
pub fn read_pid(path: String) -> u32 {
    match File::open(path) {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader
                .read_to_string(&mut contents)
                .expect("read pid file failed");
            contents
                .parse::<u32>()
                .expect("parse pid error from pid file")
        }
        Err(_) => 0,
    }
}

// delete pid file
pub fn delete_pidfile(path: String) {
    let file = path.clone();

    match remove_file(path) {
        Ok(_) => info!("Delete pid file {} success.", file),
        Err(e) => match e.kind() {
            ErrorKind::NotFound => info!("{} not found.", file),
            _ => warn!("Delete pid file {} failed!", file),
        },
    }
}

// whether process exsit
fn check_process(pidfile: String) -> Option<u32> {
    // read pid from pidfile
    let pid: u32 = read_pid(pidfile);
    if pid == 0 {
        None
    } else {
        let pid_str = pid.to_string();
        let args = vec!["-p", &pid_str, "-o", "pid="];
        let output = Command::new("ps")
            .args(args)
            .output()
            .expect("failed to execute ps -p");
        if output.status.success() {
            let otpt_str = String::from_utf8(output.stdout).unwrap();
            if otpt_str.contains(&pid.to_string()) {
                Some(pid)
            } else {
                None
            }
        } else {
            None
        }
    }
}
