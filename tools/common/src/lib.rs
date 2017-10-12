extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;

pub mod manifest {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceManifest {
    crates: Vec<MaintainerManifest>,
    skip_dev_dependencies: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct MaintainerManifest {
    maintainer: String,
    crats: Vec<String>,
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

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
