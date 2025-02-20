#![allow(dead_code)]
#![allow(unused)]

use crate::ops::types;
use anyhow::{Error, Result};
use bytes::Bytes;
use bytes::BytesMut;
use futures::stream::Stream;
use futures::stream::TryStreamExt;
use futures_util::StreamExt;
use gcp_auth::{Token, TokenProvider};
use http::Method;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, FramedRead};

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

pub struct RemoteStorage {
    pub token_provider: Arc<dyn TokenProvider>,
}

impl RemoteStorage {
    // Streaming Upload
    // 1.) Neon's call of .upload():
    // https://github.com/neondatabase/neon/blob/8c6d133d31ced1dc9bba9fc79a9ca2d50c636b66/pageserver/src/tenant/remote_timeline_client/upload.rs#L140C1-L148C81
    // 2.) Neon's .upload impl:
    // https://github.com/neondatabase/neon/blob/8c2f85b20922c9c32d255da6b0b362b7b323eb82/libs/remote_storage/src/s3_bucket.rs#L718C1-L727C21
    // 3.) Which is AWS S3 SDK, takes a byte streams, calls .send():
    // https://docs.rs/aws-sdk-s3/latest/src/aws_sdk_s3/operation/put_object/builders.rs.html#137-156
    // 4.) Which calls .orchestrate():
    // https://docs.rs/aws-sdk-s3/latest/src/aws_sdk_s3/operation/put_object.rs.html#11

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

        // API should really be: "i wasnt able to do it, try again later", since a lot of things could go wrong.
        // - file couldnt be read
        // - socket closed connection

        // Read about Rust and GDB (cli debugger) to avoid VSCode.

        //println!("---- streaming upload -----");
        //println!("Status: {}", res.status());
        //println!("Headers:\n{:#?}", res.headers());

        //let body = res.text().await?;
        //println!("Body:\n{}", body);

        Ok(())
    }

    pub async fn list(&self) -> Result<()> {
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
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
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
}
