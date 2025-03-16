#![allow(dead_code)]
#![allow(unused)]

// ---------------------------------------------------------------------------------
// a cli utility that will try to use my GCS lib as if its trying to use their S3 one
// ---------------------------------------------------------------------------------

use crate::ops::types;
use anyhow::{Error, Result};
use bytes::Bytes;
use bytes::BytesMut;
use chrono::NaiveDateTime;
use futures::stream::Stream;
use futures::stream::TryStreamExt;
use futures_util::StreamExt;
use gcp_auth::{Token, TokenProvider};
use http::Method;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::pin::pin;
use std::sync::Arc;
use std::time::SystemTime;
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::sync::CancellationToken;
use types::{DownloadError, Listing, ListingObject};

const SCOPES: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];

pub struct GCSBucket {
    pub token_provider: Arc<dyn TokenProvider>,
    pub bucket_name: String,
}

impl GCSBucket {
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

    // GCS List Objects
    // - Vec holds Object, here Neon calls '.contents()' to loop throught the Vec of Objects
    // https://github.com/neondatabase/neon/blob/2a5d7e5a78f7d699ee6590220609111bd93b07f6/libs/remote_storage/src/s3_bucket.rs#L568
    pub async fn list_objects(&self, gcs_uri: String) -> Result<types::GCSListResponse> {
        // -----------
        // | Client: |
        // -----------
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

        let res = Client::new()
            .get(gcs_uri)
            .bearer_auth(self.token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        let body = res.text().await?;
        let resp: types::GCSListResponse = serde_json::from_str(&body)?;
        Ok(resp)
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
    // List Streaming -- interface attaches here:
    // https://github.com/neondatabase/neon/blob/main/libs/remote_storage/src/lib.rs#L293
    fn list_streaming(
        &self,
        remote_prefix: Option<String>,
        max_keys: Option<NonZeroU32>,
    ) -> impl Stream<Item = Result<types::Listing, types::DownloadError>> {
        let mut max_keys = max_keys.map(|mk| mk.get() as i32);
        println!("outside of stream");

        // Initial request URI
        let mut gcs_uri = self.bucket_name.clone() + "/o?prefix=" + &remote_prefix.unwrap();

        async_stream::stream! {

            println!("restarting loop");
            let mut continuation_token = None;

            // a loop -- this is how the 'continuation token' button keeps getting hit
            'outer: loop {

                let mut result = types::Listing::default();
                println!(" --- new batch ---");

                // First layer: Get a GCS Response of GCS Objects
                let resp = self.list_objects(gcs_uri.clone()).await?;
                for res in resp.contents() {

                   // Convert 'updated' to SystemTime
                   let last_modified = res.updated.clone().unwrap();
                   //let last_modified = match res.updated.map(SystemTime::try_from) {
                   //    Some(Ok(t)) => t,
                   //    _ => SystemTime::now()
                   //};

                   // Convert 'size' to u64
                   let size = res.size.clone().unwrap_or("0".to_string()).parse::<u64>().unwrap();

                   let key = res.name.clone();

                   // Second layer: for each GCS Object in GCSReponse, pluck out the ingredients to make a
                   // ListingObject and fill up a Listing.
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
                       //println!("updated max_keys to: {:?}", &max_keys);
                   };
                }

                println!("yielding, not max_key limit reached.");
                yield Ok(result);

                continuation_token = match resp.next_page_token {
                    Some(token) => {
                        println!("got a continuation_token!");
                        gcs_uri = gcs_uri + "?pageToken=" + &token;
                        Some(token)
                    },
                    None => break
                }
            }
        }
    }

    // ---------
    // | Neon: |
    // ---------
    // They insist they call 'list' but implement 'list_streaming' when that's called:
    // https://github.com/neondatabase/neon/commit/2c0d311a54927dabea9ae4f97559a0d878f36d9c
    // ---
    // Yes, here they confess that the interface is `GenericRemoteStorage`:
    // https://github.com/neondatabase/neon/blob/main/libs/remote_storage/src/lib.rs#L1C1-L7C79
    // ---
    // And here is the `list()` wrapper around `list_streaming()`:
    // https://github.com/neondatabase/neon/blob/main/libs/remote_storage/src/lib.rs#L286C2-L301C6
    // ---
    // it's `while let Some()-ing` til it gets back a None and tacks on results to the
    // `.keys` attribute of the Type returned by `list_streaming`:  impl Stream<Item = Result<Listing, DownloadError>>
    // ---
    // That type is the Stream<Result<-wrapped "`Listing`" struct defined here:
    // https://github.com/neondatabase/neon/blob/2a5d7e5a78f7d699ee6590220609111bd93b07f6/libs/remote_storage/src/lib.rs#L178C1-L182C2
    // ---
    // All those structs are implemented for me in `lib.rs`  as the Generic interfaces. I just
    // need to make provider-specific calls to be wrapped and try to use those same generic
    // Types.
    // ---
    // https://github.com/neondatabase/neon/blob/8c2f85b20922c9c32d255da6b0b362b7b323eb82/libs/remote_storage/src/s3_bucket.rs#L494C4-L499C36
    // We care about 'key', 'last_modified', and 'size' to load up into our `ListingObject` of
    // Type `Listing` (vec of `ListingObjects`)
    // ---
    // We take in a "mode", "max_keys", "cancel (token)", and "Option<prefix>"

    // source + sink. `cat` is a source.
    //
    // Queue load management in 2 ways: backpressure or consumer drops stuff.
}
