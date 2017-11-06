#[macro_use]
extern crate derive_builder;
extern crate common;
extern crate serde;
extern crate serde_yaml;
extern crate serde_json;
extern crate git2;
extern crate chrono;
#[macro_use(log, debug, warn)]
extern crate log;
extern crate rayon;
extern crate tempdir;

use git2::Repository;
use std::collections::HashMap;
use std::env;
use std::mem;
use std::path::PathBuf;
use tempdir::TempDir;
use common::WorkspaceMetadata;
use common::cargo;
use common::configuration::WorkspaceConfiguration;
use common::snapshot::WorkspaceSnapshot;

pub use err::ServerErr;


// TODO(acmcarther): Reconsider LocalPersistence:
// For now, its disabled because it leads to questions about reconciling local changes with
// global state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceLevel {
  NoPersistence,
  GlobalPersistence
}

#[derive(Builder, Debug)]
#[builder(default)]
pub struct ServerParams {
  repo_directory: PathBuf,
  persistence_level: PersistenceLevel,
  remote_repo: String,
  main_branch: String,
  upstream_crates_io_index: String,
}

impl Default for ServerParams {
  fn default() -> ServerParams {
    ServerParams {
      repo_directory: env::home_dir().unwrap().join(".cache/stockpile-index"),
      persistence_level: PersistenceLevel::NoPersistence,
      remote_repo: "https://github.com/acmcarther/stockpile-index".to_owned(),
      main_branch: "master".to_owned(),
      upstream_crates_io_index: "https://github.com/rust-lang/crates.io-index".to_owned(),
    }
  }
}

pub struct InProcessServer {
  local_state: LocalServerState,
  params: ServerParams,
}

mod err {
  use git2;
  use std::io;
  use serde_yaml;
  use serde_json;

  #[derive(Debug)]
  pub enum ServerErr {
    InvalidCacheState(String),
    InvalidOperation(String),
    TimedOut,
    UnderspecifiedPath,
    GitErr(git2::Error),
    IoErr(io::Error),
    SerdeYamlErr(serde_yaml::Error),
    SerdeJsonErr(serde_json::Error),
  }

  impl From<git2::Error> for ServerErr {
    fn from(error: git2::Error) -> ServerErr {
      ServerErr::GitErr(error)
    }
  }

  impl From<io::Error> for ServerErr {
    fn from(error: io::Error) -> ServerErr {
      ServerErr::IoErr(error)
    }
  }

  impl From<serde_yaml::Error> for ServerErr {
    fn from(error: serde_yaml::Error) -> ServerErr {
      ServerErr::SerdeYamlErr(error)
    }
  }

  impl From<serde_json::Error> for ServerErr {
    fn from(error: serde_json::Error) -> ServerErr {
      ServerErr::SerdeJsonErr(error)
    }
  }
}

mod iter_util {
  use std::fmt::Debug;

  pub fn aggregate_results<O, E: Debug>(left: Result<Vec<O>, E>, right: Result<Vec<O>, E>) -> Result<Vec<O>, E> {
    if left.is_err() {
      return left
    }
    if right.is_err() {
      return right
    }

    let mut l_inner = left.unwrap();
    let mut r_inner = right.unwrap();
    l_inner.append(&mut r_inner);
    Ok(l_inner)
  }
}

mod filesystem {
  use cargo;
  use err::ServerErr;
  use iter_util;
  use rayon::prelude::*;
  use serde_json;
  use std::collections::HashMap;
  use std::fs;
  use std::fs::OpenOptions;
  use std::io::Write;
  use std::path::Path;
  use std::path::PathBuf;

  pub struct FileWrite {
    pub path: PathBuf,
  }

  fn get_crates_io_index_path<P: AsRef<Path>>(crates_io_index_dir: P, crate_name: &str) -> PathBuf {
    assert!(crate_name.len() > 0);
    let suffix = match crate_name.len() {
      c @ 1 ... 3 => format!("{}/{}", c, crate_name),
      _ => format!("{}/{}/{}", &crate_name[0..2], &crate_name[2..4], crate_name),
    };

    crates_io_index_dir.as_ref().join(suffix)
  }

  pub fn write_crates_io_index<P: AsRef<Path> + Sync>(crates_io_index_dir: P, whole_index: &HashMap<String, Vec<cargo::IndexEntry>>) -> Result<Vec<FileWrite>, ServerErr> {
    println!("{:?}", crates_io_index_dir.as_ref());
    assert!(crates_io_index_dir.as_ref().ends_with("crates.io-index/"));
    whole_index
      .par_iter()
      .map(|(name, elems)| (get_crates_io_index_path(&crates_io_index_dir, &name), elems))
      .map(|(path, elems)| {
        try!(fs::create_dir_all(path.parent().unwrap()));
        let mut file = try!(OpenOptions::new().write(true).create(true).open(&path));
        let lines = try!(elems.iter()
                         .map(|e| serde_json::to_string(e).map(|c| format!("{}\n", c)))
                         .collect::<Result<String, serde_json::Error>>());
        try!(file.write_all(lines.as_bytes()));
        Ok(vec![FileWrite {
          path: path
        }])
      }).reduce(|| Ok(Vec::new()), iter_util::aggregate_results)
  }

  #[cfg(test)]
  mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use tempdir::TempDir;
    use init_util;

    #[test]
    fn test_get_crates_io_index_path_works() {
      assert_eq!(get_crates_io_index_path(PathBuf::from("./"), "c"), PathBuf::from("./1/c"));
      assert_eq!(get_crates_io_index_path(PathBuf::from("./"), "cr"), PathBuf::from("./2/cr"));
      assert_eq!(get_crates_io_index_path(PathBuf::from("./"), "cra"), PathBuf::from("./3/cra"));
      assert_eq!(get_crates_io_index_path(PathBuf::from("./"), "crate"), PathBuf::from("./cr/at/crate"));
    }

    #[test]
    fn test_write_crates_io_index_works() {
      // Set up mock dir
      let tempdir = TempDir::new("test_write_crates_io_index_works").unwrap();
      let crate_dir = tempdir.path().join("crates.io-index/cr/at/");
      fs::create_dir_all(&crate_dir).unwrap();
      let mut crate_file = File::create(crate_dir.join("crate")).unwrap();
      crate_file.write_all(
        b"{\"name\":\"crate\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"12345\",\"features\":{}, \"yanked\":false}\n").unwrap();


      let mut index_contents = HashMap::new();
      index_contents.insert("cr".to_owned(), vec![ cargo::IndexEntry {
        name: "cr".to_owned(),
        vers:"1.2.3".to_owned(),
        deps: Vec::new(),
        cksum: "12345".to_owned(),
        features: HashMap::new(),
        yanked: None,
        extra: None,
      }, cargo::IndexEntry {
        name: "cr".to_owned(),
        vers:"1.2.6".to_owned(),
        deps: Vec::new(),
        cksum: "123456".to_owned(),
        features: HashMap::new(),
        yanked: None,
        extra: None,
      }]);
      index_contents.insert("crate".to_owned(), vec![ cargo::IndexEntry {
        name: "crate".to_owned(),
        vers:"1.2.3".to_owned(),
        deps: Vec::new(),
        cksum: "12345".to_owned(),
        features: HashMap::new(),
        yanked: Some(true),
        extra: None,
      }]);

      write_crates_io_index(tempdir.path().join("crates.io-index/"),
                            &index_contents).unwrap();

      let index = init_util::load_crates_io_index(tempdir.path().join("crates.io-index/")).unwrap()
        .into_iter()
        .collect::<HashMap<String, Vec<cargo::IndexEntry>>>();

      assert_eq!(index, index_contents)
    }
  }
}

mod git {
  use git2::BranchType;
  use git2::PushOptions;
  use git2::RemoteCallbacks;
  use git2::Repository;
  use git2::ResetType;
  use err::ServerErr;
  use filesystem::FileWrite;

  // A modified version of rust-lang/crates.io/src/git.rs:commit_and_push
  pub fn update_repo_atomically<F>(repo: &Repository, message: &str, mut f: F) -> Result<(), ServerErr> where F: FnMut() -> Result<Vec<FileWrite>, ServerErr> {
    let mut retry_count = 0;

    while retry_count < 10 {
      let files = try!(f());

      {
        let mut git_index = try!(repo.index());
        for file in files {
          try!(git_index.add_path(&file.path));
        }
        try!(git_index.write());
      }
      let signature = try!(repo.signature());
      let master = try!(repo.find_branch("master", BranchType::Local));
      let master_tree = try!(repo.find_tree(master.get().target().unwrap()));
      let commit_oid = try!(repo.commit(
        None /* update_ref */,
        &signature /* author */,
        &signature /* committer */,
        &message,
        &master_tree,
        &[]));

      let mut origin = try!(repo.find_remote("origin"));
      let mut all_successes = true;
      {
        let mut push_options = PushOptions::new();
        let mut remote_callbacks = RemoteCallbacks::new();
        // TODO(acmcarther): Consider remote_callbacks.credentials
        remote_callbacks.push_update_reference(|path, status_opt| {
          assert_eq!(path, "refs/heads/master");
          if status_opt.is_some() {
            all_successes = false;
          }
          Ok(())
        });
        push_options.remote_callbacks(remote_callbacks);
        try!(origin.push(&["refs/heads/master"], Some(&mut push_options)));
      }
      if !all_successes {
        retry_count = retry_count + 1;
        let head_minus_one_commit = try!(repo.find_commit(commit_oid));
        let head_minus_one = try!(head_minus_one_commit.parent(0));
        try!(repo.reset(head_minus_one.as_object(), ResetType::Hard, None /*checkout*/));
        warn!("Failed to push new crates.io index to remote due to conflict, retrying {}/10", retry_count);
      } else {
        return Ok(())
      }
    }

    Err(ServerErr::TimedOut)
  }
}

mod init_util {
  use err::ServerErr;
  use git2::Repository;
  use iter_util;
  use rayon::prelude::*;
  use serde::Deserialize;
  use serde_json;
  use serde_yaml;
  use std::fs::File;
  use std::fs;
  use std::io::Read;
  use std::path::Path;
  use tools_common::cargo;
  use tools_common::snapshot::WorkspaceSnapshot;

  pub fn load_crates_io_index<P: AsRef<Path>>(crates_io_index_dir: P) -> Result<Vec<(String, Vec<cargo::IndexEntry>)>, ServerErr> {
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

  pub fn find_or_load_repository(repo_directory: &Path, remote_repo: &str) -> Result<Repository, ServerErr> {
    match fs::metadata(&repo_directory) {
      Ok(m) => {
        if !m.is_dir() {
          return Err(ServerErr::InvalidCacheState(
                format!("Tried to load local server state from {}, but it is a file, not a directory!",
                        repo_directory.to_str().unwrap_or("[UNRENDERABLE]"))));
        }
        let repo = try!(Repository::init(&repo_directory));
        {
          let mut remote = try!(repo.find_remote("origin"));
          try!(remote.fetch(&["master"], None, None));
        }
        Ok(repo)
      },
      Err(_) => {
        let parent = try!(repo_directory.parent()
                          .ok_or(ServerErr::UnderspecifiedPath));
        let _ = fs::create_dir_all(&parent);
        Repository::clone(&remote_repo, parent).map_err(ServerErr::from)
      }
    }
  }

  pub fn load_small_file<P: AsRef<Path>, O>(path: P) -> Result<O, ServerErr> where for<'a> O: Deserialize<'a> {
    debug!("Loading {:?}", path.as_ref());
    let mut contents = String::new();
    try!(File::open(path)
      .and_then(|mut f| f.read_to_string(&mut contents)));
    serde_yaml::from_str(&contents).map_err(ServerErr::from)
  }

  pub fn load_snapshots(repo_directory: &Path) -> Result<Vec<WorkspaceSnapshot>, ServerErr> {
    let snapshots_dir = repo_directory.join("snapshots/");
    let dir_iter = try!(fs::read_dir(&snapshots_dir));
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

      snapshots.push(try!(load_small_file::<_, WorkspaceSnapshot>(&file_path)))
    }
    Ok(snapshots)
  }
}

impl InProcessServer {
  pub fn create(params: ServerParams) -> Result<InProcessServer, ServerErr> {
    load_local_state(&params).map(|state| {
      InProcessServer {
        local_state: state,
        params: params,
      }
    })
  }

  pub fn incorporate_crates_io_index(&mut self, crates_io_index: Vec<(String, Vec<cargo::IndexEntry>)>, revision: String) -> Result<(), ServerErr> {
    if self.params.persistence_level != PersistenceLevel::GlobalPersistence {
      return Err(ServerErr::InvalidOperation("Synchronizing to crates.io is not possible in a non-persistent session".to_owned()));
    }

    let mut new_state = None;
    {
      // Pull the latest state of the index
      let cb = || {
        // Sync the local repo, and get all the new stuff from it
        let mut provisional_new_state = try!(load_local_state(&self.params));

        // Take all new crates from the crates-io resolution, and incorporate them
        // TODO(acmcarther): Revisit this. We're cloning the whole index every time
        for (package_name, mut new_entries) in crates_io_index.iter().cloned() {
          // If we know about the crate already
          if provisional_new_state.crates_io_index.contains_key(&package_name) {
            // Find our known versions
            let mut old_entries = provisional_new_state.crates_io_index.get_mut(&package_name).unwrap();
            // For all new crates
            for new_entry in new_entries.iter_mut() {
              // If we have a known version
              if let Some(mut old_entry) = old_entries.iter_mut().find(|e| e.vers == new_entry.vers) {
                // Take the "extra" out of it (we're dropping the rest of the old entry)
                mem::swap(&mut new_entry.extra, &mut old_entry.extra);
              }
            }

            // And swap the old entry set with these new, post "extras" entries
            mem::swap(old_entries, &mut new_entries);
          } else {
            provisional_new_state.crates_io_index.insert(package_name, new_entries);
          }
        }

        // Write new state to crates.io-index
        let files_written = try!(filesystem::write_crates_io_index(&self.params.repo_directory.join("/crates.io-index/"), 
                                                                   &provisional_new_state.crates_io_index));
        new_state = Some(provisional_new_state);
        Ok(files_written)
      };

      try!(git::update_repo_atomically(&self.local_state.index_repo,
                                  &format!("Update crates.io-index to \"{}\"", revision),
                                  cb));
    }
    mem::swap(&mut self.local_state, &mut new_state.unwrap());

    Ok(())
  }
}

pub trait IndexService {
  /**
   * Pulls the upstream Crates.io index and incorporates those changes into the local index.
   */
  fn sync_crates_io_index(&mut self) -> Result<(), ServerErr>;
}

impl IndexService for InProcessServer {
  fn sync_crates_io_index(&mut self) -> Result<(), ServerErr> {
    if self.params.persistence_level != PersistenceLevel::GlobalPersistence {
      return Err(ServerErr::InvalidOperation("Synchronizing to crates.io is not possible in a non-persistent session".to_owned()));
    }

    let dir = try!(TempDir::new("upstream_crates_io_index"));
    let crates_io_index_repo = try!(Repository::clone(&self.params.upstream_crates_io_index, &dir));
    let crates_io_index = try!(init_util::load_crates_io_index(dir.path().join("crates.io-index/")));
    let head = try!(crates_io_index_repo.head());
    let revision = try!(head.shorthand()
      .ok_or(ServerErr::InvalidCacheState("Could not find HEAD for crate_io_index repo".to_owned()))
      .map(str::to_owned));
    self.incorporate_crates_io_index(crates_io_index, revision)
  }
}

struct LocalServerState {
  known_snapshots: Vec<WorkspaceSnapshot>,
  crates_io_index: HashMap<String, Vec<cargo::IndexEntry>>,
  configuration: WorkspaceConfiguration,
  metadata: WorkspaceMetadata,
  index_repo: Repository,
}

fn load_local_state(params: &ServerParams) -> Result<LocalServerState, ServerErr> {
  debug!("Loading LocalServerState from {:?}", params.repo_directory);
  Ok(LocalServerState {
    index_repo: try!(init_util::find_or_load_repository(&params.repo_directory, &params.remote_repo)),
    configuration: try!(init_util::load_small_file(params.repo_directory.join("configuration.yaml"))),
    known_snapshots: try!(init_util::load_snapshots(&params.repo_directory)),
    metadata: try!(init_util::load_small_file(params.repo_directory.join("metadata.yaml"))),
    crates_io_index: try!(init_util::load_crates_io_index(params.repo_directory.join("crates.io-index/"))).into_iter().collect(),
  })
}

