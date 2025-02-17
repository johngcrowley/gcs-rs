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

    use std::io::BufReader;

    use super::*;
    // https://gist.github.com/DmitrySoshnikov/2027a83bab8f00196d2ec295db1a40a8
    pub async fn list(token_provider: Arc<dyn TokenProvider>) -> Result<()> {
        // Client:
        // ----------------------------------
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
        //
        // Neon:
        // ----------------------------------
        // https://github.com/neondatabase/neon/blob/8c2f85b20922c9c32d255da6b0b362b7b323eb82/libs/remote_storage/src/s3_bucket.rs#L494C4-L499C36
        // We care about 'key', 'last_modified', and 'size'
        // ---
        // We take in a "mode", "max_keys", "cancel (token)", and "Option<prefix>"

        let gcs_uri: &'static str =
            "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer/o?prefix=bourdain/xray/parcel/one/cdl/json";

        let res = Client::new()
            .get(gcs_uri)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        let body = res.text().await?;
        //println!("Body:\n{}", body);

        let resp: types::GCSListResponse = serde_json::from_str(&body)?;

        for res in resp.contents() {
            println!("{}", res.name);
        }

        Ok(())
    }

    // Streaming Upload
    //
    // 1.) Neon's call of .upload():
    // https://github.com/neondatabase/neon/blob/8c6d133d31ced1dc9bba9fc79a9ca2d50c636b66/pageserver/src/tenant/remote_timeline_client/upload.rs#L140C1-L148C81
    //
    // 2.) Neon's .upload impl:
    // https://github.com/neondatabase/neon/blob/8c2f85b20922c9c32d255da6b0b362b7b323eb82/libs/remote_storage/src/s3_bucket.rs#L718C1-L727C21
    //
    // 3.) Which is AWS S3 SDK, takes a byte streams, calls .send():
    // https://docs.rs/aws-sdk-s3/latest/src/aws_sdk_s3/operation/put_object/builders.rs.html#137-156
    //
    // 4.) Which calls .orchestrate():
    // https://docs.rs/aws-sdk-s3/latest/src/aws_sdk_s3/operation/put_object.rs.html#11
    //
    //
    //
    //
    //
    // Great goby: https://imfeld.dev/writing/rust_s3_streaming_upload
    // Algorithtim:
    // -
    pub async fn upload(token_provider: Arc<dyn TokenProvider>, file: String) -> Result<()> {
        let upload_uri_base: String =
            "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=".to_owned();

        let gcs_uri: String = upload_uri_base + &file;

        // https://cloud.google.com/storage/docs/xml-api/reference-headers#chunked
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::TRANSFER_ENCODING,
            header::HeaderValue::from_static("chunked"),
        );
        // read from file system into Stream of bytes
        // ---
        // https://docs.rs/tokio-util/latest/tokio_util/codec/struct.BytesCodec.html
        // ---
        // "codec" = portmaneau of coder/decoder. handles a data stream.
        let async_read = tokio::fs::File::open(file.as_str()).await?;
        let buf_reader = tokio::io::BufReader::with_capacity(1024 * 1024, async_read);
        let stream = FramedRead::new(buf_reader, BytesCodec::new());
        // https://docs.rs/tokio-util/latest/tokio_util/codec/index.html
        // https://cloud.google.com/storage/docs/uploading-objects
        // https://docs.rs/reqwest/latest/reqwest/struct.Body.html#method.wrap_stream
        // https://gist.github.com/Ciantic/aa97c7a72f8356d7980756c819563566
        let res = Client::new()
            .post(gcs_uri)
            // assert that this isn't being read into memory
            .body(reqwest::Body::wrap_stream(stream))
            .headers(headers)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        // API should really be: "i wasnt able to do it, try again later", since a lot of things could go wrong.
        // - file couldnt be read
        // - socket closed connection

        // Try sending 8GiB file and see if it reads it all into memory.

        // Read about Rust and GDB (cli debugger) to avoid VSCode.

        //println!("---- streaming upload -----");
        //println!("Status: {}", res.status());
        //println!("Headers:\n{:#?}", res.headers());

        //let body = res.text().await?;
        //println!("Body:\n{}", body);

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
            pub name: String,
            pub bucket: String,
            pub generation: String,
            pub metageneration: String,
            #[serde(rename = "contentType")]
            pub content_type: String,
            #[serde(rename = "storageClass")]
            pub storage_class: String,
            pub size: String,
            #[serde(rename = "md5Hash")]
            pub md5_hash: String,
            pub crc32c: String,
            pub etag: String,
            #[serde(rename = "timeCreated")]
            pub time_created: String,
            pub updated: String,
            #[serde(rename = "timeStorageClassUpdated")]
            pub time_storage_class_updated: String,
            #[serde(rename = "timeFinalized")]
            pub time_finalized: String,
            pub metadata: Map<String, String>,
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
    upload(Arc::clone(&provider), "./foo.jsonl".to_owned()).await?;

    Ok(())
}
