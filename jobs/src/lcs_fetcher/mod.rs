use ::Job;
use ::JobErr;
use common::cargo;
use index::KeyedByCrateKey;
use index::crates_io::CratesIoIndex;
use lcs_fetcher::repository::HttpLcsRepository;
use lcs_fetcher::repository::LcsRepositorySink;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::LocalFsLcsRepository;
use lcs_fetcher::repository::S3LcsRepository;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempdir::TempDir;

mod flags {
  define_pub_cfg!(max_session_crates,
                  u32,
                  1000u32,
                  "The maximum number of crates to download in a single execution of lcs-fetcher.");
}

pub mod repository;

/**
 * A Job that syncs crate artifacts from upstream Crates.io to a LCS Repository.
 *
 * This does not derive the builder from Default because the default dependencies (such as S3),
 * must not be eagerly evaluated.
 */
#[derive(Builder)]
pub struct LcsFetcherJob {
  upstream_index: CratesIoIndex,
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

impl LcsFetcherJob {
  pub fn from_crates_io_to_s3() -> Result<LcsFetcherJob, JobErr> {
    Ok(LcsFetcherJobBuilder::default()
      .upstream_index(try!(CratesIoIndex::upstream_index()))
      .lcs_source(Box::new(HttpLcsRepository::default()))
      .lcs_sink(Box::new(S3LcsRepository::default()))
      .build()
      .unwrap())
  }

  pub fn from_crates_io_to_cwd() -> Result<LcsFetcherJob, JobErr> {
    Ok(LcsFetcherJobBuilder::default()
      .upstream_index(try!(CratesIoIndex::upstream_index()))
      .lcs_source(Box::new(HttpLcsRepository::default()))
      .lcs_sink(Box::new(LocalFsLcsRepository::from_cwd().unwrap()))
      .build()
      .unwrap())
  }

  fn run_now(&mut self) -> Result<(), JobErr> {
    let existing_crate_keys = self.lcs_sink.get_existing_crate_keys()
      .unwrap()
      .into_iter()
      .collect::<HashSet<_>>();
    let mut crate_keys_in_index: Vec<cargo::CrateKey> = self.upstream_index.get_crate_keys()
      .iter()
      .map(|c| (*c).clone())
      .collect();
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
  use std::collections::HashMap;
  use common::cargo;
  use index::crates_io;
  use tempdir::TempDir;
  use lcs_fetcher::LcsFetcherJobBuilder;
  use lcs_fetcher::repository::LcsBase;
  use lcs_fetcher::repository::LocalFsLcsRepository;
  use lcs_fetcher::repository::testing::TestingCrate;
  use lcs_fetcher::repository;

  #[test]
  fn test_trivial_fetcher_doesnt_explode() {
    let source_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();
    let dest_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();

    let mut lcs_fetcher_job =
      LcsFetcherJobBuilder::default()
        .upstream_index(crates_io::testing::get_minimum_index())
        .lcs_source(Box::new(source_fs_lcs))
        .lcs_sink(Box::new(dest_fs_lcs))
        .build()
        .unwrap();

    lcs_fetcher_job.run_now().unwrap();
  }

  #[test]
  fn test_fetcher_copies_crates_from_source_into_dest() {
    let test_crates = vec![
      TestingCrate {
        key: cargo::CrateKey {
          name: "test".to_owned(),
          version: "0.0.0".to_owned(),
        },
        contents: b"hello crate".to_vec(),
      }
    ];
    let source_fs_lcs = repository::testing::create_localfs_for_testing(&test_crates).unwrap();
    let dest_temp_dir = TempDir::new("test_destination").unwrap();
    let dest_fs_lcs = LocalFsLcsRepository::new(dest_temp_dir.path());
    let index = crates_io::testing::get_seeded_index(vec![
      cargo::IndexEntry {
        name: "test".to_owned(),
        vers: "0.0.0".to_owned(),
        deps: Vec::new(),
        cksum: "111".to_owned(),
        features: HashMap::new(),
        yanked: None,
      }
    ]);

    let mut lcs_fetcher_job =
      LcsFetcherJobBuilder::default()
        .upstream_index(index)
        .lcs_source(Box::new(source_fs_lcs))
        .lcs_sink(Box::new(dest_fs_lcs))
        .build()
        .unwrap();

    lcs_fetcher_job.run_now().unwrap();

    let dest_fs_lcs = LocalFsLcsRepository::new(dest_temp_dir.path());
    assert_eq!(dest_fs_lcs.get_existing_crate_keys().unwrap(), vec![
      cargo::CrateKey {
        name: "test".to_owned(),
        version: "0.0.0".to_owned()
      }
    ])
  }
}
