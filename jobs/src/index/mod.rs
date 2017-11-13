use common::cargo::CrateKey;
use ::JobErr;
use common::iter_util;
use git2::Repository;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json;
use std::fs::File;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tempdir::TempDir;
use url::Url;

pub mod crates_io;
pub mod augmented;

/**
 * A trait that is applicable to any object that has data keyed by CrateKey.
 *
 * Typically, indexes for Rust drates will be keyed off of CrateKey, and their contents will be
 * accessible using a CrateKey.
 */
pub trait KeyedByCrateKey {
  type Item;

  /** Enumerates all known crate keys. */
  fn get_crate_keys(&self) -> Vec<&CrateKey>;
  /** Fetches whatever the object stores using the provided CrateKey. */
  fn get_entry(&self, key: &CrateKey) -> Option<&Self::Item>;
}

/** The parameters required to load an arbitrary Crates.io-like index */
#[derive(Builder, Clone)]
pub struct GenericIndexParams {
  url: Url,
  pre_pulled_index_path: Option<PathBuf>,
}

/**
 * A generic loader object for any indexed data that is mastered in a Git repo, and has a folder
 * structure similar to the Crates.io folder structure.
 *
 * More specifically, a Git repository based index that is organized matching:
 * - If the artifact name is one character, the path is 1/$CRATE_NAME
 * - If the artifact name is two characters, the path is 0/$ARTIFACT_NAME
 * - If the artifact name is three characters, the path is 2/$ARTIFACT_NAME
 * - If the artifact name is four or more characters, the path is
 *   $FIRST_TWO_CHARS/$NEXT_TWO_CHARS/$ARTIFACT_NAME
 */
pub struct GenericIndexLoader {
  pub params: GenericIndexParams,
}

impl GenericIndexLoader {
  pub fn new(params: GenericIndexParams) -> GenericIndexLoader {
    GenericIndexLoader {
      params: params
    }
  }

  /**
   * Loads a generic index from the provided arguments.
   *
   * If a pre_pulled_index_path is provided, it is loaded directly. Otherwise, the index is pulled
   * into a temporary directory and loaded.
   */
  pub fn load_index<T: DeserializeOwned + Send>(&self) -> Result<(GenericIndexArtifacts, Vec<T>), JobErr> {
    let path;
    let repo;
    let maybe_tempdir;

    if let Some(ref raw_path) = self.params.pre_pulled_index_path {
      // Load from pre-pulled directory
      maybe_tempdir = None;
      debug!("Loading Index from {:?}", raw_path);
      path = PathBuf::from(raw_path);
      repo = try!(Repository::open(&path));
    } else {
      // Download into a temp dir
      let tempdir = try!(TempDir::new("upstream_in_memory_index"));
      debug!("Cloning upstream crates io index from {}, into {:?}", self.params.url, tempdir.path());
      repo = try!(Repository::clone(&self.params.url.to_string(), &tempdir.path()));
      path = PathBuf::from(tempdir.path());
      maybe_tempdir = Some(tempdir);
    }

    let contents = try!(self.load_contents(&path));
    Ok((GenericIndexArtifacts {
      repository: Arc::new(repo),
      tempdir: Arc::new(maybe_tempdir),
    }, contents))
  }

  /**
   * Extracts the contents of the index (whatever they are) by traversing the common index file
   * structure.
   */
  fn load_contents<T: DeserializeOwned + Send, P: AsRef<Path>>(&self, path: P) -> Result<Vec<T>, JobErr> {
    debug!("Loading crates.io-index from {:?}", path.as_ref());
    let mut dir_iters = Vec::new();
    let mut leaves = Vec::new();
    dir_iters.push(try!(fs::read_dir(&path)));
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

        Ok(index_entries)
      }).reduce(|| Ok(Vec::new()), iter_util::aggregate_results)
  }
}

/** The data-agostic byproducts of loading a generic index, such as tempdirs and Git repos. */
#[derive(Clone)]
pub struct GenericIndexArtifacts {
  pub repository: Arc<Repository>,
  pub tempdir: Arc<Option<TempDir>>
}

pub mod testing {
  use common::cargo;
  use git2::Repository;
  use index;
  use serde::Serialize;
  use serde_json;
  use std::fs::File;
  use std::fs;
  use std::io::Write;
  use std::path::PathBuf;
  use tempdir::TempDir;

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
  pub fn seed_index_with_contents<T: Serialize + Clone>(index_entries: Vec<T>) -> TempDir 
      where cargo::CrateKey: From<T> {
    let index_tempdir = index::testing::seed_minimum_index();

    for entry in index_entries.iter() {
      let key = cargo::CrateKey::from(entry.clone());
      let path = index::testing::get_path_for_crate(&key.name);
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
  use std::path::PathBuf;
  use ::index::testing;

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
}
