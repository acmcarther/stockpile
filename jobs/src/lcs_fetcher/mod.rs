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
use common::cargo::CrateKey;
use lcs_fetcher::repository::LcsRepositorySink;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::HttpLcsRepository;
use lcs_fetcher::repository::S3LcsRepository;

mod flags {
  define_pub_cfg!(max_session_crates,
                  u32,
                  1000u32,
                  "The maximum number of crates to download in a single execution of lcs-fetcher.");
}

mod repository;

/** A Job that syncs crate artifacts from upstream Crates.io to a LCS Repository */
#[derive(Builder)]
#[builder(default)]
pub struct LcsFetcherJob {
  upstream_index: UpstreamIndex,
  lcs_sink: Box<LcsRepositorySink>,
  lcs_source: Box<LcsRepositorySource>,
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
    let existing_crate_keys = self.lcs_sink.get_existing_crate_keys()
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
      self.lcs_source.fetch_crate(&key_to_backfill, &crate_tempdir_path);

      let expected_file_path: PathBuf =
        crate_tempdir_path.join(format!("/{}-{}.crate",
                                        key_to_backfill.name,
                                        key_to_backfill.version));
      fs::metadata(&expected_file_path)
        .expect(&format!("upstream crate source failed to download {:?}", expected_file_path));


      self.lcs_sink.upload_crate(key_to_backfill, &expected_file_path);

      // Minor optimization -- remove file early if possible
      fs::remove_file(&expected_file_path);
    }

    Ok(())
  }
}

impl Default for LcsFetcherJob {
  fn default() -> LcsFetcherJob {
    LcsFetcherJob {
      upstream_index: UpstreamIndex::default(),
      lcs_sink: Box::new(S3LcsRepository::default()),
      lcs_source: Box::new(HttpLcsRepository::default()),
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
