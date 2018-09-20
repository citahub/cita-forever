// CITA
// Copyright 2016-2017 Cryptape Technologies LLC.

// This program is free software: you can redistribute it
// and/or modify it under the terms of the GNU General Public
// License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any
// later version.

// This program is distributed in the hope that it will be
// useful, but WITHOUT ANY WARRANTY; without even the implied
// warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

extern crate clap;
#[macro_use]
extern crate logger;
#[macro_use]
extern crate util;

#[macro_use]
extern crate serde_derive;

pub mod config;
pub mod process;

use clap::{App, Arg, SubCommand};
use config::ForeverConfig;
use process::Processes;

use std::env;

include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

fn main() {
    // Always print backtrace on panic.
    env::set_var("RUST_BACKTRACE", "full");

    logger::init_config("cita-forever");
    info!("CITA:forever:cita-forever");
    info!("Version: {}", get_build_info_str(true));

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

    let config_file = matches.value_of("config").unwrap_or("forever.toml");
    let config = ForeverConfig::new(config_file);
    let mut daemon: Processes = Processes::new(config);

    match matches.subcommand_name() {
        Some("start") => match daemon.find_process() {
            Some(pid) => {
                let name = daemon.processcfg.name.clone().unwrap();
                warn!("{} already started,pid is {}", name, pid);
                return;
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
