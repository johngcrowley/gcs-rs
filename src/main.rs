#![allow(dead_code)]
#![allow(unused)]

use anyhow::{Error, Result};
use gcp_auth::Token;
use http::Method;
use reqwest::{header, Client};
use std::sync::Arc;

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

    pub async fn get_resumable_upload_uri(
        token: Arc<Token>,
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
            .bearer_auth(token.as_str())
            .send()
            .await?;

        let uri = res
            .headers()
            .get("location")
            .ok_or_else(|| anyhow::anyhow!("Missing 'location' in header"))?
            .clone();

        Ok(uri)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use cli::parse_args;
    use ops::{get_resumable_upload_uri, list, upload};

    // #TODO  CLI Arguments
    let matches = parse_args();
    let uri = matches.get_one::<String>("uri");
    let op = matches.get_one::<String>("op");

    // Auth
    let provider = gcp_auth::provider().await?;
    let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
    let token = provider.token(scopes).await?;

    // -- Print bearer token to stdout --
    //println!("{}", token.as_str());

    // -- List bucket --
    //list(Arc::clone(&token)).await?;

    // -- Upload object --
    //upload(Arc::clone(&token), "./foo.txt".to_owned()).await?;

    // -- Get URI for resumable uploads --
    let uri = get_resumable_upload_uri(Arc::clone(&token), "./foo.txt".to_owned()).await?;
    println!("{:?}", uri);

    Ok(())
}
