#![feature(used)]
#![allow(dead_code)]
extern crate aws_sdk_rust;
extern crate git2;
extern crate hyper;
#[macro_use] extern crate lazy_static;
extern crate tempdir;
extern crate url;
#[macro_use] extern crate zcfg;

define_cfg!(s3_access_key_id,
            ::zcfg::NoneableCfg<String>,
            None,
            "The S3 access key id credential to be used."); 
define_cfg!(s3_secret_access_key,
            ::zcfg::NoneableCfg<String>,
            None,
            "The S3 secret key credential to be used.");
define_cfg!(s3_api_url,
            String,
            "http://minio-small".to_owned(),
            "The location of the local crate service S3 Bucket.");
define_cfg!(s3_crate_bucket_name,
            String,
            "local-crate-service",
            "The name of the S3 bucket where crates are stored.");
define_cfg!(crates_io_index_url,
            String,
            "https://github.com/rust-lang/crates.io-index",
            "The URL for the upstream crates.io index repository");


mod aws {
  use aws_sdk_rust::aws::common::credentials::DefaultCredentialsProvider;
  use aws_sdk_rust::aws::common::credentials::ParametersProvider;
  use aws_sdk_rust::aws::common::region::Region;
  use aws_sdk_rust::aws::s3::endpoint::Endpoint;
  use aws_sdk_rust::aws::s3::endpoint::Signature;
  use aws_sdk_rust::aws::s3::s3client::S3Client;
  use aws_sdk_rust;
  use hyper;
  use std;
  use url::Url;

  type AbsurdAwsType =
    aws_sdk_rust::aws::s3::s3client::S3Client<
      aws_sdk_rust::aws::common::credentials::BaseAutoRefreshingProvider<
        aws_sdk_rust::aws::common::credentials::ChainProvider,
        std::cell::RefCell<aws_sdk_rust::aws::common::credentials::AwsCredentials>>,
      hyper::Client>;


  pub struct SimpleS3Client {
    inner_client: AbsurdAwsType
  }

  impl Default for SimpleS3Client {
    fn default() -> SimpleS3Client {
      let s3_url = Url::parse(&::s3_api_url::CONFIG.get_value()).unwrap();
      let provider = DefaultCredentialsProvider::new(Some(get_default_s3_params())).unwrap();
      let endpoint = Endpoint::new(
        Region::UsEast1 /* irrelevant for internal */,
        Signature::V4,
        Some(s3_url),
        None /* proxy */,
        None /* user_agent */,
        None /* is_bucket_virtual */);
      let inner_client = S3Client::new(provider, endpoint);
      SimpleS3Client {
        inner_client: inner_client
      }
    }
  }

  fn get_default_s3_params() -> ParametersProvider {
    let access_key_id =
      ::s3_access_key_id::CONFIG.get_value().inner()
        .expect("--s3_access_key_id must be set");
    let secret_access_key =
      ::s3_secret_access_key::CONFIG.get_value().inner()
        .expect("--s3_secret_access_key must be set");
    ParametersProvider::with_parameters(
        access_key_id,
        secret_access_key,
        None).unwrap()
  }
}

mod index {
  use git2;
  use tempdir::TempDir;

  pub struct UpstreamIndex {
  }
}

use aws::SimpleS3Client;
use tempdir::TempDir;

trait Job {
  fn run(self);
}

struct CrateClonerJob {
  s3_client: SimpleS3Client,
}

impl Default for CrateClonerJob {
  fn default() -> CrateClonerJob {
    CrateClonerJob {
      s3_client: SimpleS3Client::default()
    }
  }
}

impl Job for CrateClonerJob {
  fn run(self) {
    // Clone crates.io_index

  }
}

#[cfg(test)]
mod tests {
}
