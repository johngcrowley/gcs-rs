#![allow(dead_code)]
#![allow(unused)]

use anyhow::{Error, Result};
use futures::stream::Stream;
use futures::StreamExt;
use gcp_auth;
use gcs_rs::cli::parse_args;
use gcs_rs::ops::gcs_bucket::RemoteStorage;
use std::num::NonZero;
use std::pin::pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    //let matches = parse_args();
    //let uri = matches.get_one::<String>("uri");
    //let op = matches.get_one::<String>("op");

    const BUFFER_SIZE: usize = 32 * 1024;

    // Auth
    // https://docs.rs/gcp_auth/latest/src/gcp_auth/custom_service_account.rs.html#130-157
    let provider = gcp_auth::provider().await?;

    let gcs = gcs_rs::ops::gcs_bucket::GCSBucket {
        token_provider: Arc::clone(&provider),
        bucket_name: "https://storage.googleapis.com/storage/v1/b/acrelab-production-us1c-transfer"
            .to_string(),
        prefix_in_bucket: None,
    };

    // --- Bearer Token: ---
    //println!("{:?}", gcs.token_provider.token(SCOPES).await?.as_str());

    // --- Upload: ---
    let source_file =
        tokio::fs::File::open("/home/john/code/rust/gcs-rs/src/tests/nullbytes").await?;
    let fs_size = usize::try_from(source_file.metadata().await?.len())?;
    let gcs_uri = "https://storage.googleapis.com/upload/storage/v1/b/acrelab-production-us1c-transfer/o?uploadType=media&name=nullbytes";
    let reader = tokio_util::io::ReaderStream::with_capacity(source_file, BUFFER_SIZE);
    gcs.upload(reader, fs_size, gcs_uri).await?;

    // --- Download: ---
    let cancel = CancellationToken::new();
    let remote_prefix = "bonk.geojson".to_string();
    use futures::stream::StreamExt;
    let downloads = gcs.download_object(remote_prefix, &cancel).await?;
    let mut stream = std::pin::pin!(downloads.download_stream);
    while let Some(item) = stream.next().await {
        println!("{:?}", item);
    }

    // --- List: ---
    // let cancel = CancellationToken::new();
    let remote_prefix = "box/tiff/2023/TN".to_string();
    let max_keys: u32 = 100000;
    let mut stream = pin!(gcs.list_streaming(Some(remote_prefix), NonZero::new(max_keys)));
    // Return some iterator
    let mut combined = stream.next().await.expect("At least one item required")?;
    while let Some(list) = stream.next().await {
        // The ListingObject vector we return from 'list_streaming()'
        let list = list?;
        combined.keys.extend(list.keys.into_iter());
        combined.prefixes.extend_from_slice(&list.prefixes);
    }
    for key in combined.keys {
        println!("Item: {} -- {:?}", key.key, key.last_modified);
    }

    Ok(())
}
