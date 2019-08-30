// Copyright Cryptape Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate crossbeam_channel as channel;
#[macro_use]
extern crate cita_logger;
#[macro_use]
extern crate util;

#[macro_use]
extern crate serde_derive;

pub mod config;
pub mod process;

use crate::config::ForeverConfig;
use crate::process::Processes;
use clap::{App, Arg, SubCommand};

use crate::cita_logger::{init_config, LogFavour};
use std::env;

include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

fn main() {
    // Always print backtrace on panic.
    env::set_var("RUST_BACKTRACE", "full");

    let matches = App::new("Forever")
        .version(get_build_info_str(true))
        .long_version(get_build_info_str(false))
        .author("Cryptape")
        .about("Forever the processes")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Set forever.yaml")
                .takes_value(true),
        )
        .arg_from_usage("-s, --stdout 'Log to console'")
        .subcommand(
            SubCommand::with_name("start")
                .about("Start all proccesses in the background")
                .version("0.1")
                .author("Cryptape"),
        )
        .subcommand(
            SubCommand::with_name("stop")
                .about("Stop all proccesses")
                .version("0.1")
                .author("Cryptape"),
        )
        .subcommand(
            SubCommand::with_name("logrotate")
                .about("rotate logs")
                .version("0.1")
                .author("Cryptape"),
        )
        .subcommand(
            SubCommand::with_name("")
                .about("Start all proccesses in the foreground")
                .version("0.1")
                .author("Cryptape"),
        )
        .get_matches();

    let favour = if matches.is_present("stdout") {
        LogFavour::Stdout("cita-forever")
    } else {
        LogFavour::File("cita-forever")
    };
    init_config(&favour);
    info!("CITA:forever:cita-forever");
    info!("Version: {}", get_build_info_str(true));

    let config_file = matches.value_of("config").unwrap_or("forever.toml");
    let config = ForeverConfig::new(config_file);
    info!("config_file: {:?}", config);
    let mut daemon: Processes = Processes::new(config);

    match matches.subcommand_name() {
        Some("start") => match daemon.find_process() {
            Some(pid) => {
                let name = daemon.processcfg.name.clone().unwrap();
                warn!("{} already started,pid is {}", name, pid);
            }
            None => daemon.start(),
        },
        Some("stop") => daemon.stop_all(),
        Some("logrotate") => daemon.logrotate(),
        Some(&_) => {}
        None => {
            daemon.start_all();
        }
    }
}
