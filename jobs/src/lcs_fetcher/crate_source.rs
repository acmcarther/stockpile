use std::path::Path;
use lcs_fetcher::common::CrateKey;

mod flags {
  define_pub_cfg!(upstream_s3_prefix,
                  String,
                  "https://crates-io.s3-us-west-1.amazonaws.com/crates/",
                  "The name of the S3 bucket where crates are stored.");
}

pub trait UpstreamCrateSource: _UpstreamCrateSourceClone {
  fn fetch_crate(&self, key: &CrateKey, destination: &Path);
}

trait _UpstreamCrateSourceClone : Send {
  fn clone_box(&self) -> Box<UpstreamCrateSource>;
}

impl<T> _UpstreamCrateSourceClone for T where T: 'static + UpstreamCrateSource + Clone {
  fn clone_box(&self) -> Box<UpstreamCrateSource> {
    Box::new(self.clone())
  }
}

impl Clone for Box<UpstreamCrateSource> {
  fn clone(&self) -> Box<UpstreamCrateSource> {
    self.clone_box()
  }
}

#[derive(Clone)]
pub struct S3UpstreamCrateSource {
  upstream_s3_prefix: String,
}

impl Default for S3UpstreamCrateSource {
  fn default() -> S3UpstreamCrateSource {
    S3UpstreamCrateSource {
      upstream_s3_prefix: flags::upstream_s3_prefix::CONFIG.get_value(),
    }
  }
}

impl UpstreamCrateSource for S3UpstreamCrateSource {
  fn fetch_crate(&self, key: &CrateKey, destination: &Path) {
    // TODO(acmcarther): Stub
  }
}
