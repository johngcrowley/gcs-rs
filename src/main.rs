#![allow(dead_code)]
#![allow(unused)]

use anyhow::Result;
use http::Method;

pub mod cli {

    use clap::{arg, ArgMatches, Command};

    pub fn parse_args() -> ArgMatches {
        let matches = Command::new("gcs-rs")
            .about("A rust-based, barebones GCS client based on Google Cloud HTTP API")
            .arg(arg!(--op <VALUE>).required(true))
            .arg(arg!(--uri <VALUE>))
            .get_matches();

        println!("Value for uri: {:?}", matches.get_one::<String>("uri"));

        println!(
            "Value for op: {:?}",
            matches.get_one::<String>("op").expect("required")
        );

        matches
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use cli::parse_args;

    let matches = parse_args();
    let uri = matches.get_one::<String>("uri").expect("required");
    let op = matches.get_one::<String>("op").expect("required");

    /// JSON API only supports a bearer token. I think this is only obtained through `gcloud
    /// auth`, e.g.:
    ///
    /// curl -X GET \
    ///   -H "Authorization: Bearer $(gcloud auth print-access-token)" \
    ///   -H "x-goog-user-project: acrelab-production" \
    ///   "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer/o?prefix=bourdain/xray/mortgage/one/cdl/json"
    let gcs_uri: &'static str =
        "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer/o/";

    let provider = gcp_auth::provider().await?;
    let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
    let token = provider.token(scopes).await?;

    let res = reqwest::Client::new()
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
