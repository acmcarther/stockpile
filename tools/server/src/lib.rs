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
use std::path::Path;
use std::io::Read;
use std::io;
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
  pub fn create(params: ServerParams) -> Result<InProcessServer, ServerInitErr> {
    load_local_state(&params).map(|state| {
      InProcessServer {
        local_state: state,
        params: params,
      }
    })
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

#[derive(Debug)]
pub enum ServerInitErr {
  InvalidCacheState(String),
  UnderspecifiedPath,
  GitErr(git2::Error),
  IoErr(io::Error),
  SerdeErr(serde_yaml::Error),
}

impl From<git2::Error> for ServerInitErr {
  fn from(error: git2::Error) -> ServerInitErr {
    ServerInitErr::GitErr(error)
  }
}

impl From<io::Error> for ServerInitErr {
  fn from(error: io::Error) -> ServerInitErr {
    ServerInitErr::IoErr(error)
  }
}

impl From<serde_yaml::Error> for ServerInitErr {
  fn from(error: serde_yaml::Error) -> ServerInitErr {
    ServerInitErr::SerdeErr(error)
  }
}

fn find_or_load_repository(params: &ServerParams) -> Result<Repository, ServerInitErr> {
  match fs::metadata(&params.repo_directory) {
    Ok(m) => {
      if !m.is_dir() {
        return Err(ServerInitErr::InvalidCacheState(
              format!("Tried to load local server state from {}, but it is a file, not a directory!",
                      params.repo_directory.to_str().unwrap_or("[UNRENDERABLE]"))));
      }
      Repository::init(&params.repo_directory).map_err(ServerInitErr::from)
    },
    Err(_) => {
      let parent = try!(params.repo_directory.parent()
                        .ok_or(ServerInitErr::UnderspecifiedPath));
      let _ = fs::create_dir_all(&parent);
      Repository::clone(&params.remote_repo, parent).map_err(ServerInitErr::from)
    }
  }
}

fn load_small_file<P: AsRef<Path>>(path: P) -> Result<String, ServerInitErr> {
  let mut contents = String::new();
  try!(File::open(path)
    .map(|mut f| f.read_to_string(&mut contents))
    .map_err(ServerInitErr::from));
  Ok(contents)
}

fn load_local_state(params: &ServerParams) -> Result<LocalServerState, ServerInitErr> {
  let repo = try!(find_or_load_repository(&params));
  let manifest_str = try!(load_small_file(params.repo_directory.join("manifest.yaml")));
  let manifest = try!(serde_yaml::from_str::<WorkspaceManifest>(&manifest_str)
                      .map_err(ServerInitErr::from));

  let snapshots_dir = params.repo_directory.join("snapshots/");
  let dir_iter = try!(fs::read_dir(&snapshots_dir)
                      .map_err(ServerInitErr::from));
  let mut snapshots = Vec::new();

  for entry_res in dir_iter {
    // Skip unreadable files
    if entry_res.is_err() {
      continue
    }
    let entry = entry_res.unwrap();
    let file_path = entry.path();

    // Skip unknown files
    if !file_path.starts_with("snapshot") || !file_path.ends_with("yaml") {
      continue
    }

    let snapshot_str = try!(load_small_file(&file_path));
    let snapshot = try!(serde_yaml::from_str::<WorkspaceSnapshot>(&snapshot_str)
                        .map_err(ServerInitErr::from));
    snapshots.push(snapshot);
  }

  Ok(LocalServerState {
    known_snapshots: snapshots,
    manifest: manifest,
  })
}
