//! S3/R2 storage service for MD file backups.

use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client, Config,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("S3 error: {0}")]
    S3(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("File not found: {0}")]
    NotFound(String),
}

/// S3/R2 storage service for file uploads and downloads.
pub struct StorageService {
    client: Client,
    bucket: String,
}

impl StorageService {
    /// Create a new storage service from environment variables.
    ///
    /// Required env vars:
    /// - S3_BUCKET: Bucket name
    /// - S3_REGION: Region (use "auto" for Cloudflare R2)
    /// - S3_ENDPOINT: Custom endpoint URL (required for R2)
    /// - S3_ACCESS_KEY: Access key ID
    /// - S3_SECRET_KEY: Secret access key
    pub async fn new() -> Result<Self, StorageError> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| StorageError::Config("S3_BUCKET not set".to_string()))?;

        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "auto".to_string());

        let endpoint = std::env::var("S3_ENDPOINT").ok();

        let access_key = std::env::var("S3_ACCESS_KEY")
            .map_err(|_| StorageError::Config("S3_ACCESS_KEY not set".to_string()))?;

        let secret_key = std::env::var("S3_SECRET_KEY")
            .map_err(|_| StorageError::Config("S3_SECRET_KEY not set".to_string()))?;

        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,  // session token
            None,  // expiry
            "env", // provider name
        );

        let mut config_builder = Config::builder()
            .region(Region::new(region))
            .credentials_provider(credentials)
            .behavior_version_latest();

        // Set custom endpoint for R2 or other S3-compatible services
        if let Some(endpoint_url) = endpoint {
            config_builder = config_builder.endpoint_url(endpoint_url);
        }

        let config = config_builder.build();
        let client = Client::from_conf(config);

        Ok(Self { client, bucket })
    }

    /// Upload a file to S3.
    ///
    /// # Arguments
    /// * `key` - The S3 object key (path within bucket)
    /// * `content` - File content as bytes
    /// * `content_type` - MIME type (e.g., "text/markdown")
    ///
    /// # Returns
    /// The full S3 key of the uploaded object
    pub async fn upload_file(
        &self,
        key: &str,
        content: &[u8],
        content_type: Option<&str>,
    ) -> Result<String, StorageError> {
        let body = ByteStream::from(content.to_vec());

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        tracing::info!("Uploaded file to S3: {}", key);
        Ok(key.to_string())
    }

    /// Download a file from S3.
    ///
    /// # Arguments
    /// * `key` - The S3 object key
    ///
    /// # Returns
    /// File content as bytes
    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("not found") {
                    StorageError::NotFound(key.to_string())
                } else {
                    StorageError::S3(err_str)
                }
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?
            .into_bytes()
            .to_vec();

        Ok(bytes)
    }

    /// Delete a file from S3.
    ///
    /// # Arguments
    /// * `key` - The S3 object key to delete
    pub async fn delete_file(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        tracing::info!("Deleted file from S3: {}", key);
        Ok(())
    }

    /// List files with a given prefix.
    ///
    /// # Arguments
    /// * `prefix` - The prefix to filter objects by (e.g., "device_id/")
    ///
    /// # Returns
    /// List of object keys matching the prefix
    pub async fn list_files(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        Ok(keys)
    }

    /// Check if a file exists in S3.
    ///
    /// # Arguments
    /// * `key` - The S3 object key
    ///
    /// # Returns
    /// true if the object exists, false otherwise
    pub async fn file_exists(&self, key: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NotFound") || err_str.contains("not found") {
                    Ok(false)
                } else {
                    Err(StorageError::S3(err_str))
                }
            }
        }
    }

    /// Generate the S3 key for a device's file.
    ///
    /// Format: `{device_id}/{file_path}`
    pub fn make_key(device_id: &str, file_path: &str) -> String {
        format!("{}/{}", device_id, file_path.trim_start_matches('/'))
    }
}
