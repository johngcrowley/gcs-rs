#![allow(dead_code)]
#![allow(unused)]

use anyhow::{Error, Result};
use gcp_auth::{Token, TokenProvider};
use http::Method;
use reqwest::{header, Client};
use std::sync::Arc;

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

    pub async fn list(gcs_uri: &'static str, token: Arc<Token>) -> Result<()> {
        let res = Client::new()
            .get(gcs_uri)
            .bearer_auth(token.as_str())
            .send()
            .await?;

        println!("Status: {}", res.status());
        println!("Headers:\n{:#?}", res.headers());

        let body = res.text().await?;
        println!("Body:\n{}", body);

        Ok(())
    }

    pub async fn upload(token: Arc<Token>, file: String) -> Result<()> {
        let gcs_uri: &'static str =
            "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=foo.txt";

        let data_binary = std::fs::read(file).expect("File path doesn't exist");

        let res = Client::new()
            .post(gcs_uri)
            .body(data_binary)
            .bearer_auth(token.as_str())
            .send()
            .await?;

        println!("Status: {}", res.status());
        println!("Headers:\n{:#?}", res.headers());

        let body = res.text().await?;
        println!("Body:\n{}", body);

        Ok(())
    }

    // TODO
    /// Currently, I have to do dynamic dispatch because TokenProvider is a trait, not a concrete type.
    ///
    /// But, I'm using
    /// [CustomServiceAccount](https://docs.rs/gcp_auth/latest/src/gcp_auth/custom_service_account.rs.html#130)
    /// concrete type (from `gcp_auth` crate), so i don't really need dynamic dispatch.
    ///
    /// I also don't have to worry about code bloat with monomorphization because I'm only ever going to
    /// use 'CustomServiceAccount' concrete type, so I can choose to use Generics here as my
    /// solution.
    ///
    /// Instead of passing around an Arc< dyn ...>, I can make a struct like
    ///
    /// struct HttpClient<T: TokenProvider> {
    ///     token_provider: Arc<T>
    /// }
    ///
    /// and then
    ///
    /// impl<T: TokenProvider> for HttpClient<T> {
    ///     fn upload {}
    ///     fn download {}
    /// }
    ///
    /// etc, where that will compile down to just 'CustomServiceAccount'. Now, I'm not getting a
    /// run-time cost, and I'm getting to pass around ownership, and I'm getting to call my
    /// `.token()` method for refresh logic.
    ///

    pub async fn get_resumable_upload_uri(
        token_provider: Arc<dyn TokenProvider>,
        file: String,
    ) -> Result<header::HeaderValue, anyhow::Error> {
        let gcs_uri: &'static str =
            "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=resumable&name=foo.txt";

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::CONTENT_LENGTH,
            header::HeaderValue::from_static("0"),
        );

        let res = Client::new()
            .post(gcs_uri)
            .headers(headers)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        let uri = res
            .headers()
            .get("location")
            .ok_or_else(|| anyhow::anyhow!("Missing 'location' in header"))?
            .clone();

        Ok(uri)
    }

    pub async fn resumable_upload(
        token_provider: Arc<dyn TokenProvider>,
        file: String,
    ) -> Result<()> {
        // -- Get URI for resumable uploads --
        ///
        // Interesting error I had gotten when method-chaining `to_str()` to be a one-liner:
        //----------------------------------------------------------------------------------------------
        //                    "temporary value dropped while borrowed"
        //----------------------------------------------------------------------------------------------
        // The owned value of type `HeaderValue` which returns from `get_resumable_upload_uri()`
        // never made a pointer back to this stack frame (function). It's arm was groping from the
        // pit, but fell, limply. The `to_str()` turned it's owned value return into a reference,
        // which was dangling back to the stack frame it just exited. Thus, I must `let uri` BE
        // `uri`. Then, from this stack frame, I may play with it.
        let uri = get_resumable_upload_uri(Arc::clone(&token_provider), file.clone()).await?;

        // PUT Chunk
        let data_binary = std::fs::read(&file).expect("File path doesn't exist");
        let data_bytes = std::fs::metadata(&file)
            .expect("File path doesn't exist")
            .len();

        let res = Client::new()
            .put(uri.to_str()?)
            .body(data_binary)
            .header(header::CONTENT_LENGTH, data_bytes)
            .bearer_auth(token_provider.token(SCOPES).await?.as_str())
            .send()
            .await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use cli::parse_args;
    use ops::{get_resumable_upload_uri, list, resumable_upload, upload};

    // TODO  CLI Arguments
    let matches = parse_args();
    let uri = matches.get_one::<String>("uri");
    let op = matches.get_one::<String>("op");

    // Auth
    let provider = gcp_auth::provider().await?;

    // -- Print bearer token to stdout --
    //println!("{}", token.as_str());

    // -- List bucket --
    //list(Arc::clone(&token)).await?;

    // -- Upload object --
    //upload(Arc::clone(&token), "./foo.txt".to_owned()).await?;

    // -- Resumably Upload objects --
    resumable_upload(Arc::clone(&provider), "./foo.txt".to_owned()).await?;

    Ok(())
}
