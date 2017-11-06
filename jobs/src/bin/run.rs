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

use jobs::CrateClonerJob;
use jobs::Job;

define_pub_cfg!(job_to_run,
                ::zcfg::NoneableCfg<String>,
                None,
                "Which job should be run.");

fn main() {
  common::init_flags();
  common::init_logger();

  let job_to_run = ::job_to_run::CONFIG.get_value().inner()
    .expect("--job_to_run must be specified");

  match job_to_run.as_ref() {
    "clone_crates" => CrateClonerJob::default().run(),
    name => error!("Unknown job name {}", name)
  }
}
