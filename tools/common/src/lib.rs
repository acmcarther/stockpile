extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

pub mod manifest {
  pub struct WorkspaceManifest {
  }
}

pub mod snapshot {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceSnapshot {
    version: String,
    members: Vec<String>,
    details: Vec<CrateSnapshot>
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
