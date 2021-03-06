extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;

pub mod commands {
  use std::path::PathBuf;
  use std::env;

  pub struct SnapshotNowParams {
    pub repo_directory: Option<PathBuf>,
  }

  pub fn snapshot_now(params: SnapshotNowParams) {
  }

  pub struct SyncIndexParams{
    pub repo_directory: Option<PathBuf>,
  }


  pub fn sync_index(params: SyncIndexParams) {
  }

  pub struct QueryParams {
    pub snapshot_version: Option<String>,
    pub repo_directory: Option<PathBuf>,
    pub crate_name: String,
  }

  pub fn query(params: QueryParams) {
  }

  pub struct TryAddingParams {
    pub snapshot_version: Option<String>,
    pub repo_directory: Option<PathBuf>,
    pub crate_name: String,
  }

  pub fn try_adding(params: TryAddingParams) {
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
