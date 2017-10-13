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

  pub struct QueryParams {
    pub snapshot_version: Option<String>,
    pub repo_directory: Option<PathBuf>,
    pub crate_name: String,
  }

  pub struct TryAddingParams {
    pub snapshot_version: Option<String>,
    pub repo_directory: Option<PathBuf>,
    pub crate_name: String,
  }

  pub fn snapshot_now(params: SnapshotNowParams) {
    let mut server_params_builder = ServerParamsBuilder::default();
    if let Some(repo_dir) = params.repo_directory {
      server_params_builder.repo_directory(repo_dir);
    }

    let server = InProcessServer::create(server_params_builder.build().unwrap()).unwrap();
    server.debug();
  }

  pub fn query(params: QueryParams) {
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
