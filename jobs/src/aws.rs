use aws_sdk_rust::aws::common::credentials::DefaultCredentialsProvider;
use aws_sdk_rust::aws::common::credentials::ParametersProvider;
use aws_sdk_rust::aws::common::region::Region;
use aws_sdk_rust::aws::s3::endpoint::Endpoint;
use aws_sdk_rust::aws::s3::endpoint::Signature;
use aws_sdk_rust::aws::s3::s3client::S3Client;
use aws_sdk_rust;
use hyper;
use std;
use std::sync::Arc;
use url::Url;

mod flags {
  define_pub_cfg!(s3_access_key_id,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 access key id credential to be used.");
  define_pub_cfg!(s3_secret_access_key,
                  ::zcfg::NoneableCfg<String>,
                  None,
                  "The S3 secret key credential to be used.");
}

type AbsurdAwsType =
  aws_sdk_rust::aws::s3::s3client::S3Client<
    aws_sdk_rust::aws::common::credentials::BaseAutoRefreshingProvider<
      aws_sdk_rust::aws::common::credentials::ChainProvider,
      std::cell::RefCell<aws_sdk_rust::aws::common::credentials::AwsCredentials>>,
    hyper::Client>;

define_pub_cfg!(s3_api_url,
                String,
                "http://minio-small".to_owned(),
                "The location of the local crate service S3 Bucket.");

#[derive(Clone)]
pub struct SimpleS3Client {
  pub inner_client: Arc<AbsurdAwsType>
}

impl Default for SimpleS3Client {
  fn default() -> SimpleS3Client {
    let s3_url = Url::parse(&s3_api_url::CONFIG.get_value()).unwrap();
    let provider = DefaultCredentialsProvider::new(Some(get_default_s3_params())).unwrap();
    let endpoint = Endpoint::new(
      Region::UsEast1 /* irrelevant for internal */,
      Signature::V4,
      Some(s3_url),
      None /* proxy */,
      None /* user_agent */,
      None /* is_bucket_virtual */);
    let inner_client = S3Client::new(provider, endpoint);
    SimpleS3Client {
      inner_client: Arc::new(inner_client)
    }
  }
}

fn get_default_s3_params() -> ParametersProvider {
  let access_key_id =
    flags::s3_access_key_id::CONFIG.get_value().inner()
      .expect("--s3_access_key_id must be set");
  let secret_access_key =
    flags::s3_secret_access_key::CONFIG.get_value().inner()
      .expect("--s3_secret_access_key must be set");
  ParametersProvider::with_parameters(
      access_key_id,
      secret_access_key,
      None).unwrap()
}
