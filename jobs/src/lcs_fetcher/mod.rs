use tempdir::TempDir;
use index::UpstreamIndex;
use super::Job;
use std::fs;
use std::path::PathBuf;
use std::collections::HashSet;
use std::io;
use aws_sdk_rust::aws::errors::s3::S3Error;
use hyper;
use lcs_fetcher::repository::LcsRepositorySink;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::LocalFsLcsRepository;
use lcs_fetcher::repository::HttpLcsRepository;
use lcs_fetcher::repository::S3LcsRepository;

mod flags {
  define_pub_cfg!(max_session_crates,
                  u32,
                  1000u32,
                  "The maximum number of crates to download in a single execution of lcs-fetcher.");
}

mod repository;

/**
 * A Job that syncs crate artifacts from upstream Crates.io to a LCS Repository.
 *
 * This does not derive the builder from Default because the default dependencies (such as S3),
 * must not be eagerly evaluated.
 */
#[derive(Builder)]
pub struct LcsFetcherJob {
  upstream_index: UpstreamIndex,
  lcs_source: Box<LcsRepositorySource>,
  lcs_sink: Box<LcsRepositorySink>,
  #[builder(default)]
  params: LcsFetcherParams,
}

#[derive(Clone, Builder)]
#[builder(default)]
pub struct LcsFetcherParams {
  pub max_session_crates: u32,
}

impl Default for LcsFetcherParams {
  fn default() -> LcsFetcherParams {
    LcsFetcherParams {
      max_session_crates: flags::max_session_crates::CONFIG.get_value(),
    }
  }
}

#[derive(Debug)]
pub enum LcsFetchErr {
  IoErr(io::Error),
  HyperErr(hyper::Error),
  S3Err(S3Error),
}
define_from_error_boilerplate!(io::Error, LcsFetchErr, LcsFetchErr::IoErr);
define_from_error_boilerplate!(hyper::Error, LcsFetchErr, LcsFetchErr::HyperErr);
define_from_error_boilerplate!(S3Error, LcsFetchErr, LcsFetchErr::S3Err);

impl LcsFetcherJob {
  pub fn from_crates_io_to_s3() -> LcsFetcherJob {
    LcsFetcherJobBuilder::default()
      .upstream_index(UpstreamIndex::default())
      .lcs_source(Box::new(HttpLcsRepository::default()))
      .lcs_sink(Box::new(S3LcsRepository::default()))
      .build()
      .unwrap()
  }

  pub fn from_crates_io_to_cwd() -> LcsFetcherJob {
    LcsFetcherJobBuilder::default()
      .upstream_index(UpstreamIndex::default())
      .lcs_source(Box::new(HttpLcsRepository::default()))
      .lcs_sink(Box::new(LocalFsLcsRepository::from_cwd().unwrap()))
      .build()
      .unwrap()
  }

  fn run_now(&mut self) -> Result<(), LcsFetchErr> {
    let existing_crate_keys = self.lcs_sink.get_existing_crate_keys()
      .unwrap()
      .into_iter()
      .collect::<HashSet<_>>();
    let mut crate_keys_in_index = self.upstream_index.get_all_crate_keys();
    crate_keys_in_index.sort_by_key(|k| k.name.to_lowercase());

    let keys_to_backfill = crate_keys_in_index
      .into_iter()
      .filter(|k| !existing_crate_keys.contains(k))
      .take(self.params.max_session_crates as usize)
      .collect::<Vec<_>>();

    info!("Backfilling {} crate keys", keys_to_backfill.len());

    let crate_tempdir = try!(TempDir::new("downloaded_crate_scratch"));
    let crate_tempdir_path = crate_tempdir.path();
    info!("Temp staging path is {:?}",crate_tempdir_path);

    for key_to_backfill in keys_to_backfill.into_iter() {
      info!("Downloading {:?}", key_to_backfill);
      try!(self.lcs_source.fetch_crate(&key_to_backfill, &crate_tempdir_path));
      debug!("Finished download");

      let expected_file_path: PathBuf =
        crate_tempdir_path.join(format!("{}-{}.crate",
                                        key_to_backfill.name,
                                        key_to_backfill.version));
      fs::metadata(&expected_file_path)
        .expect(&format!("upstream crate source failed to download {:?}", expected_file_path));


      info!("Uploading {:?} from {:?} to upstream sink.", key_to_backfill, expected_file_path);
      self.lcs_sink.upload_crate(&key_to_backfill, &expected_file_path).unwrap();

      // Minor optimization -- remove file early if possible
      let _ = fs::remove_file(&expected_file_path);
    }

    Ok(())
  }
}

impl Job for LcsFetcherJob {
  fn run(&mut self) {
    self.run_now().unwrap()
  }
}

#[cfg(test)]
mod tests {
  use common::cargo::CrateKey;
  use index::UpstreamIndex;
  use index::UpstreamIndexParamsBuilder;
  use index;
  use lcs_fetcher::LcsFetcherJobBuilder;
  use lcs_fetcher::LcsFetcherParams;
  use lcs_fetcher::repository::LcsRepositorySink;
  use lcs_fetcher::repository::LcsRepositorySource;
  use lcs_fetcher::repository::LocalFsLcsRepository;
  use lcs_fetcher::repository::testing::TestingCrate;
  use std::fs::File;
  use tempdir::TempDir;
  use url::Url;

  #[test]
  fn test_trivial_fetcher_doesnt_explode() {
    let mut source_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();
    let mut dest_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();
    let upstream_index = {
      let tempdir = index::testing::seed_minimum_index();
      let params = UpstreamIndexParamsBuilder::default()
        .pre_pulled_index_path(Some(tempdir.path().to_path_buf()))
        .build()
        .unwrap();

      UpstreamIndex::load_from_params(params).unwrap()
    };

    let mut lcs_fetcher_job =
      LcsFetcherJobBuilder::default()
        .upstream_index(upstream_index)
        .lcs_source(Box::new(source_fs_lcs))
        .lcs_sink(Box::new(dest_fs_lcs))
        .build()
        .unwrap();

    lcs_fetcher_job.run_now();
  }
}
