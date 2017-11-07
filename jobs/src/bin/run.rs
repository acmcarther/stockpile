#![feature(used)]
#[macro_use]
extern crate zcfg;
#[macro_use]
extern crate zcfg_flag_parser;
extern crate jobs;
#[macro_use]
extern crate lazy_static;
extern crate fern;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate common;

use jobs::LcsFetcherJob;
use jobs::Job;
use std::collections::HashMap;

define_pub_cfg!(job_to_run,
                ::zcfg::NoneableCfg<String>,
                None,
                "Which job should be run.");

fn main() {
  common::init_flags();
  common::init_logger();

  let mut jobs = get_all_jobs();

  let job_to_run = ::job_to_run::CONFIG.get_value().inner()
    .expect("--job_to_run must be specified");

  let all_jobs: Vec<String> = jobs.keys()
    .cloned()
    .map(str::to_owned)
    .collect();

  let job = jobs.get_mut(job_to_run.as_str())
    .expect(&format!("Unknown job name {}, known jobs are {:?}", job_to_run, all_jobs));

  job().run();
}

fn get_lcs_fetcher() -> Box<Job> {
  Box::new(LcsFetcherJob::default())
}

/** Enumerates all jobs with a job_to_run key */
fn get_all_jobs() -> HashMap<&'static str, fn () -> Box<Job>> {
  let mut jobs: HashMap<&'static str, fn () -> Box<Job>> = HashMap::new();

  jobs.insert("lcs-fetcher", get_lcs_fetcher);

  return jobs;
}
