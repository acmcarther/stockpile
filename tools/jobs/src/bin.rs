#![feature(used)]
#[macro_use]
extern crate zcfg
#[macro_use]
extern crate zcfg_flag_parser
extern crate jobs;
extern crate lazy_static;

use jobs::CrateClonerJob;

fn main() {
  FlagParser::new().parse_from_args(env::args().skip(1));

  let job = CrateCloneJob::default();

  job.run();
}
