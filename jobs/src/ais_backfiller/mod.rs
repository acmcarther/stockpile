use common::cargo;
use git2;
use index::GenericIndex;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::LocalFsLcsRepository;
use lcs_fetcher::repository::S3LcsRepository;
use std::io;
use super::Job;

#[derive(Builder)]
pub struct AisBackfillerJob {
  augmented_index: GenericIndex<cargo::AugmentedIndexEntry>,
  lcs_source: Box<LcsRepositorySource>,
  #[builder(default)]
  params: AisBackfillerParams,
}

impl Default for AisBackfillerJob {
  // TODO(acmcarther): USe S3LcsRepository as lcs_source, not temp
  fn default() -> AisBackfillerJob {
    AisBackfillerJobBuilder::default()
      .augmented_index(GenericIndex::augmented_index())
      .lcs_source(Box::new(LocalFsLcsRepository::from_tmp().unwrap()))
      .build()
      .unwrap()
  }
}

#[derive(Clone, Builder, Default)]
#[builder(default)]
pub struct AisBackfillerParams {
}

#[derive(Debug)]
pub enum AisBackfillErr {
  GitErr(git2::Error),
  IoErr(io::Error),
}
define_from_error_boilerplate!(io::Error, AisBackfillErr, AisBackfillErr::IoErr);
define_from_error_boilerplate!(git2::Error, AisBackfillErr, AisBackfillErr::GitErr);

impl AisBackfillerJob {
  fn run_now(&mut self) -> Result<(), AisBackfillErr> {
    Ok(())
  }
}

impl Job for AisBackfillerJob {
  fn run(&mut self) {
    self.run_now().unwrap()
  }
}


pub mod testing {
}


#[cfg(test)]
mod tests {
  use index;
  use url::Url;
  use std::str::FromStr;
  use common::cargo;
  use index::GenericIndex;
  use index::GenericIndexParamsBuilder;
  use ais_backfiller::AisBackfillerJobBuilder;
  use lcs_fetcher::repository::LocalFsLcsRepository;

  #[test]
  fn test_trivial_backfiller_doesnt_explode() {
    let source_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();

    let augmented_index: GenericIndex<cargo::AugmentedIndexEntry> = {
      let tempdir = index::testing::seed_minimum_index();
      let params = GenericIndexParamsBuilder::default()
        .url(Url::from_str("http://invalid-url").unwrap())
        .pre_pulled_index_path(Some(tempdir.path().to_path_buf()))
        .build()
        .unwrap();

      GenericIndex::load_from_params(params).unwrap()
    };

    let mut ais_backfiller_job =
      AisBackfillerJobBuilder::default()
        .augmented_index(augmented_index)
        .lcs_source(Box::new(source_fs_lcs))
        .build()
        .unwrap();

    ais_backfiller_job.run_now().unwrap();
  }
}
