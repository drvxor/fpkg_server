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
    pub dependencies: Vec<String>,
    pub source_based: bool,
    pub binary_based: bool,
    pub build_cmd: Option<String>
}

#[derive(TryFromMultipart)]
pub struct UpdatePayload {
    pub original_name: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub file: Option<FieldData<NamedTempFile>>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub file_name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub source_based: bool,
    pub binary_based: bool,
    pub build_cmd: Option<String>,
    pub manually_installed: bool
}

#[derive(Clone)]
pub struct AppState {
    pub packages: Arc<RwLock<Vec<Package>>>,
}