#![feature(used)]
#![allow(dead_code)]
extern crate aws_sdk_rust;
extern crate common;
extern crate git2;
extern crate hyper;
#[macro_use(log, debug, warn)]
extern crate log;
extern crate serde_json;
extern crate rayon;
extern crate serde;
#[macro_use] extern crate lazy_static;
extern crate tempdir;
extern crate url;
#[macro_use] extern crate zcfg;

mod flags {
  define_pub_cfg!(s3_access_key_id,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 access key id credential to be used."); 
  define_pub_cfg!(s3_secret_access_key,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 secret key credential to be used.");
  define_pub_cfg!(s3_api_url,
                  String,
                  "http://minio-small".to_owned(),
                  "The location of the local crate service S3 Bucket.");
  define_pub_cfg!(s3_crate_bucket_name,
                  String,
                  "local-crate-service",
                  "The name of the S3 bucket where crates are stored.");
  define_pub_cfg!(crates_io_index_url,
                  String,
                  "https://github.com/rust-lang/crates.io-index",
                  "The URL for the upstream crates.io index repository");
  define_pub_cfg!(pre_pulled_crates_io_index_directory,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The path to the crates.io index to use in lieu of pulling a fresh copy.");
  define_pub_cfg!(max_session_crates,
                  u32,
                  1000u32,
                  "The maximum number of crates to download in a single execution of crate cloner.");
}

mod aws;
mod index;

use aws_sdk_rust::aws::s3::object::ListObjectsRequest;
use aws_sdk_rust::aws::s3::object::ListObjectsOutput;
use aws::SimpleS3Client;
use tempdir::TempDir;
use index::UpstreamIndex;
use index::UpstreamIndexParams;

pub trait Job {
  fn run(self);
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CrateKey {
  name: String,
  version: String,
}

pub struct CrateClonerJob {
  s3_bucket_name: String,
  s3_client: SimpleS3Client,
  upstream_index: UpstreamIndex,
}

impl CrateClonerJob {
  fn run_now(&mut self) {
    self.debug();
    let existing_crates = self.get_existing_crates_from_storage();
  }

  fn debug(&self) {
    use aws_sdk_rust::aws::s3::writeparse::ListBucketsOutput;

    let o = self.s3_client.inner_client.list_buckets().unwrap();
    warn!("o: {:?}", o);
  }

  fn get_existing_crates_from_storage(&self) -> Vec<CrateKey> {
    let request = ListObjectsRequest {
      bucket: self.s3_bucket_name.clone(),
      version: Some(1),
      prefix: None,
      max_keys: None,
      marker: None,
      delimiter: None,
      encoding_type: None,
    };
    let response = self.s3_client.inner_client.list_objects(&request).unwrap();
    let contents = response.contents;
    contents.into_iter()
      .map(|c| c.key)
      .map(|k| {
        let split = k.split(':').collect::<Vec<_>>();
        CrateKey {
          name: split.get(0).cloned().unwrap().to_owned(),
          version: split.get(1).cloned().unwrap().to_owned(),
        }
      }).collect()
  }
}

impl Default for CrateClonerJob {
  fn default() -> CrateClonerJob {
    CrateClonerJob {
      s3_bucket_name: ::flags::s3_crate_bucket_name::CONFIG.get_value(),
      s3_client: SimpleS3Client::default(),
      upstream_index: UpstreamIndex::load_from_params(UpstreamIndexParams::default()).unwrap()
    }
  }
}

impl Job for CrateClonerJob {
  fn run(mut self) {
    self.run_now()
  }
}

#[cfg(test)]
mod tests {
}
