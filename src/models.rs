use axum_typed_multipart::{FieldData, TryFromMultipart};
use tempfile::NamedTempFile;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(TryFromMultipart)]
pub struct UploadPayload {
    pub name: String,
    pub version: String,
    pub description: String,
    pub file: FieldData<NamedTempFile>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub file_name: String,
    pub version: String,
    pub description: String,
}

#[derive(Clone)]
pub struct AppState {
    pub packages: Arc<RwLock<Vec<Package>>>,
}