use ::Job;
use ::JobErr;
use common::cargo;
use index::crates_io::CratesIoIndex;
use index::augmented::AugmentedIndex;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::LocalFsLcsRepository;
use std::collections::HashSet;
use index::KeyedByCrateKey;

mod flags {
  define_pub_cfg!(max_backfill_changes_per_commit,
                  u32,
                  10000u32,
                  "The maximum number of crate augmented index entries to backfill per commit to the index.");
  define_pub_cfg!(write_changes_to_ais,
                  bool,
                  true,
                  "Whether or not to record backfilled entries to the augmented index.");
  define_pub_cfg!(commit_changes_to_ais,
                  bool,
                  true,
                  "Whether or not to record a commit for backfilled entries. Requires '--write_changes_to_augmented_index'.");
}

#[derive(Clone, Builder)]
#[builder(default)]
pub struct AisBackfillerParams {
  max_changes_per_commit: u32,
  should_write_changes: bool,
  should_commit_changes: bool,
}

impl Default for AisBackfillerParams {
  fn default() -> AisBackfillerParams {
    let should_write_changes =
      flags::write_changes_to_ais::CONFIG.get_value();
    let should_commit_changes =
      flags::commit_changes_to_ais::CONFIG.get_value();

    if should_commit_changes && !should_write_changes {
      panic!("--commit_changes_to_augmented_index requires `--write_changes_to_augmented_index`.")
    }

    AisBackfillerParams {
      should_write_changes: should_write_changes,
      should_commit_changes: should_commit_changes,
      max_changes_per_commit: flags::max_backfill_changes_per_commit::CONFIG.get_value(),
    }
  }
}

struct BackfillEntry {
  pub crate_key: cargo::CrateKey,
  pub variant: BackfillEntryVariant,
}

enum BackfillEntryVariant {
  MissingEntry,
  IncompleteEntry,
}

#[derive(Builder)]
pub struct AisBackfillerJob {
  upstream_index: CratesIoIndex,
  augmented_index: AugmentedIndex,
  lcs_source: Box<LcsRepositorySource>,
  #[builder(default)]
  params: AisBackfillerParams,
}

impl AisBackfillerJob {
  // TODO(acmcarther): USe S3LcsRepository as lcs_source, not temp
  pub fn for_upstream_indexes() -> Result<AisBackfillerJob, JobErr> {
    Ok(AisBackfillerJobBuilder::default()
      .upstream_index(try!(CratesIoIndex::upstream_index()))
      .augmented_index(try!(AugmentedIndex::upstream_index()))
      .lcs_source(Box::new(LocalFsLcsRepository::from_tmp().unwrap()))
      .build()
      .unwrap())
  }

  fn run_now(&mut self) -> Result<(), JobErr> {
    let augmented_index_crate_keys = self.augmented_index.get_crate_keys().into_iter().collect::<HashSet<_>>();
    let lcs_crate_keys = try!(self.lcs_source.get_existing_crate_keys());
    let mut backfill_candidates = Vec::new();

    for lcs_crate_key in lcs_crate_keys.into_iter() {
      if !augmented_index_crate_keys.contains(&lcs_crate_key) {
        backfill_candidates.push(BackfillEntry {
          crate_key: lcs_crate_key,
          variant: BackfillEntryVariant::MissingEntry,
        });
      }
    }

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
  use common::cargo;
  use index::GenericIndexParamsBuilder;
  use index::augmented::AugmentedIndex;
  use index::augmented;
  use index::crates_io::CratesIoIndex;
  use index::crates_io;
  use index;
  use lcs_fetcher::repository::LocalFsLcsRepository;
  use std::str::FromStr;
  use url::Url;

  #[test]
  fn test_trivial_backfiller_doesnt_explode() {
    let source_fs_lcs = LocalFsLcsRepository::from_tmp().unwrap();

    let mut ais_backfiller_job =
      AisBackfillerJobBuilder::default()
        .augmented_index(augmented::testing::get_minimum_index())
        .upstream_index(crates_io::testing::get_minimum_index())
        .lcs_source(Box::new(source_fs_lcs))
        .build()
        .unwrap();

    ais_backfiller_job.run_now().unwrap();
  }
}
