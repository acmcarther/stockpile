#![feature(used)]
#![allow(dead_code)]
extern crate aws_sdk_rust;
#[macro_use] extern crate common;
extern crate git2;
#[macro_use] extern crate derive_builder;
extern crate hyper;
#[macro_use(log, debug, info, warn)] extern crate log;
extern crate serde_json;
extern crate rayon;
extern crate scoped_threadpool;
extern crate serde;
#[macro_use] extern crate lazy_static;
extern crate tempdir;
extern crate url;
#[macro_use] extern crate zcfg;

mod aws;
mod index;
mod lcs_fetcher;
mod ais_backfiller;

pub trait Job {
  fn run(&mut self);
}

pub use lcs_fetcher::LcsFetcherJob;
pub use ais_backfiller::AisBackfillerJob;
