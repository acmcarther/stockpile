use aws::SimpleS3Client;
use aws::SimpleS3ClientParams;
use aws_sdk_rust::aws::errors::s3::S3Error;
use aws_sdk_rust::aws::s3::object::ListObjectsOutput;
use aws_sdk_rust::aws::s3::object::ListObjectsRequest;
use common::cargo::CrateKey;
use hyper::Client;
use hyper::header::Connection;
use hyper;
use index::UpstreamIndex;
use index::UpstreamIndexParams;
use lcs_fetcher::LcsFetchErr;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use super::Job;
use tempdir::TempDir;

mod flags {
  define_pub_cfg!(s3_crate_bucket_name,
                  String,
                  "local-crate-service",
                  "The name of the S3 bucket where crates are stored.");
  define_pub_cfg!(s3_access_key_id,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 access key id credential to be used.");
  define_pub_cfg!(s3_secret_access_key,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 secret key credential to be used.");
  define_pub_cfg!(s3_api_url,
                  String,
                  "http://minio-small".to_owned(),
                  "The location of the local crate service S3 Bucket.");
  define_pub_cfg!(upstream_crate_server_url,
                  String,
                  "https://crates-io.s3-us-west-1.amazonaws.com/crates/",
                  "The url prefix for the upstream crate system. Typically cargo's backing storage.");
}

/** A "LocalCrateService" repository source, which can furnish crates tarballs. */
pub trait LcsRepositorySource: __LcsRepositorySource_BoxClone {
  /**
   * Retrieves the provided CrateKey from the internal repository, and writes it into the 
   * destination directory.
   */
  fn fetch_crate(&self, key: &CrateKey, destination: &Path) -> Result<(), LcsFetchErr>;
}
define_box_clone_boilerplate!(LcsRepositorySource, __LcsRepositorySource_BoxClone);

/** A "LocalCrateService" repository sink, which can receive new crates. */
pub trait LcsRepositorySink: __LcsRepositorySink_BoxClone {
  /** Retrieves all known crate keys. */
  fn get_existing_crate_keys(&self) -> Result<Vec<CrateKey>, LcsFetchErr>;

  /** Uploads a new crate with the provide crate key for the file at the path. */
  fn upload_crate(&mut self, key: &CrateKey, path: &Path) -> Result<(), LcsFetchErr>;
}
define_box_clone_boilerplate!(LcsRepositorySink, __LcsRepositorySink_BoxClone);


/** A "LocalCrateService" repository defined out of the local file system. */
#[derive(Clone)]
pub struct LocalFsLcsRepository {
  crates_path: PathBuf,
  backing_tmpdir: Arc<Option<TempDir>>
}

impl LocalFsLcsRepository {
  fn new(crates_path: PathBuf) -> LocalFsLcsRepository {
    assert!(crates_path.is_dir());
    LocalFsLcsRepository {
      crates_path: crates_path,
      backing_tmpdir: Arc::new(None)
    }
  }

  fn from_tmp() -> Result<LocalFsLcsRepository, LcsFetchErr> {
    let tempdir = try!(TempDir::new("local_fs_lcs_repo"));
    let index_path =
      tempdir.path().join("index.txt");
    try!(File::create(index_path));

    Ok(LocalFsLcsRepository {
      crates_path: tempdir.path().to_path_buf(),
      backing_tmpdir: Arc::new(Some(tempdir)),
    })
  }

  fn get_index_path(&self) -> PathBuf {
    self.crates_path.join("index.txt")
  }
}

impl LcsRepositorySink for LocalFsLcsRepository {
  fn get_existing_crate_keys(&self) -> Result<Vec<CrateKey>, LcsFetchErr> {
    // TODO(acmcarther): Clean this API up. Its super unsafe
    let index_path =
      self.crates_path.join("index.txt");

    let mut index_file = try!(File::open(index_path));
    let mut contents = String::new();
    try!(index_file.read_to_string(&mut contents));
    Ok(contents.lines()
      .map(|line| line.split(':').collect::<Vec<_>>())
      .map(|line_parts| {
        CrateKey {
          name: line_parts.get(0).unwrap().to_string(),
          version: line_parts.get(1).unwrap().to_string(),
        }
      }).collect())
  }

  fn upload_crate(&mut self, key: &CrateKey, path: &Path) -> Result <(), LcsFetchErr> {
    // TODO(acmcarther): Clean this API up. Its super unsafe
    let crate_filename = format!("{name}-{version}.crate",
                                 name = key.name,
                                 version = key.version);
    let index_path = self.crates_path.join("index.txt");
    let crate_path = self.crates_path.join(&crate_filename);

    try!(fs::copy(path, crate_path));

    let mut index_file = try!(OpenOptions::new()
      .append(true)
      .open(index_path));

    try!(index_file.write_all(format!("{name}:{version}\n",
                                 name = key.name,
                                 version = key.version).as_bytes()));
    Ok(())

  }
}

/** A "LocalCrateService" repository defined out of S3. */
#[derive(Clone)]
pub struct S3LcsRepository {
  s3_bucket_name: String,
  s3_client: SimpleS3Client,
}

impl Default for S3LcsRepository {
  fn default() -> S3LcsRepository {
    S3LcsRepository {
      s3_bucket_name:
        flags::s3_crate_bucket_name::CONFIG.get_value(),
      s3_client: SimpleS3Client::new(SimpleS3ClientParams {
        api_url:
          flags::s3_api_url::CONFIG.get_value(),
        access_key_id:
          flags::s3_access_key_id::CONFIG.get_value().inner()
            .expect("--s3_access_key_id must be set"),
        secret_access_key:
          flags::s3_secret_access_key::CONFIG.get_value().inner()
            .expect("--s3_secret_access_key must be set"),
      }),
    }
  }
}

impl LcsRepositorySink for S3LcsRepository {
  fn get_existing_crate_keys(&self) -> Result<Vec<CrateKey>, LcsFetchErr> {
    let request = ListObjectsRequest {
      bucket: self.s3_bucket_name.clone(),
      version: Some(1),
      prefix: None,
      max_keys: None,
      marker: None,
      delimiter: None,
      encoding_type: None,
    };
    let response = try!(self.s3_client.inner_client.list_objects(&request));
    let contents = response.contents;
    Ok(contents.into_iter()
      .map(|c| c.key)
      .map(|k| {
        let split = k.split(':').collect::<Vec<_>>();
        CrateKey {
          name: split.get(0).cloned().unwrap().to_owned(),
          version: split.get(1).cloned().unwrap().to_owned(),
        }
      }).collect())
  }

  fn upload_crate(&mut self, key: &CrateKey, path: &Path) -> Result<(), LcsFetchErr> {
    // TODO: Stub
    warn!("S3LcsRepository::upload_crate is unimplemented");
    Ok(())
  }
}


/** A "LocalCrateService" repository defined from some HTTP server. */
#[derive(Clone)]
pub struct HttpLcsRepository {
  http_prefix: String,
  client: Arc<Client>,
}

impl Default for HttpLcsRepository {
  fn default() -> HttpLcsRepository {
    HttpLcsRepository {
      http_prefix: flags::upstream_crate_server_url::CONFIG.get_value(),
      client: Arc::new(Client::new()),
    }
  }
}

impl HttpLcsRepository {
  fn new(http_prefix: String) -> HttpLcsRepository {
    HttpLcsRepository {
      http_prefix: http_prefix,
      client: Arc::new(Client::new()),
    }
  }
}

impl LcsRepositorySource for HttpLcsRepository {
  // TODO(acmcarther): Handle errors gracefully
  fn fetch_crate(&self, key: &CrateKey, destination: &Path) -> Result<(), LcsFetchErr>{
    let full_url = format!("{prefix}/{crate_name}/{crate_name}-{crate_version}.crate",
                           prefix=self.http_prefix,
                           crate_name=key.name,
                           crate_version=key.version);

    let mut res = try!(self.client.get(&full_url)
      .header(Connection::close())
      .send());

    let mut bytes = Vec::new();
    try!(res.read_to_end(&mut bytes));

    let output_path = destination.join(&format!("/{crate_name}-{crate_version}.crate",
                                                crate_name=key.name,
                                                crate_version=key.version));

    let mut file = try!(File::create(&output_path));

    try!(file.write_all(bytes.as_slice()));
    Ok(())
  }
}

mod testing {
  use super::*;
  use tempdir::TempDir;

  pub struct TestingCrate {
    pub key: CrateKey,
    pub contents: Vec<u8>
  }

  pub fn create_localfs_for_testing(crates: &Vec<TestingCrate>) -> Result<LocalFsLcsRepository, LcsFetchErr> {
    let mut lfs_lcs_repo = try!(LocalFsLcsRepository::from_tmp());
    let tempdir = try!(TempDir::new("seed_crates"));

    for krate in crates.iter() {
      let crate_path = tempdir.path().join(&format!("{name}-{version}.crate",
                                                    name = krate.key.name,
                                                    version = krate.key.version));
      let mut crate_on_fs = try!(File::create(&crate_path));
      try!(crate_on_fs.write_all(krate.contents.as_slice()));
      try!(lfs_lcs_repo.upload_crate(&krate.key, &crate_path));
    }

    return Ok(lfs_lcs_repo);
  }
}

#[cfg(test)]
mod tests {
  pub use super::*;
  mod localfs {
    use lcs_fetcher::repository::testing::TestingCrate;
    use lcs_fetcher::repository::testing;
    use super::*;

    #[test]
    fn test_empty_fs_behaves_correctly() {
      let lfs_lcs_repo = LocalFsLcsRepository::from_tmp().unwrap();
      let get_res = lfs_lcs_repo.get_existing_crate_keys();

      assert!(get_res.is_ok());
      assert_eq!(get_res.unwrap(), Vec::new());
    }

    #[test]
    fn test_seeded_fs_contains_expected_crates() {
      let testing_crates = vec![
        TestingCrate {
          key: CrateKey {
            name: "example".to_owned(),
            version: "1.0.0".to_owned(),
          },
          contents: b"CrateTarContents".to_vec()
        }
      ];
      let lfs_lcs_repo = testing::create_localfs_for_testing(&testing_crates).unwrap();

      let get_res = lfs_lcs_repo.get_existing_crate_keys();

      assert!(get_res.is_ok());
      assert_eq!(get_res.unwrap(), vec![CrateKey {
        name: "example".to_owned(),
        version: "1.0.0".to_owned(),
      }]);
    }

  }

  mod http {
    // TODO(acmcarther): Spin up hyper testing this
  }

  mod s3 {
    // TODO(acmcarther): Some kind of testingg strategy
  }
}
