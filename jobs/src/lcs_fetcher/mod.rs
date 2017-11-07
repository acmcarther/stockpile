use aws_sdk_rust::aws::s3::object::ListObjectsRequest;
use aws_sdk_rust::aws::s3::object::ListObjectsOutput;
use aws::SimpleS3Client;
use tempdir::TempDir;
use index::UpstreamIndex;
use index::UpstreamIndexParams;
use super::Job;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashSet;
use std::io;

mod flags {
  define_pub_cfg!(max_session_crates,
                  u32,
                  1000u32,
                  "The maximum number of crates to download in a single execution of crate cloner.");
}

// TODO(acmcarther): Fix visibility
// This is a hack to let "index' see it.
pub mod common;
mod repository;
mod crate_source;

use lcs_fetcher::common::CrateKey;
use lcs_fetcher::repository::LcsRepository;
use lcs_fetcher::repository::S3LcsRepository;
use lcs_fetcher::crate_source::UpstreamCrateSource;
use lcs_fetcher::crate_source::S3UpstreamCrateSource;

/** A Job that syncs crate artifacts from upstream Crates.io to a LCS Repository */
#[derive(Builder)]
#[builder(default)]
pub struct LcsFetcherJob {
  lcs_repository: Box<LcsRepository>,
  upstream_index: UpstreamIndex,
  upstream_crate_source: Box<UpstreamCrateSource>,
  params: LcsFetcherParams,
}

#[derive(Clone, Builder)]
#[builder(default)]
pub struct LcsFetcherParams {
  pub max_session_crates: u32,
}

#[derive(Debug)]
pub enum LcsFetchErr {
  IoErr(io::Error),
}

impl From<io::Error> for LcsFetchErr {
  fn from(error: io::Error) -> LcsFetchErr {
    LcsFetchErr::IoErr(error)
  }
}

impl LcsFetcherJob {
  fn run_now(&mut self) -> Result<(), LcsFetchErr> {
    let existing_crate_keys = self.lcs_repository.get_existing_crate_keys()
      .into_iter()
      .collect::<HashSet<_>>();
    let crate_keys_in_index = self.upstream_index.get_all_crate_keys();

    let keys_to_backfill = crate_keys_in_index
      .into_iter()
      .filter(|k| !existing_crate_keys.contains(k))
      .take(self.params.max_session_crates as usize)
      .collect::<Vec<_>>();

    info!("Backfilling {} crate keys", keys_to_backfill.len());

    let crate_tempdir = try!(TempDir::new("downloaded_crate_scratch"));
    let crate_tempdir_path = crate_tempdir.path();

    for key_to_backfill in keys_to_backfill.into_iter() {
      self.upstream_crate_source.fetch_crate(&key_to_backfill, &crate_tempdir_path);

      let expected_file_path: PathBuf =
        crate_tempdir_path.join(format!("/{}-{}.crate",
                                        key_to_backfill.name,
                                        key_to_backfill.version));
      fs::metadata(&expected_file_path)
        .expect(&format!("upstream crate source failed to download {:?}", expected_file_path));


      self.lcs_repository.upload_crate(key_to_backfill, &expected_file_path);

      // Minor optimization -- remove file early if possible
      fs::remove_file(&expected_file_path);
    }

    Ok(())
  }
}

impl Default for LcsFetcherJob {
  fn default() -> LcsFetcherJob {
    LcsFetcherJob {
      lcs_repository: Box::new(S3LcsRepository::default()),
      upstream_index: UpstreamIndex::load_from_params(UpstreamIndexParams::default()).unwrap(),
      upstream_crate_source: Box::new(S3UpstreamCrateSource::default()),
      params: LcsFetcherParams::default(),
    }
  }
}

impl Default for LcsFetcherParams {
  fn default() -> LcsFetcherParams {
    LcsFetcherParams {
      max_session_crates: flags::max_session_crates::CONFIG.get_value(),
    }
  }
}
impl Job for LcsFetcherJob {
  fn run(&mut self) {
    self.run_now().unwrap()
  }
}

#[cfg(test)]
mod tests {
}
