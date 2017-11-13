use ::JobErr;
use common::cargo::CrateKey;
use common::cargo;
use index::GenericIndexLoader;
use index::GenericIndexParams;
use index::GenericIndexArtifacts;
use index::KeyedByCrateKey;
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

mod flags {
  define_pub_cfg!(augmented_index_url,
                  String,
                  "https://github.com/acmcarther/stockpile-index",
                  "The URL for the upstream stockpile augmented crate index.");
  define_pub_cfg!(pre_pulled_augmented_index_directory,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The path to the augmented index to use in lieu of pulling a fresh copy.");
}

/** The parameters required to load and use an augmented crate metadata index. */
#[derive(Builder, Clone)]
pub struct AugmentedIndexParams {
  generic_params: GenericIndexParams,
}

impl AugmentedIndexParams {
  /** Provideds a default set of parameters based on binary flags. */
  pub fn upstream_index() -> AugmentedIndexParams {
    let url = Url::parse(&flags::augmented_index_url::CONFIG.get_value()).unwrap();
    let pre_pulled_index_path = flags::pre_pulled_augmented_index_directory::CONFIG.get_value().inner()
      .map(PathBuf::from);
    AugmentedIndexParams {
      generic_params: GenericIndexParams {
        url: url,
        pre_pulled_index_path: pre_pulled_index_path,
      },
    }
  }
}

/** A loaded, ready-to-use augmented crate metadata index. */
#[derive(Clone)]
pub struct AugmentedIndex {
  params: AugmentedIndexParams,
  loader_artifacts: GenericIndexArtifacts,
  contents: HashMap<cargo::CrateKey, cargo::AugmentedIndexEntry>,
}

impl AugmentedIndex {
  pub fn upstream_index() -> Result<AugmentedIndex, JobErr> {
    AugmentedIndex::new(AugmentedIndexParams::upstream_index())
  }

  /** Produces a ready-to-use AugmentedIndex using the provided params. */
  pub fn new(params: AugmentedIndexParams) -> Result<AugmentedIndex, JobErr> {
    let loader = GenericIndexLoader::new(params.generic_params.clone());
    let (artifacts, contents) = try!(loader.load_index::<cargo::AugmentedIndexEntry>());
    let keyed_contents = contents.into_iter()
      .map(|content| (cargo::CrateKey::from(content.clone()), content))
      .collect::<HashMap<_, _>>();

    Ok(AugmentedIndex {
      params: params,
      loader_artifacts: artifacts,
      contents: keyed_contents
    })
  }
}

impl KeyedByCrateKey for AugmentedIndex {
  type Item = cargo::AugmentedIndexEntry;

  fn get_crate_keys(&self) -> Vec<&CrateKey> {
    self.contents.keys().collect()
  }

  fn get_entry(&self, key: &CrateKey) -> Option<&cargo::AugmentedIndexEntry> {
    self.contents.get(key)
  }
}

pub mod testing {
  use common::cargo;
  use index;
  use index::GenericIndexParams;
  use index::augmented::AugmentedIndex;
  use index::augmented::AugmentedIndexParams;
  use url::Url;

  pub fn get_seeded_index(entries: Vec<cargo::AugmentedIndexEntry>) -> AugmentedIndex {
    let tempdir = index::testing::seed_index_with_contents(entries);
    let params = AugmentedIndexParams {
      generic_params: GenericIndexParams {
        url: Url::parse("http://not-resolvable").unwrap(),
        pre_pulled_index_path: Some(tempdir.path().to_path_buf()),
      }
    };

    AugmentedIndex::new(params).unwrap()
  }

  pub fn get_minimum_index() -> AugmentedIndex {
    get_seeded_index(Vec::new())
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use common::cargo;
  use url::Url;
  use index::augmented;
  use index::KeyedByCrateKey;

  #[test]
  fn test_empty_local_index_works() {
    let index = augmented::testing::get_minimum_index();
    assert_eq!(index.get_crate_keys(), Vec::<&cargo::CrateKey>::new());
  }

  #[test]
  fn test_loads_trivial_index() {
    let index_entry = cargo::AugmentedIndexEntry {
      name: "a".to_owned(),
      vers: "0.0.1".to_owned(),
      dev_dependencies: Some(Vec::new()),
    };
    let index = augmented::testing::get_seeded_index(vec![index_entry]);

    assert_eq!(index.get_crate_keys(),
               vec![&cargo::CrateKey {
                 name: "a".to_owned(),
                 version: "0.0.1".to_owned()
               }]);
  }
}
