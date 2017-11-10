#![feature(used)]
#[macro_use]
extern crate zcfg;
extern crate zcfg_flag_parser;
extern crate jobs;
#[macro_use]
extern crate lazy_static;
extern crate fern;
extern crate chrono;
extern crate log;
extern crate common;

use jobs::LcsFetcherJob;
use jobs::Job;
use std::collections::HashMap;
use std::ops::Deref;

define_pub_cfg!(job_to_run,
                ::zcfg::NoneableCfg<String>,
                None,
                "Which job should be run.");
define_pub_cfg!(fetch_destination,
                String,
                "s3",
                "Where to fetch the crates into (cwd or s3)");

lazy_static! {
  /**
   * A mapping from job name (flag) to a function yielding the job.
   *
   * A level of indirection is needed to prevent constructing the job unless we need it. Some jobs
   * have mandatory flags that we want to avoid evaluating unless the job is requested.
   */
  static ref JOBS: HashMap<&'static str, fn() -> Box<Job>> = {
    let mut jobs: HashMap<&'static str, fn() -> Box<Job>> = HashMap::new();
    jobs.insert("lcs-fetcher", get_lcs_fetcher);
    jobs
  };

  /** A list of all job keys. */
  static ref ALL_JOBS: Vec<&'static str> = {
    JOBS.keys()
      .cloned()
      .collect()
  };
}

fn main() {
  common::init();

  let job_to_run = ::job_to_run::CONFIG.get_value().inner()
    .expect("--job_to_run must be specified");

  let job_thunk = JOBS.get(job_to_run.as_str())
    .expect(&format!("Unknown job name {}, known jobs are {:?}",
                     job_to_run,
                     ALL_JOBS.deref()));

  job_thunk().run();
}

fn get_lcs_fetcher() -> Box<Job> {
  let destination = ::fetch_destination::CONFIG.get_value();
  match destination.as_str() {
    "s3" => Box::new(LcsFetcherJob::from_crates_io_to_s3()),
    "cwd" => Box::new(LcsFetcherJob::from_crates_io_to_cwd()),
    other => panic!("Unknown --fetch_destination \"{}\"", other),
  }
}
