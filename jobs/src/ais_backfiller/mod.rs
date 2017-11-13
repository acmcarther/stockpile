use ::Job;
use ::JobErr;
use toml;
use cargo::util::toml::TomlManifest;
use common::cargo;
use flate2::read::GzDecoder;
use std::path::PathBuf;
use index::KeyedByCrateKey;
use index::augmented::AugmentedIndex;
use index::crates_io::CratesIoIndex;
use lcs_fetcher::repository::LcsRepositorySource;
use lcs_fetcher::repository::LocalFsLcsRepository;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use tar::Archive;
use tempdir::TempDir;

mod flags {
  define_pub_cfg!(max_backfill_changes_per_commit,
                  i32,
                  10000i32,
                  "The maximum number of crate augmented index entries to backfill per commit to the index. Set to -1 for no limit.");
  define_pub_cfg!(write_changes_to_ais,
                  bool,
                  true,
                  "Whether or not to record backfilled entries to the augmented index.");
  define_pub_cfg!(commit_changes_to_ais,
                  bool,
                  true,
                  "Whether or not to record a commit for backfilled entries. Requires '--write_changes_to_augmented_index'.");
  define_pub_cfg!(force_backfill_all_keys,
                  bool,
                  false,
                  "Whether or not to backfill all keys, regardless of backfill status.");
}

#[derive(Clone, Builder)]
#[builder(default)]
pub struct AisBackfillerParams {
  max_changes_per_commit: i32,
  force_backfill_all_keys: bool,
  should_write_changes: bool,
  should_commit_changes: bool,
}

impl Default for AisBackfillerParams {
  fn default() -> AisBackfillerParams {
    let should_write_changes =
      flags::write_changes_to_ais::CONFIG.get_value();
    let should_commit_changes =
      flags::commit_changes_to_ais::CONFIG.get_value();
    let force_backfill_all_keys =
      flags::force_backfill_all_keys::CONFIG.get_value();
    let max_changes_per_commit =
      flags::max_backfill_changes_per_commit::CONFIG.get_value();

    if should_commit_changes && !should_write_changes {
      panic!("--commit_changes_to_augmented_index requires `--write_changes_to_augmented_index`.");
    }

    if force_backfill_all_keys && max_changes_per_commit != -1 {
      panic!("--should_backfill_all requires `--max_changes_per_commit to be -1 (indicating no limit)`.");
    }

    AisBackfillerParams {
      should_write_changes: should_write_changes,
      should_commit_changes: should_commit_changes,
      force_backfill_all_keys: force_backfill_all_keys,
      max_changes_per_commit: max_changes_per_commit,
    }
  }
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

    if self.params.force_backfill_all_keys {
      backfill_candidates = lcs_crate_keys;
    } else {
      for lcs_crate_key in lcs_crate_keys.into_iter() {
        if !augmented_index_crate_keys.contains(&lcs_crate_key) {
          backfill_candidates.push(lcs_crate_key);
        }
      }

      for augmented_index_key in augmented_index_crate_keys.into_iter() {
        let item = self.augmented_index.get_entry(&augmented_index_key).unwrap();
        if item.dev_dependencies.is_none() {
          backfill_candidates.push(augmented_index_key.clone());
        }
      }
    }
    backfill_candidates.sort_by_key(|k| k.name.to_lowercase());
    let keys_to_backfill = backfill_candidates.into_iter()
      .take(self.params.max_changes_per_commit as usize)
      .collect::<Vec<_>>();

    let tempdir = try!(TempDir::new("local_crates_during_backfill"));
    let tempdir_path = tempdir.path();
    for key_to_backfill in keys_to_backfill.into_iter() {
      let upstream_entry = self.upstream_index.get_entry(&key_to_backfill);
      let augmented_entry = self.augmented_index.get_entry(&key_to_backfill);
      let crate_filename = format!("{name}-{version}.crate",
                                   name = key_to_backfill.name,
                                   version = key_to_backfill.version);
      try!(self.lcs_source.fetch_crate(&key_to_backfill, &tempdir_path));
      let crate_path = tempdir_path.join(crate_filename);

      let file = try!(File::open(crate_path));
      let gz = try!(GzDecoder::new(file));
      let mut tar = Archive::new(gz);
      let mut found_file = false;
      let mut toml_contents = String::new();
      for entry_res in try!(tar.entries()) {
        let mut entry = try!(entry_res);
        {
          let file_path = try!(entry.path());
          // TODO(acmcarther): This is likely inefficient
          if !(file_path == PathBuf::from("Cargo.toml")) {
            continue
          }
        }

        found_file = true;
        entry.read_to_string(&mut toml_contents);
        break;
      }

      if !found_file {
        return Err(JobErr::OtherErr(format!("{}:{} did not have a valid Cargo.toml",
                                            key_to_backfill.name,
                                            key_to_backfill.version)))
      }

      if toml_contents.is_empty() {
        return Err(JobErr::OtherErr(format!("{}:{} has a Cargo.toml but it is empty",
                                            key_to_backfill.name,
                                            key_to_backfill.version)))
      }

      let manifest = try!(toml::from_str::<TomlManifest>(&toml_contents));

      // TODO(acmcarther): Extract useful information from the manifest
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
  use ais_backfiller::AisBackfillerParamsBuilder;
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
