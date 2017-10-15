extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;
extern crate chrono;

use chrono::DateTime;
use chrono::Utc;

pub mod cargo {
  use super::*;
  use std::collections::HashMap;
  // Mostly a copy from github/rust-lang/crates.io/src/git.rs
  // WARNING: On sync from upstream crates.io-index, all modifications
  // besides the "extra" entry are lost. Add new metadata into "extra".
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct IndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<DependencyEntry>,
    pub cksum: String,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: Option<bool>,
    pub extra: Option<ExtraEntry>,
  }

  // Mostly a copy from github/rust-lang/crates.io/src/git.rs
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct DependencyEntry {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<String>,
  }

  // Stockpile-specific data added to the index entry
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct ExtraEntry {
    dev_dependencies: Option<Vec<DependencyEntry>>,
  }
}

pub mod configuration {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceConfiguration {
    crate_sets: Vec<MaintainerConfiguration>,
    skip_dev_dependencies: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct MaintainerConfiguration {
    maintainer: String,
    crates: Vec<String>,
  }
}

pub mod snapshot {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceSnapshot {
    version: String,
    members: Vec<String>,
    details: Vec<CrateSnapshot>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct CrateSnapshot {
    name: String,
    version: String,
    maintainer: String,
    dependencies: String,
    resolution_type: Option<ResolutionType>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ResolutionType {
    crates_io: Option<bool>,
    git: Option<GitResolution>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct GitResolution {
    repository: String,
    revision: String,
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
  last_index_time: Option<DateTime<Utc>>,
  crates_io_index_revision: String,
}


#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
