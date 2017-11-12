use common::cargo::CrateKey;
use common::cargo;
use common::iter_util;
use git2::Repository;
use git2;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::fs;
use std::io::Read;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tempdir::TempDir;
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

/** Defines the params needed to build an GenericIndex. */
#[derive(Builder)]
pub struct GenericIndexParams {
  url: Url,
  pre_pulled_index_path: Option<PathBuf>,
}

impl GenericIndexParams {
  /** Constructs an GenericIndexParams from flags. */
  fn crates_io_index_params() -> GenericIndexParams {
    let url = Url::parse(&flags::crates_io_index_url::CONFIG.get_value()).unwrap();
    let pre_pulled_index_path = flags::pre_pulled_crates_io_index_directory::CONFIG.get_value().inner()
      .map(PathBuf::from);

    GenericIndexParams {
      url: url,
      pre_pulled_index_path: pre_pulled_index_path,
    }
  }
}

/** A structured index object of the same form as the Crates.io index.*/
#[derive(Clone)]
pub struct GenericIndex<T: Serialize + DeserializeOwned + Send> {
  repository: Arc<Repository>,
  in_memory_index: HashMap<String, Vec<T>>,
  tempdir: Arc<Option<TempDir>>
}

/** The set of possible errors that may occur while using an GenericIndex. */
#[derive(Debug)]
pub enum GenericIndexErr {
  InvalidCacheState(String),
  GitErr(git2::Error),
  IoErr(io::Error),
  SerdeJsonErr(serde_json::Error),
}
define_from_error_boilerplate!(String, GenericIndexErr, GenericIndexErr::InvalidCacheState);
define_from_error_boilerplate!(git2::Error, GenericIndexErr, GenericIndexErr::GitErr);
define_from_error_boilerplate!(io::Error, GenericIndexErr, GenericIndexErr::IoErr);
define_from_error_boilerplate!(serde_json::Error, GenericIndexErr, GenericIndexErr::SerdeJsonErr);

impl GenericIndex<cargo::IndexEntry> {
  /** Builds a Crates.io GenericIndex from flags. */
  pub fn crates_io_index() -> GenericIndex<cargo::IndexEntry> {
    GenericIndex::load_from_params(GenericIndexParams::crates_io_index_params()).unwrap()
  }

  /** Retrieves all known CrateKey objects from the index. */
  pub fn get_all_crate_keys(&self) -> Vec<CrateKey> {
    self.in_memory_index.values()
      .flat_map(|v| v.iter())
      .map(|index_entry| CrateKey {
        name: index_entry.name.clone(),
        version: index_entry.vers.clone()
      })
      .collect()
  }
}

impl <T: Serialize + DeserializeOwned + Send> GenericIndex<T> {
  /**
   * Constructs an GenericIndex from the provided arguments.
   *
   * If a pre_pulled_index_path is provided, it is loaded directly. Otherwise, the index is pulled
   * into a temporary directory and loaded.
   */
  pub fn load_from_params(params: GenericIndexParams) -> Result<GenericIndex<T>, GenericIndexErr> {
    if params.pre_pulled_index_path.is_some() {
      let path = PathBuf::from(params.pre_pulled_index_path.unwrap());
      debug!("Loading Index from {:?}", path);
      let repo = try!(Repository::open(&path));
      let in_memory_index = try!(GenericIndex::load_in_memory_index(path));
      Ok(GenericIndex {
        repository: Arc::new(repo),
        in_memory_index: in_memory_index.into_iter().collect(),
        tempdir: Arc::new(None),
      })
    } else {
      debug!("Creating temp dir for upstream crates io index");
      let dir = try!(TempDir::new("upstream_in_memory_index"));
      debug!("Cloning upstream crates io index from {}, into {:?}", params.url, dir.path());
      let repo = try!(Repository::clone(&params.url.to_string(), &dir));
      let in_memory_index = try!(GenericIndex::load_in_memory_index(dir.path()));
      Ok(GenericIndex {
        repository: Arc::new(repo),
        tempdir: Arc::new(Some(dir)),
        in_memory_index: in_memory_index.into_iter().collect(),
      })
    }
  }

  /** Loads the Crates.io Index into memory, ready for use. */
  fn load_in_memory_index<P: AsRef<Path>>(in_memory_index_dir: P) -> Result<Vec<(String, Vec<T>)>, GenericIndexErr> {
    debug!("Loading crates.io-index from {:?}", in_memory_index_dir.as_ref());
    let mut dir_iters = Vec::new();
    let mut leaves = Vec::new();
    dir_iters.push(try!(fs::read_dir(&in_memory_index_dir)));
    while !dir_iters.is_empty() {
      let dir_iter = dir_iters.pop().unwrap();
      for entry_res in dir_iter {
        // Skip unreadable files
        if entry_res.is_err() {
          continue
        }
        let entry = entry_res.unwrap();
        let file_type = entry.file_type().unwrap();
        let path = entry.path();

        if path.ends_with("config.json") || path.ends_with(".git") {
          continue
        }

        if file_type.is_dir() {
          dir_iters.push(try!(fs::read_dir(path)));
        } else if file_type.is_file() {
          leaves.push(path)
        }
      }
    }

    // TODO(acmcarther): Revisit this implementation. Its fast, but really ugly
    leaves.par_iter()
      .map(|leaf| {
        let path = leaf.file_name()
          .unwrap()
          .to_str()
          .unwrap()
          .to_owned();

        let mut index_entries = Vec::new();
        let mut contents = String::new();
        try!(File::open(leaf)
          .and_then(|mut f| f.read_to_string(&mut contents)));
        for line in contents.lines() {
          index_entries.push(try!(serde_json::from_str::<T>(&line)))
        }

        Ok(vec![(path, index_entries)])
      }).reduce(|| Ok(Vec::new()), iter_util::aggregate_results)
  }
}

pub mod testing {
  use common::cargo::IndexEntry;
  use git2::Repository;
  use serde_json;
  use std::fs::File;
  use std::fs;
  use std::io::Write;
  use tempdir::TempDir;
  use std::path::PathBuf;

  /**
   * Constructs an "index-like" directory path for the given crate name.
   *
   * If the crate name is one character, the path is 1/$CRATE_NAME
   * If the crate name is two characters, the path is 2/$CRATE_NAME
   * If the crate name is three characters, the path is 3/$CRATE_NAME
   * If the crate name is four or more characters, the path is
   *   $FIRST_TWO_CHARS/$NEXT_TWO_CHARS/$CRATE_NAME
   */
  pub fn get_path_for_crate(crate_name: &str) -> PathBuf {
    match crate_name.len() {
      0 => panic!("Can't generate a path for an empty string"),
      1 => PathBuf::from(format!("1/{}", crate_name)),
      2 => PathBuf::from(format!("2/{}", crate_name)),
      3 => PathBuf::from(format!("3/{}", crate_name)),
      _ => PathBuf::from(format!("{}/{}/{}",
                                 crate_name[0..2].to_owned(),
                                 crate_name[2..4].to_owned(),
                                 crate_name)),
    }
  }

  /** Constructs a basic index directory with no contents. */
  pub fn seed_minimum_index() -> TempDir {
    let tempdir = TempDir::new("enpty_index").unwrap();
    Repository::init(tempdir.path()).unwrap();
    return tempdir;
  }

  /** Constructs an index directory seeded with the provided crates. */
  pub fn seed_index_with_crates(index_entries: Vec<IndexEntry>) -> TempDir {
    let index_tempdir = seed_minimum_index();

    for entry in index_entries.iter() {
      let path = get_path_for_crate(&entry.name);
      let path_from_index = index_tempdir.path().join(path);
      if let Some(ref parent) = path_from_index.parent() {
        fs::create_dir_all(parent).unwrap();
      };
      let mut crate_file = File::create(path_from_index).unwrap();
      let json = serde_json::to_string(&entry).unwrap();
      crate_file.write_all(json.as_bytes()).unwrap();
    }

    index_tempdir
  }
}

#[cfg(test)]
mod tests {
  use common::cargo::IndexEntry;
  use std::path::PathBuf;
  use super::*;
  use index::testing;

  #[test]
  fn get_path_for_crate_works_for_all_crate_names() {
    assert_eq!(testing::get_path_for_crate("a"),
               PathBuf::from("1/a"));
    assert_eq!(testing::get_path_for_crate("ab"),
               PathBuf::from("2/ab"));
    assert_eq!(testing::get_path_for_crate("abc"),
               PathBuf::from("3/abc"));
    assert_eq!(testing::get_path_for_crate("abcd"),
               PathBuf::from("ab/cd/abcd"));
  }

  #[test]
  fn test_empty_local_index_works() {
    let tempdir = testing::seed_minimum_index();
    let params = GenericIndexParams {
      url: Url::parse("http://not-resolvable").unwrap(),
      pre_pulled_index_path: Some(tempdir.path().to_path_buf()),
    };

    let upstream_index = GenericIndex::load_from_params(params).unwrap();

    assert_eq!(upstream_index.get_all_crate_keys(),
               Vec::new());
  }

  #[test]
  fn test_loads_trivial_index() {
      let index_entry = IndexEntry {
        name: "a".to_owned(),
        vers: "0.0.1".to_owned(),
        deps: Vec::new(),
        cksum: "111".to_owned(),
        features: HashMap::new(),
        yanked: None,
      };
    let tempdir = testing::seed_index_with_crates(vec![index_entry]);

    let params = GenericIndexParams {
      url: Url::parse("http://not-resolvable").unwrap(),
      pre_pulled_index_path: Some(tempdir.path().to_path_buf()),
    };

    let upstream_index = GenericIndex::load_from_params(params).unwrap();

    assert_eq!(upstream_index.get_all_crate_keys(),
               vec![CrateKey {
                 name: "a".to_owned(),
                 version: "0.0.1".to_owned()
               }]);
  }
}
