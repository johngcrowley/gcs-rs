#![allow(dead_code)]
#![allow(unused)]

use crate::ops::types;
use anyhow::{Error, Result};
use azure_core::Etag;
use bytes::Bytes;
use bytes::BytesMut;
use chrono::NaiveDateTime;
use futures::stream::Stream;
use futures::stream::TryStreamExt;
use futures_util::StreamExt;
use gcp_auth::{Token, TokenProvider};
use http::Method;
use http::StatusCode;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::num::NonZeroU32;
use std::pin::{pin, Pin};
use std::sync::Arc;
use url::Url;
//use std::time::SystemTime;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::sync::CancellationToken;
use types::{DownloadError, Listing, ListingObject};

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

pub struct GCSBucket {
    pub token_provider: Arc<dyn TokenProvider>,
    pub bucket_name: String,
}

impl GCSBucket {
    pub async fn upload(
        &self,
        byte_stream: impl Stream<Item = std::io::Result<Bytes>> + Send + Sync + 'static,
        fs_size: usize,
        gcs_uri: &str,
    ) -> Result<()> {
        // https://cloud.google.com/storage/docs/xml-api/reference-headers#chunked
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::TRANSFER_ENCODING,
            header::HeaderValue::from_static("chunked"),
        );

        let res = Client::new()
            .post(gcs_uri)
            // assert that this isn't being read into memory
            .body(reqwest::Body::wrap_stream(byte_stream))
            .headers(headers)
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        println!("Status: {}", res.status());

        Ok(())
    }

    pub async fn list_objects(&self, gcs_uri: String) -> Result<types::GCSListResponse> {
        let res = Client::new()
            .get(gcs_uri)
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        let body = res.text().await?;
        let resp: types::GCSListResponse = serde_json::from_str(&body)?;
        Ok(resp)
    }

    // need a 'bucket', a 'key', and a bytes 'range'.
    pub async fn download_object(&self, key: String) -> Result<Download, DownloadError> {
        // Metadata from body
        let metadata_uri_mod = "alt=json";
        let uri = format!(
            "{}/o/{}?{}",
            self.bucket_name,
            key.replace("/", "%2F"),
            metadata_uri_mod
        );
        let url_encoded = Url::parse(&uri).unwrap();
        println!("{}", url_encoded.as_str());

        let res = Client::new()
            .get(uri)
            .bearer_auth(
                self.token_provider
                    .token(SCOPES)
                    .await
                    .map_err(|e: gcp_auth::Error| DownloadError::Other(e.into()))?
                    .as_str(),
            )
            .send()
            .await
            .map_err(|e: reqwest::Error| DownloadError::Other(e.into()))?;

        let body = res
            .text()
            .await
            .map_err(|e: reqwest::Error| DownloadError::Other(e.into()))?;

        let resp: types::GCSObject = serde_json::from_str(&body)
            .map_err(|e: serde_json::Error| DownloadError::Other(e.into()))?;

        println!("{:?}", resp);

        //let mut metadata = HashMap::new();

        let stream_uri_mod = "alt=media";

        let mut headers = header::HeaderMap::new();
        headers.insert(header::RANGE, header::HeaderValue::from_static("bytes=0-"));

        let uri = format!("{}/o/{}?{}", self.bucket_name, key, stream_uri_mod);
        let url_encoded: String = url::form_urlencoded::byte_serialize(uri.as_bytes()).collect();
        println!("{url_encoded}");

        let mut res = Client::new()
            .get(uri)
            .headers(headers)
            .bearer_auth(
                self.token_provider
                    .token(SCOPES)
                    .await
                    .map_err(|e: gcp_auth::Error| DownloadError::Other(e.into()))?
                    .as_str(),
            )
            .send()
            .await
            .map_err(|e: reqwest::Error| DownloadError::Other(e.into()))?;

        // Eureka:
        // 1. Reqwest is .await-ing on the socket to open. that's it.
        // 2. We check the header status_code to continue or not. we can check the color of water
        //    without having to collect all of it!
        // 3. We then call 'bytes_stream' to get a `Stream`. This is an aynchronous iterator.
        // 4. Each call to it looks like `.next().await` which is what creates the `Future`
        // 5. But! We don't do that here. We do it in the outer functions of Neon that call this
        //    function.
        //    https://github.com/neondatabase/neon/blob/55cb07f680603ff768a3cbe1ff8367a4fe8566e2/libs/remote_storage/src/local_fs.rs#L1194C1-L1203C16
        // 6. We have to apply a mask over our stream with Serde
        // 7. And to return a Stream from a function we need to Pin it in memory.
        // --- Those two requirements are what I need to do-.
        // Notes:
        // - the `tokio::select!` thing in the S3 download function is just a race. It's checking
        //   if the timeout Future finishes first before the request.

        // We serialize headers
        if !res.status().is_success() {
            match res.status() {
                StatusCode::NOT_FOUND => return Err(DownloadError::NotFound),
                _ => {
                    return Err(DownloadError::Other(anyhow::anyhow!(
                        "GCS GET resposne contained no response body"
                    )))
                }
            }
        };

        //let resp: types::GCSObject = serde_json::from_str(fresh_headers)
        //    .map_err(|e: serde_json::Error| DownloadError::Other(e.into()))?;

        //println!("{:?}", resp);

        // But let data stream pass through
        Ok(Download {
            download_stream: Box::pin(res.bytes_stream().map(|item| {
                item.map_err(|e: reqwest::Error| std::io::Error::new(std::io::ErrorKind::Other, e))
            })),
            etag: resp.etag.into(),
            last_modified: resp.updated.unwrap(),
            metadata: Some(StorageMetadata(resp.metadata.unwrap())),
        })
    }
}

struct GetObjectRequest {
    bucket: String,
    key: String,
    etag: Option<String>,
    range: Option<String>,
}

/// Data part of an ongoing [`Download`].
///
/// `DownloadStream` is sensitive to the timeout and cancellation used with the original
/// [`RemoteStorage::download`] request. The type yields `std::io::Result<Bytes>` to be compatible
/// with `tokio::io::copy_buf`.
// This has 'static because safekeepers do not use cancellation tokens (yet)
pub type DownloadStream =
    Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send + Sync + 'static>>;

pub struct Download {
    pub download_stream: DownloadStream,
    /// The last time the file was modified (`last-modified` HTTP header)
    pub last_modified: String,
    /// A way to identify this specific version of the resource (`etag` HTTP header)
    pub etag: Etag,
    /// Extra key-value data, associated with the current remote file.
    pub metadata: Option<StorageMetadata>,
}

impl Debug for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Download")
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Extra set of key-value pairs that contain arbitrary metadata about the storage entry.
/// Immutable, cannot be changed once the file is created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMetadata(HashMap<String, String>);

impl<const N: usize> From<[(&str, &str); N]> for StorageMetadata {
    fn from(arr: [(&str, &str); N]) -> Self {
        let map: HashMap<String, String> = arr
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self(map)
    }
}

#[allow(async_fn_in_trait)]
pub trait RemoteStorage: Send + Sync + 'static {
    fn list_streaming(
        &self,
        prefix: Option<String>,
        max_keys: Option<NonZeroU32>,
    ) -> impl Stream<Item = Result<Listing, DownloadError>> + Send;
}

impl RemoteStorage for GCSBucket {
    fn list_streaming(
        &self,
        remote_prefix: Option<String>,
        max_keys: Option<NonZeroU32>,
    ) -> impl Stream<Item = Result<types::Listing, types::DownloadError>> {
        let mut max_keys = max_keys.map(|mk| mk.get() as i32);
        let mut gcs_uri = self.bucket_name.clone() + "/o?prefix=" + &remote_prefix.unwrap();

        async_stream::stream! {
            let mut continuation_token = None;

            'outer: loop {

                let mut result = types::Listing::default();
                let resp = self.list_objects(gcs_uri.clone()).await?;
                for res in resp.contents() {

                   let last_modified = res.updated.clone().unwrap();
                   let size = res.size.clone().unwrap_or("0".to_string()).parse::<u64>().unwrap();
                   let key = res.name.clone();
                   result.keys.push(
                        types::ListingObject{
                            key,
                            last_modified,
                            size,
                        }
                   );
                   // TODO: when fork, write unit test to expose this bug.
                   // this never gets hit, because `max_keys`  is always `None`
                   // we take min of:
                   // https://github.com/neondatabase/neon/blob/main/libs/remote_storage/src/lib.rs#L71
                   // or
                   // https://github.com/search?q=repo%3Aneondatabase%2Fneon%20%22.list_streaming%22&type=code
                   // point being: every call to `.list_streaming()` sets `max_keys` to `None`.
                   if let Some(mut mk) = max_keys {
                       assert!(mk > 0);
                       mk -= 1;
                       if mk == 0 {
                          println!("limit reached set by max_keys");
                          yield Ok(result);
                          break 'outer;
                       }
                       max_keys = Some(mk);
                   };
                }

                yield Ok(result);

                continuation_token = match resp.next_page_token {
                    Some(token) => {
                        gcs_uri = gcs_uri + "?pageToken=" + &token;
                        Some(token)
                    },
                    None => break
                }
            }
        }
    }
}
