use super::Job;
use git2;
use std::io;

#[derive(Builder)]
pub struct AisBackfillerJob {
  upstream_index: UpstreamIndex,
  augmented_index: AugmentedIndex,
  lcs_source: Box<LcsRepositorySource>,
  #[builder(default)]
  params: AisBackfillerParams,
}

#[derive(Clone, Builder)]
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
  use ais_backfiller::AisBackfillerJobBuilder;

  #[test]
  fn test_trivial_backfiller_doesnt_explode() {
    let source_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();
    let upstream_index = {
      let tempdir = index::testing::seed_minimum_index();
      let params = UpstreamIndexParamsBuilder::default()
        .pre_pulled_index_path(Some(tempdir.path().to_path_buf()))
        .build()
        .unwrap();

      UpstreamIndex::load_from_params(params).unwrap()
    };

    let augmented_index = {
      let tempdir = index::testing::seed_minimum_index();
      let params = AugmentedIndexParamsBuilder::default()
        .pre_pulled_index_path(Some(tempdir.path().to_path_buf()))
        .build()
        .unwrap();

      AugmentedIndex::load_from_params(params).unwrap()
    }

    let mut ais_backfiller_job =
      AisBackfillerJobBuilder::default()
        .upstream_index(upstream_index)
        .augmented_index(augmented_index)
        .lcs_source(Box::new(source_fs_lcs))
        .build()
        .unwrap();

    ais_backfiller_job.run_now().unwrap();
  }
}
