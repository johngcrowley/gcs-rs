#![allow(dead_code)]
#![allow(unused)]

use anyhow::{Error, Result};
use gcp_auth::{Token, TokenProvider};
use http::Method;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as Map;
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, FramedRead};

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

pub mod cli {

    use clap::{arg, ArgMatches, Command};

    pub fn parse_args() -> ArgMatches {
        let matches = Command::new("gcs-rs")
            .about("A rust-based, barebones GCS client based on Google Cloud HTTP API")
            .arg(arg!(--op <VALUE>))
            .arg(arg!(--uri <VALUE>))
            .get_matches();

        //println!("Value for uri: {:?}", matches.get_one::<String>("uri"));

        matches
    }
}

pub mod ops {

    use super::*;

    pub async fn list(token_provider: Arc<dyn TokenProvider>) -> Result<()> {
        // AWS S3 SDK for Rust has a ListObjectsV2 call on the Client.
        // It returns a ListObjectsV2FluentBuilder, which has `.send()` impld on it.
        // That `.send()` returns a Result<ListObjectsV2Output, SDKError>
        // the ListObjectsV2Output is a struct like my `GCSListResponse` that parses the fields of
        // the response.
        // https://github.com/awslabs/aws-sdk-rust/blob/main/sdk/s3/src/operation/list_objects_v2/_list_objects_v2_output.rs#L5
        // ---
        // Here is how the `client.list_objects_v2().send()` gets used (records are fetched):
        // https://github.com/awslabs/aws-sdk-rust/blob/main/examples/examples/s3/src/bin/list-objects.rs#L27
        // ---
        // `res.contents()` is an iterator (Vec) of Option<Object> and Object has a `.key()`.
        // ---
        // So my GCSListReponse should be the parent Vec, and each item in it the
        // ListResponseObject, which should impl an interface similar to that.

        let gcs_uri: &'static str =
            "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer/o?prefix=bourdain/xray/parcel/one/cdl/json";

        let res = Client::new()
            .get(gcs_uri)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        //println!("Status: {}", res.status());
        //println!("Headers:\n{:#?}", res.headers());
        let body = res.text().await?;
        println!("Body:\n{}", body);

        let resp: types::GCSListResponse = serde_json::from_str(&body)?;

        for res in resp.contents() {
            println!("{:?}", res);
        }

        Ok(())
    }

    pub async fn upload(token_provider: Arc<dyn TokenProvider>, file: String) -> Result<()> {
        // upload foo.txt for now
        let gcs_uri: &'static str =
            "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=foo.txt";

        // https://cloud.google.com/storage/docs/xml-api/reference-headers#chunked
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::TRANSFER_ENCODING,
            header::HeaderValue::from_static("chunked"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        // read from file system into Stream of bytes
        // ---
        // https://docs.rs/tokio-util/latest/tokio_util/codec/struct.BytesCodec.html
        // ---
        // "codec" = portmaneau of coder/decoder. handles a data stream.
        let async_read = tokio::fs::File::open("./foo.txt").await?;
        let stream = FramedRead::new(async_read, BytesCodec::new());

        // https://docs.rs/tokio-util/latest/tokio_util/codec/index.html
        // https://cloud.google.com/storage/docs/uploading-objects
        // https://docs.rs/reqwest/latest/reqwest/struct.Body.html#method.wrap_stream
        // https://gist.github.com/Ciantic/aa97c7a72f8356d7980756c819563566
        let res = Client::new()
            .post(gcs_uri)
            .body(reqwest::Body::wrap_stream(stream))
            .headers(headers)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        println!("---- streaming upload -----");
        println!("Status: {}", res.status());
        println!("Headers:\n{:#?}", res.headers());

        let body = res.text().await?;
        println!("Body:\n{}", body);

        Ok(())
    }

    pub mod types {

        use super::*;

        #[derive(Serialize, Deserialize, Debug)]
        pub struct GCSListResponse {
            items: Option<Vec<GCSObject>>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "snake_case")]
        pub struct GCSObject {
            name: String,
            bucket: String,
            generation: String,
            metageneration: String,
            #[serde(rename = "contentType")]
            content_type: String,
            #[serde(rename = "storageClass")]
            storage_class: String,
            size: String,
            #[serde(rename = "md5Hash")]
            md5_hash: String,
            crc32c: String,
            etag: String,
            #[serde(rename = "timeCreated")]
            time_created: String,
            updated: String,
            #[serde(rename = "timeStorageClassUpdated")]
            time_storage_class_updated: String,
            #[serde(rename = "timeFinalized")]
            time_finalized: String,
            metadata: Map<String, String>,
        }

        impl GCSListResponse {
            pub fn contents(&self) -> &[GCSObject] {
                self.items.as_deref().unwrap_or_default()
            }
        }
    }

    #[cfg(test)]
    mod tests {

        use super::*;
        //use gcp_auth;

        #[tokio::test]
        async fn list_object_returns_paths() {
            let provider = gcp_auth::provider().await.unwrap();
            list(Arc::clone(&provider)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use cli::parse_args;
    use ops::{list, upload};

    // TODO  CLI Arguments
    let matches = parse_args();
    let uri = matches.get_one::<String>("uri");
    let op = matches.get_one::<String>("op");

    // Auth
    let provider = gcp_auth::provider().await?;

    // -- Print bearer token to stdout --
    //println!("{}", token.as_str());

    // -- List bucket --
    //list(Arc::clone(&provider)).await?;

    // -- Upload object --
    upload(Arc::clone(&provider), "./foo.txt".to_owned()).await?;

    // -- Resumably Upload objects --
    //upload(Arc::clone(&provider), "./foo.txt".to_owned()).await?;

    Ok(())
}
