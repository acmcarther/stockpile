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

type AbsurdAwsType =
  aws_sdk_rust::aws::s3::s3client::S3Client<
    aws_sdk_rust::aws::common::credentials::BaseAutoRefreshingProvider<
      aws_sdk_rust::aws::common::credentials::ChainProvider,
      std::cell::RefCell<aws_sdk_rust::aws::common::credentials::AwsCredentials>>,
    hyper::Client>;

#[derive(Clone)]
pub struct SimpleS3Client {
  pub inner_client: Arc<AbsurdAwsType>
}

pub struct SimpleS3ClientParams {
  pub api_url: String,
  pub access_key_id: String,
  pub secret_access_key: String,
}

impl SimpleS3Client {
  pub fn new(params: SimpleS3ClientParams) -> SimpleS3Client {
    let s3_url = Url::parse(&params.api_url).unwrap();

    let parameters_provider =
        ParametersProvider::with_parameters(
            params.access_key_id.as_str(),
            params.secret_access_key.as_str(),
            None).unwrap();
    let provider = DefaultCredentialsProvider::new(Some(parameters_provider)).unwrap();

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
