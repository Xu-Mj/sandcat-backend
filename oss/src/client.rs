use crate::Oss;
use abi::config::Config;
use abi::errors::Error;
use async_trait::async_trait;
use aws_sdk_s3::config::{Builder, Credentials, Region};
use aws_sdk_s3::Client;
use aws_smithy_runtime_api::client::result::SdkError;
use bytes::Bytes;
use tracing::error;

#[derive(Debug, Clone)]
pub(crate) struct S3Client {
    bucket: String,
    client: Client,
}

impl S3Client {
    pub async fn new(config: &Config) -> Self {
        let credentials = Credentials::new(
            &config.oss.access_key,
            &config.oss.secret_key,
            None,
            None,
            "MinioCredentials",
        );

        let bucket = config.oss.bucket.clone();

        let config = Builder::new()
            .region(Region::new(config.oss.region.clone()))
            .credentials_provider(credentials)
            .endpoint_url(&config.oss.endpoint)
            // use latest behavior version, have to set it manually,
            // although we turn on the feature
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .build();

        let client = Client::from_conf(config);

        let self_ = Self { client, bucket };

        self_.create_bucket().await.unwrap();
        self_
    }

    async fn check_bucket_exists(&self) -> Result<bool, Error> {
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_response) => Ok(true),
            Err(SdkError::ServiceError(e)) => {
                if e.raw().status().as_u16() == 404 {
                    Ok(false)
                } else {
                    Err(Error::InternalServer(
                        "check_bucket_exists error".to_string(),
                    ))
                }
            }
            Err(e) => {
                error!("check_bucket_exists error: {:?}", e);
                Err(Error::InternalServer(e.to_string()))
            }
        }
    }

    async fn create_bucket(&self) -> Result<(), Error> {
        let is_exist = self.check_bucket_exists().await?;
        if is_exist {
            return Ok(());
        }
        self.client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl Oss for S3Client {
    async fn file_exists(&self, key: &str, local_md5: &str) -> Result<bool, Error> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(resp) => {
                if let Some(etag) = resp.e_tag() {
                    // remove the double quotes
                    let etag = etag.trim_matches('"');
                    Ok(etag == local_md5)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    async fn upload_file(&self, key: &str, content: Vec<u8>) -> Result<(), Error> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(content.into())
            .send()
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn download_file(&self, key: &str) -> Result<Bytes, Error> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;

        Ok(data.into_bytes())
    }

    async fn delete_file(&self, key: &str) -> Result<(), Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Error::InternalServer(e.to_string()))?;

        Ok(())
    }
}