#[macro_use]
extern crate derive_builder;
extern crate tools_common;
extern crate serde;
extern crate serde_yaml;
extern crate git2;

use std::env;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::io::Read;
use tools_common::snapshot::WorkspaceSnapshot;
use tools_common::manifest::WorkspaceManifest;
use git2::Repository;

#[derive(Builder, Debug)]
#[builder(default)]
pub struct ServerParams {
  repo_directory: PathBuf,
  persistent: bool,
  remote_repo: String,
}

impl Default for ServerParams {
  fn default() -> ServerParams {
    ServerParams {
      repo_directory: env::home_dir().unwrap().join(".cache/stockpile"),
      persistent: false,
      remote_repo: "https://github.com/acmcarther/stockpile".to_owned(),
    }
  }
}

pub struct InProcessServer {
  local_state: LocalServerState,
  params: ServerParams,
}

impl InProcessServer {
  pub fn create(params: ServerParams) -> InProcessServer {
    InProcessServer {
      local_state: load_local_repo(&params),
      params: params,
    }
  }

  pub fn debug(&self) {
    println!("{:#?}", self.local_state)
  }
}

#[derive(Debug, Clone)]
struct LocalServerState {
  known_snapshots: Vec<WorkspaceSnapshot>,
  manifest: WorkspaceManifest,
}

fn load_local_repo(params: &ServerParams) -> LocalServerState {
  // Verify dir exists, or create it
  match fs::metadata(&params.repo_directory) {
    Ok(m) => {
      if !m.is_dir() {
        panic!("Tried to load local server state from {}, but it is a file, not a directory!",
               params.repo_directory.to_str().unwrap_or("[UNRENDERABLE]"))
      }
    },
    Err(_) => {
      let parent = params.repo_directory.parent().unwrap();
      let _ = fs::create_dir_all(&parent);
      let repo = Repository::clone(&params.remote_repo, parent).unwrap();
    }
  };

  let manifest_path = params.repo_directory.join("manifest.yaml");
  let mut manifest_str = String::new();
  File::open(manifest_path).unwrap().read_to_string(&mut manifest_str).unwrap();
  let manifest = serde_yaml::from_str::<WorkspaceManifest>(&manifest_str).unwrap();

  let snapshots_dir = params.repo_directory.join("snapshots/");
  let snapshots = match fs::metadata(&snapshots_dir) {
    Ok(m) => {
      if m.is_dir() {
        fs::read_dir(&snapshots_dir).unwrap()
          .map(|entry| entry.unwrap())
          .filter(|entry| match entry.path().file_name() {
            None => false,
            Some(file_name) => {
              let s = file_name.to_str().unwrap();
              s.starts_with("snapshot") && s.ends_with(".yaml")
            }
          }).map(|entry| {
            let mut snapshot_str = String::new();
            let file = File::open(entry.path()).unwrap().read_to_string(&mut snapshot_str).unwrap();
            let snapshot = serde_yaml::from_str::<WorkspaceSnapshot>(&snapshot_str).unwrap();
            snapshot
          }).collect::<Vec<_>>()
      } else {
        Vec::new()
      }
    },
    Err(_) => {
      Vec::new()
    }
  };

  LocalServerState {
    known_snapshots: snapshots,
    manifest: manifest,
  }
}
