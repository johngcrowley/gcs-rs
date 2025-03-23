#![allow(dead_code)]
#![allow(unused)]

use crate::ops::types;
use anyhow::{Error, Result};
use azure_core::Etag;
use bytes::Bytes;
use bytes::BytesMut;
use chrono::DateTime;
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
use std::time::SystemTime;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::sync::CancellationToken;
use types::{DownloadError, Listing, ListingObject};
use url::Url;
use uuid::Uuid;

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

pub struct GCSBucket {
    pub token_provider: Arc<dyn TokenProvider>,
    pub bucket_name: String,
    pub prefix_in_bucket: Option<String>,
    //max_keys_per_list_response: Option<i32>,
    //pub timeout: std::time::Duration,
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
            .body(reqwest::Body::wrap_stream(byte_stream))
            .headers(headers)
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        Ok(())
    }

    pub async fn copy(&self, from: String, to: String, cancel: &CancellationToken) -> Result<()> {
        let bucket: &str = "acrelab-production-us1c-transfer";
        let copy_uri = self.bucket_name.clone()
            + "/"
            + bucket
            + "/o/"
            + &from
            + "/copyTo/b/"
            + bucket
            + "/o/"
            + &to;
        let res = Client::new().post(copy_uri);

        Ok(())
    }

    pub async fn delete_objects(&self, paths: &[&str], cancel: &CancellationToken) -> Result<()> {
        let mut delete_objects = Vec::with_capacity(paths.len());

        let mut cancel = std::pin::pin!(cancel.cancelled());

        for path in paths {
            let encoded_path: String =
                url::form_urlencoded::byte_serialize(path.as_bytes()).collect();
            delete_objects.push(encoded_path);
            //delete_objects.push(match &self.prefix_in_bucket {
            //    Some(prefix) => self.bucket_name.clone() + &prefix.clone() + "/" + path,
            //    None => self.bucket_name.clone() + "/" + path,
            //});
        }

        let mut form = reqwest::multipart::Form::new();
        let bulk_uri = "https://storage.googleapis.com/batch/storage/v1";

        let mut logger = HashMap::new();

        for (index, path_to_delete) in delete_objects.iter().enumerate() {
            logger.insert(index + 1, false);

            let delete_req = format!(
                "
                DELETE /storage/v1/b/acrelab-production-us1c-transfer/o/{} HTTP/1.1\r\n\
                Content-Type: application/json\r\n\
                accept: application/json\r\n\
                content-length: 0\r\n
                ",
                path_to_delete
            )
            .trim()
            .to_string();

            println!(
                "Trying to delete: {} in bucket: {}",
                path_to_delete, self.bucket_name
            );

            //println!("Delete request: \n{}", delete_req);

            let content_id = format!("<{}+{}>", Uuid::new_v4(), index + 1);

            let mut part_headers = header::HeaderMap::new();
            part_headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/http"),
            );
            part_headers.insert(
                header::TRANSFER_ENCODING,
                header::HeaderValue::from_static("binary"),
            );
            part_headers.insert(
                header::HeaderName::from_static("content-id"),
                header::HeaderValue::from_str(&content_id)?,
            );
            let part = reqwest::multipart::Part::text(delete_req).headers(part_headers);

            form = form.part(format!("request-{}", index), part);
        }

        println!("{:?}", form.boundary());

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_str(&format!(
                "multipart/mixed; boundary={}",
                form.boundary()
            ))?,
        );

        let res = Client::new()
            .post(bulk_uri)
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
            .multipart(form)
            .headers(headers)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("stuff happened"));
        }

        // I have to do this "owning" first
        // https://users.rust-lang.org/t/creates-a-temporary-which-is-freed-while-still-in-use-again/29211/2
        // -> else, i'm creating a heap object then returning a referene to it with `to_str` or
        //   `get`, before it's tethered. Like I've built a home without a foundation. It gets
        //   washed away into the sea...
        let res_headers = res.headers().to_owned();

        let boundary = res_headers
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()?
            .split("=")
            .last()
            .unwrap();

        println!("{}", boundary);

        println!("---");

        let res_body = res.text().await?;

        let parsed: HashMap<String, String> = res_body
            .split(&format!("--{}", boundary))
            // only keep these things, and do stuff to them on the way.
            .filter_map(|c| {
                // mutable because 'find_map' requires mutable self.
                let mut lines = c.lines();

                let id = lines
                    // .find() is: look for this static pattern and give me Some(index) or None.
                    // .find_map():
                    //      1. do all this stuff
                    //      2. return the first non-None result
                    .find_map(|line| {
                        line
                            // may give me all Nones, else, Some(everything-after-prefix)
                            .strip_prefix("Content-ID:")
                            // takes the Option<> and returns a new Some(),
                            // or stops at the None it gets.
                            .and_then(|suf| suf.split('+').last())
                            // return a new Some()
                            .and_then(|suf| suf.split('>').next())
                            // trim() and to_string() can't fail (don't return Option<>)
                            // so might as well use it.
                            // -> we could also return
                            .map(|x| x.trim().to_string())
                    });

                let status_code = lines.find_map(|line| {
                    line.strip_prefix("HTTP/1.1")
                        .and_then(|x| x.split_whitespace().next())
                        .map(|x| x.trim().to_string())
                });

                id.zip(status_code)
            })
            .collect();

        for (id, code) in parsed.iter() {
            println!("content-id: {}: status_code: {}", id, code);
            println!("---");
        }

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
    pub async fn download_object(
        &self,
        key: String,
        cancel: &CancellationToken,
    ) -> Result<Download, DownloadError> {
        // Serialize Metadata in initial request
        let metadata_uri_mod = "alt=json";
        let uri = format!(
            "{}/o/{}?{}",
            self.bucket_name,
            key.replace("/", "%2F"),
            metadata_uri_mod
        );
        let url_encoded: String = url::form_urlencoded::byte_serialize(uri.as_bytes()).collect();

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

        let body = res
            .text()
            .await
            .map_err(|e: reqwest::Error| DownloadError::Other(e.into()))?;

        let resp: types::GCSObject = serde_json::from_str(&body)
            .map_err(|e: serde_json::Error| DownloadError::Other(e.into()))?;

        // Byte Stream request
        let stream_uri_mod = "alt=media";
        let mut headers = header::HeaderMap::new();
        headers.insert(header::RANGE, header::HeaderValue::from_static("bytes=0-"));
        let uri = format!("{}/o/{}?{}", self.bucket_name, key, stream_uri_mod);
        let url_encoded: String = url::form_urlencoded::byte_serialize(uri.as_bytes()).collect();

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

        //let object_output = tokio::select! {
        //    res = get_object => res,
        //    //_ = tokio::time::sleep(self.timeout) => return Err(DownloadError::Timeout),
        //    _ = cancel.cancelled() => return Err(DownloadError::Cancelled),
        //};

        let metadata = resp.metadata.map(StorageMetadata);

        // How does "into()" really work?
        let last_modified: SystemTime = resp
            .updated
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|s| s.into())
            .unwrap_or(SystemTime::now());

        // But let data stream pass through
        Ok(Download {
            download_stream: Box::pin(res.bytes_stream().map(|item| {
                item.map_err(|e: reqwest::Error| std::io::Error::new(std::io::ErrorKind::Other, e))
            })),
            etag: resp.etag.into(),
            last_modified,
            metadata,
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
    pub last_modified: SystemTime,
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

                   let last_modified: SystemTime = res.updated.clone()
                       .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                       .map(|s| s.into())
                       .unwrap_or(SystemTime::now());

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

#[cfg(test)]
mod tests {

    use super::*;
    use gcp_auth;
    use std::num::NonZero;
    use std::pin::pin;
    use std::sync::Arc;

    const BUFFER_SIZE: usize = 32 * 1024;
    const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];
    const BUCKET: &str =
        "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer";

    // ---

    #[tokio::test]
    async fn list_returns_keys_from_bucket() {
        let provider = gcp_auth::provider().await.unwrap();
        let gcs = GCSBucket {
            token_provider: Arc::clone(&provider),
            bucket_name: BUCKET.to_string(),
            prefix_in_bucket: None,
        };

        // --- List: ---
        let cancel = CancellationToken::new();
        let remote_prefix = "box/tiff/2023/TN".to_string();
        let max_keys: u32 = 100;
        let mut stream = pin!(gcs.list_streaming(Some(remote_prefix), NonZero::new(max_keys)));
        // Return some iterator
        let mut combined = stream
            .next()
            .await
            .expect("At least one item required")
            .unwrap();
        while let Some(list) = stream.next().await {
            // The ListingObject vector we return from 'list_streaming()'
            let list = list.unwrap();
            combined.keys.extend(list.keys.into_iter());
            combined.prefixes.extend_from_slice(&list.prefixes);
        }

        for key in combined.keys.iter() {
            println!("Item: {} -- {:?}", key.key, key.last_modified);
        }

        assert_ne!(0, combined.keys.len());
    }
}
