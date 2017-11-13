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
  define_pub_cfg!(crates_io_index_url,
                  String,
                  "https://github.com/rust-lang/crates.io-index",
                  "The URL for the upstream crates.io index repository");

  define_pub_cfg!(pre_pulled_crates_io_index_directory,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The path to the crates.io index to use in lieu of pulling a fresh copy.");
}

/** The parameters required to load and use a crates.io metadata index. */
#[derive(Builder, Clone)]
pub struct CratesIoIndexParams {
  generic_params: GenericIndexParams,
}

impl CratesIoIndexParams {
  /** Provides a default set of parameters based on binary flags. */
  pub fn upstream_index() -> CratesIoIndexParams {
    let url = Url::parse(&flags::crates_io_index_url::CONFIG.get_value()).unwrap();
    let pre_pulled_index_path = flags::pre_pulled_crates_io_index_directory::CONFIG.get_value().inner()
      .map(PathBuf::from);
    CratesIoIndexParams {
      generic_params: GenericIndexParams {
        url: url,
        pre_pulled_index_path: pre_pulled_index_path,
      },
    }
  }
}

/** A loaded, ready-to-use crates.io metadata index. */
#[derive(Clone)]
pub struct CratesIoIndex {
  params: CratesIoIndexParams,
  loader_artifacts: GenericIndexArtifacts,
  contents: HashMap<cargo::CrateKey, cargo::IndexEntry>,
}

impl CratesIoIndex {
  pub fn upstream_index() -> Result<CratesIoIndex, JobErr> {
    CratesIoIndex::new(CratesIoIndexParams::upstream_index())
  }

  /** Produces a ready-to-use CratesIoIndex using the provided params. */
  pub fn new(params: CratesIoIndexParams) -> Result<CratesIoIndex, JobErr> {
    let loader = GenericIndexLoader::new(params.generic_params.clone());
    let (artifacts, contents) = try!(loader.load_index::<cargo::IndexEntry>());
    let keyed_contents = contents.into_iter()
      .map(|content| (cargo::CrateKey::from(content.clone()), content))
      .collect::<HashMap<_, _>>();

    Ok(CratesIoIndex {
      params: params,
      loader_artifacts: artifacts,
      contents: keyed_contents
    })
  }
}

impl KeyedByCrateKey for CratesIoIndex {
  type Item = cargo::IndexEntry;

  fn get_crate_keys(&self) -> Vec<&CrateKey> {
    self.contents.keys().collect()
  }

  fn get_entry(&self, key: &CrateKey) -> Option<&cargo::IndexEntry> {
    self.contents.get(key)
  }
}

pub mod testing {
  use common::cargo;
  use index;
  use index::GenericIndexParams;
  use index::crates_io::CratesIoIndex;
  use index::crates_io::CratesIoIndexParams;
  use url::Url;

  pub fn get_seeded_index(entries: Vec<cargo::IndexEntry>) -> CratesIoIndex {
    let tempdir = index::testing::seed_index_with_contents(entries);
    let params = CratesIoIndexParams {
      generic_params: GenericIndexParams {
        url: Url::parse("http://not-resolvable").unwrap(),
        pre_pulled_index_path: Some(tempdir.path().to_path_buf()),
      }
    };

    CratesIoIndex::new(params).unwrap()
  }

  pub fn get_minimum_index() -> CratesIoIndex {
    get_seeded_index(Vec::new())
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use common::cargo;
  use url::Url;
  use index::crates_io;
  use index::KeyedByCrateKey;

  #[test]
  fn test_empty_local_index_works() {
    let index = crates_io::testing::get_minimum_index();
    assert_eq!(index.get_crate_keys(), Vec::<&cargo::CrateKey>::new());
  }

  #[test]
  fn test_loads_trivial_index() {
    let index_entry = cargo::IndexEntry {
      name: "a".to_owned(),
      vers: "0.0.1".to_owned(),
      deps: Vec::new(),
      cksum: "111".to_owned(),
      features: HashMap::new(),
      yanked: None,
    };
    let index = crates_io::testing::get_seeded_index(vec![index_entry]);

    assert_eq!(index.get_crate_keys(),
               vec![&cargo::CrateKey {
                 name: "a".to_owned(),
                 version: "0.0.1".to_owned()
               }]);
  }
}
