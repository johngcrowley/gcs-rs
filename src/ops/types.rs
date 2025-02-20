use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as Map;

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
