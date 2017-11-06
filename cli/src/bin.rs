extern crate common;
extern crate cli;
extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;
extern crate clap;
extern crate fern;
extern crate chrono;

use clap::Arg;
use clap::App;
use clap::SubCommand;
use clap::ArgMatches;
use cli::commands;
use cli::commands::SnapshotNowParams;
use cli::commands::QueryParams;
use cli::commands::TryAddingParams;
use std::path::PathBuf;

fn main() {
  fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!("{}[{}][{}] {}",
          chrono::Local::now()
              .format("[%Y-%m-%d][%H:%M:%S]"),
          record.target(),
          record.level(),
          message))
    })
    .level(log::LogLevelFilter::Debug)
    .chain(std::io::stdout())
    .apply()
    .unwrap();

  let matches = App::new("stock-cli")
    .subcommand(SubCommand::with_name("snapshot_now")
                .about("Attempt to resolve the latest versions of provided dependencies.")
                .arg(Arg::with_name("repo_directory")
                     .long("repo_directory")
                     .takes_value(true)))
    .subcommand(SubCommand::with_name("query")
                .about("Retrieve details about the given crate in the provided snapshot.")
                .arg(Arg::with_name("snapshot_version")
                     .long("snapshot_version")
                     .takes_value(true))
                .arg(Arg::with_name("repo_directory")
                     .long("repo_directory")
                     .takes_value(true))
                .arg(Arg::with_name("crate_name")
                     .required(true)))
    .subcommand(SubCommand::with_name("try_adding")
                .about("Attempt a naive preview of an addition of a crate into the current snapshot.")
                .arg(Arg::with_name("repo_directory")
                     .long("repo_directory")
                     .takes_value(true))
                .arg(Arg::with_name("snapshot_version")
                     .long("snapshot_version")
                     .takes_value(true))
                .arg(Arg::with_name("crate_name")
                     .required(true)))
    .get_matches();

  match matches.subcommand() {
    ("snapshot_now", Some(sub_matches)) => run_snapshot_now(sub_matches),
    ("query", Some(sub_matches)) => run_query(sub_matches),
    ("try_adding", Some(sub_matches)) => run_try_adding(sub_matches),
    _ => println!("No command matched.")
  }
}

fn run_snapshot_now(arg_matches: &ArgMatches) {
  let params = SnapshotNowParams {
    repo_directory: arg_matches.value_of("repo_directory").map(PathBuf::from),
  };

  commands::snapshot_now(params);
}

fn run_query(arg_matches: &ArgMatches) {
  let params = QueryParams {
    snapshot_version: arg_matches.value_of("snapshot_version").map(ToOwned::to_owned),
    repo_directory: arg_matches.value_of("repo_directory").map(PathBuf::from),
    crate_name: arg_matches.value_of("crate_name").map(ToOwned::to_owned).unwrap(),
  };

  commands::query(params);
}

fn run_try_adding(arg_matches: &ArgMatches) {
  let params = TryAddingParams {
    snapshot_version: arg_matches.value_of("snapshot_versin").map(ToOwned::to_owned),
    repo_directory: arg_matches.value_of("repo_directory").map(PathBuf::from),
    crate_name: arg_matches.value_of("crate_name").map(ToOwned::to_owned).unwrap(),
  };

  commands::try_adding(params);
}
