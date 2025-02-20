#![allow(dead_code)]
#![allow(unused)]

use anyhow::{Error, Result};
use gcp_auth;
use gcs_rs::cli::parse_args;
use gcs_rs::ops::gcs_bucket::RemoteStorage;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    //let matches = parse_args();
    //let uri = matches.get_one::<String>("uri");
    //let op = matches.get_one::<String>("op");

    const BUFFER_SIZE: usize = 32 * 1024;

    // Auth
    // https://docs.rs/gcp_auth/latest/src/gcp_auth/custom_service_account.rs.html#130-157
    let provider = gcp_auth::provider().await?;

    //TODO: error handling
    let source_file = tokio::fs::File::open("./src/nullbytes").await?;

    let fs_size = usize::try_from(source_file.metadata().await?.len())?;

    let gcs = gcs_rs::ops::gcs_bucket::RemoteStorage {
        token_provider: Arc::clone(&provider),
    };

    let gcs_uri = "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=nullbytes";

    let reader = tokio_util::io::ReaderStream::with_capacity(source_file, BUFFER_SIZE);

    gcs.upload(reader, fs_size, gcs_uri).await?;

    Ok(())
}
