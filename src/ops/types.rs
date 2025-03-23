use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct GCSListResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub items: Option<Vec<GCSObject>>,
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
    pub size: Option<String>,
    #[serde(rename = "md5Hash")]
    pub md5_hash: Option<String>,
    pub crc32c: String,
    pub etag: String,
    #[serde(rename = "timeCreated")]
    pub time_created: String,
    pub updated: Option<String>,
    #[serde(rename = "timeStorageClassUpdated")]
    pub time_storage_class_updated: String,
    #[serde(rename = "timeFinalized")]
    pub time_finalized: String,
    pub metadata: Option<HashMap<String, String>>,
}

impl GCSListResponse {
    pub fn contents(&self) -> &[GCSObject] {
        self.items.as_deref().unwrap_or_default()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ListingObject {
    pub key: String,
    pub last_modified: SystemTime,
    pub size: u64,
}

#[derive(Default)]
pub struct Listing {
    pub prefixes: Vec<String>,
    pub keys: Vec<ListingObject>,
}

/// Reasons for downloads or listings to fail.
#[derive(Debug)]
pub enum DownloadError {
    /// Validation or other error happened due to user input.
    BadInput(anyhow::Error),
    /// The file was not found in the remote storage.
    NotFound,
    /// The caller provided an ETag, and the file was not modified.
    Unmodified,
    /// A cancellation token aborted the download, typically during
    /// tenant detach or process shutdown.
    Cancelled,
    /// A timeout happened while executing the request. Possible reasons:
    /// - stuck tcp connection
    ///
    /// Concurrency control is not timed within timeout.
    Timeout,
    /// Some integrity/consistency check failed during download. This is used during
    /// timeline loads to cancel the load of a tenant if some timeline detects fatal corruption.
    Fatal(String),
    /// The file was found in the remote storage, but the download failed.
    Other(anyhow::Error),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::BadInput(e) => {
                write!(f, "Failed to download a remote file due to user input: {e}")
            }
            DownloadError::NotFound => write!(f, "No file found for the remote object id given"),
            DownloadError::Unmodified => write!(f, "File was not modified"),
            DownloadError::Cancelled => write!(f, "Cancelled, shutting down"),
            DownloadError::Timeout => write!(f, "timeout"),
            DownloadError::Fatal(why) => write!(f, "Fatal read error: {why}"),
            DownloadError::Other(e) => write!(f, "Failed to download a remote file: {e:?}"),
        }
    }
}

impl From<anyhow::Error> for DownloadError {
    fn from(error: anyhow::Error) -> Self {
        DownloadError::Other(error)
    }
}

impl std::error::Error for DownloadError {}
