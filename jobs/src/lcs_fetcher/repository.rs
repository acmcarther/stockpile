use aws_sdk_rust::aws::s3::object::ListObjectsRequest;
use aws_sdk_rust::aws::s3::object::ListObjectsOutput;
use aws::SimpleS3Client;
use tempdir::TempDir;
use index::UpstreamIndex;
use index::UpstreamIndexParams;
use super::Job;
use std::path::Path;
use std::path::PathBuf;

use lcs_fetcher::common::CrateKey;

mod flags {
  define_pub_cfg!(s3_crate_bucket_name,
                  String,
                  "local-crate-service",
                  "The name of the S3 bucket where crates are stored.");
}

pub trait LcsRepository: _LcsRepositoryClone {
  fn get_existing_crate_keys(&self) -> Vec<CrateKey>;
  fn upload_crate(&mut self, key: CrateKey, path: &Path);
}

trait _LcsRepositoryClone {
  fn clone_box(&self) -> Box<LcsRepository>;
}

impl<T> _LcsRepositoryClone for T where T: 'static + LcsRepository + Clone {
  fn clone_box(&self) -> Box<LcsRepository> {
    Box::new(self.clone())
  }
}

impl Clone for Box<LcsRepository> {
  fn clone(&self) -> Box<LcsRepository> {
    self.clone_box()
  }
}

#[derive(Clone)]
pub struct LocalDirectoryLcsRepository {
  directory: PathBuf
}

impl LcsRepository for LocalDirectoryLcsRepository {
  fn get_existing_crate_keys(&self) -> Vec<CrateKey> {
    // TODO: Stub
    Vec::new()
  }
  fn upload_crate(&mut self, key: CrateKey, path: &Path) {
    // TODO: Stub
    ()
  }
}

#[derive(Clone)]
pub struct S3LcsRepository {
  s3_bucket_name: String,
  s3_client: SimpleS3Client,
}

impl Default for S3LcsRepository {
  fn default() -> S3LcsRepository {
    S3LcsRepository {
      s3_bucket_name: flags::s3_crate_bucket_name::CONFIG.get_value(),
      s3_client: SimpleS3Client::default(),
    }
  }
}

impl LcsRepository for S3LcsRepository {
  fn get_existing_crate_keys(&self) -> Vec<CrateKey> {
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
  fn upload_crate(&mut self, key: CrateKey, path: &Path) {
    // TODO: Stub
    ()
  }
}

