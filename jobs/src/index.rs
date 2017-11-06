use common::cargo;
use git2::Repository;
use git2;
use serde_json;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tempdir::TempDir;
use url::Url;

pub struct UpstreamIndexParams {
  url: Url,
  pre_pulled_crate_path: Option<String>,
}

impl Default for UpstreamIndexParams {
  fn default() -> UpstreamIndexParams {
    let url = Url::parse(&::flags::crates_io_index_url::CONFIG.get_value()).unwrap();
    let pre_pulled_crate_path = ::flags::pre_pulled_crates_io_index_directory::CONFIG.get_value().inner();

    UpstreamIndexParams {
      url: url,
      pre_pulled_crate_path: pre_pulled_crate_path,
    }
  }
}

pub struct UpstreamIndex {
  crates_io_index_repo: Repository,
  crates_io_index: HashMap<String, Vec<cargo::IndexEntry>>,
}

#[derive(Debug)]
pub enum UpstreamIndexErr {
  InvalidCacheState(String),
  GitErr(git2::Error),
  IoErr(io::Error),
  SerdeJsonErr(serde_json::Error),
}

impl From<git2::Error> for UpstreamIndexErr {
  fn from(error: git2::Error) -> UpstreamIndexErr {
    UpstreamIndexErr::GitErr(error)
  }
}

impl From<io::Error> for UpstreamIndexErr {
  fn from(error: io::Error) -> UpstreamIndexErr {
    UpstreamIndexErr::IoErr(error)
  }
}

impl From<serde_json::Error> for UpstreamIndexErr {
  fn from(error: serde_json::Error) -> UpstreamIndexErr {
    UpstreamIndexErr::SerdeJsonErr(error)
  }
}

impl UpstreamIndex {
  pub fn load_from_params(params: UpstreamIndexParams) -> Result<UpstreamIndex, UpstreamIndexErr> {
    let repo: Repository;
    let path: PathBuf;
    if params.pre_pulled_crate_path.is_some() {
      path = PathBuf::from(params.pre_pulled_crate_path.unwrap());
      debug!("Loading Index from {:?}", path);
      repo = try!(Repository::open(&path));
    } else {
      debug!("Creating temp dir for upstream crates io index");
      let dir = try!(TempDir::new("upstream_crates_io_index"));
      debug!("Cloning upstream crates io index from {}, into {:?}", params.url, dir.path());
      repo = try!(Repository::clone(&params.url.to_string(), &dir));
      path = dir.path().join("crates.io-index/");
    }
    debug!("Loading the contents of the index");
    let crates_io_index = try!(init_util::load_crates_io_index(path));

    Ok(UpstreamIndex {
      crates_io_index_repo: repo,
      crates_io_index: crates_io_index.into_iter().collect(),
    })
  }
}


mod init_util {
  use git2::Repository;
  use rayon::prelude::*;
  use serde::Deserialize;
  use serde_json;
  use common::iter_util;
  use std::fs::File;
  use std::fs;
  use std::io::Read;
  use std::path::Path;
  use common::cargo;
  use common::snapshot::WorkspaceSnapshot;
  use super::UpstreamIndexErr;

  pub fn load_crates_io_index<P: AsRef<Path>>(crates_io_index_dir: P) -> Result<Vec<(String, Vec<cargo::IndexEntry>)>, UpstreamIndexErr> {
    assert!(crates_io_index_dir.as_ref().ends_with("crates.io-index/"));
    debug!("Loading crates.io-index from {:?}", crates_io_index_dir.as_ref());
    let mut dir_iters = Vec::new();
    let mut leaves = Vec::new();
    dir_iters.push(try!(fs::read_dir(&crates_io_index_dir)));
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
          index_entries.push(try!(serde_json::from_str::<cargo::IndexEntry>(&line)))
        }

        Ok(vec![(path, index_entries)])
      }).reduce(|| Ok(Vec::new()), iter_util::aggregate_results)
  }
}
