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

use std::io;
use aws_sdk_rust::aws::errors::s3::S3Error;

#[derive(Debug)]
pub enum JobErr {
  IoErr(io::Error),
  HyperErr(hyper::Error),
  SerdeJsonErr(serde_json::Error),
  S3Err(S3Error),
  GitErr(git2::Error),
  UnsupportedOperation,
}
define_from_error_boilerplate!(io::Error, JobErr, JobErr::IoErr);
define_from_error_boilerplate!(hyper::Error, JobErr, JobErr::HyperErr);
define_from_error_boilerplate!(serde_json::Error, JobErr, JobErr::SerdeJsonErr);
define_from_error_boilerplate!(git2::Error, JobErr, JobErr::GitErr);
define_from_error_boilerplate!(S3Error, JobErr, JobErr::S3Err);

pub trait Job {
  fn run(&mut self);
}

pub use lcs_fetcher::LcsFetcherJob;
pub use ais_backfiller::AisBackfillerJob;
