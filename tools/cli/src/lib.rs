extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;
extern crate tools_server;

pub mod commands {
  use tools_server::InProcessServer;
  use tools_server::ServerParamsBuilder;
  use std::path::PathBuf;
  use std::env;

  pub struct SnapshotNowParams {
    pub repo_directory: Option<PathBuf>,
  }

  impl <'a> From<&'a SnapshotNowParams> for ServerParamsBuilder {
    fn from(params: &'a SnapshotNowParams) -> ServerParamsBuilder {
      let mut server_params_builder = ServerParamsBuilder::default();
      if let Some(ref repo_dir) = params.repo_directory {
        server_params_builder.repo_directory(repo_dir.clone());
      }
      server_params_builder
    }
  }

  pub fn snapshot_now(params: SnapshotNowParams) {
    let server_params = ServerParamsBuilder::from(&params).build().unwrap();
    let server = InProcessServer::create(server_params);
  }

  pub struct SyncIndexParams{
    pub repo_directory: Option<PathBuf>,
  }

  impl <'a> From<&'a SyncIndexParams> for ServerParamsBuilder {
    fn from(params: &'a SyncIndexParams) -> ServerParamsBuilder {
      let mut server_params_builder = ServerParamsBuilder::default();
      if let Some(ref repo_dir) = params.repo_directory {
        server_params_builder.repo_directory(repo_dir.clone());
      }
      server_params_builder
    }
  }

  pub fn sync_index(params: SyncIndexParams) {
    let server_params = ServerParamsBuilder::from(&params).build().unwrap();
    let server = InProcessServer::create(server_params);

    server.synchronize_index();
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
