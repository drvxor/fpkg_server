use axum::extract::Path;
use axum::http::StatusCode;
use tokio::fs;
use axum::{extract::State};
use axum::response::IntoResponse;
use crate::models::{AppState, Package, UploadPayload};
use axum::http::{header, HeaderMap};
use axum_typed_multipart::TypedMultipart;
use tokio::io::AsyncWriteExt;
use rkyv::{to_bytes, rancor::Error};

pub async fn get_file(
    Path(filename): Path<String>
) -> Result<(HeaderMap, Vec<u8>), (StatusCode, String)> {

    let safe_filename = filename.replace("..", "");
    let path = format!("static/{}", safe_filename);

    let bytes = fs::read(path)
        .await
        .map_err(|err| (StatusCode::NOT_FOUND, format!("File not found: {}", err)))?;

    let mut headers = HeaderMap::new();
    if safe_filename.ends_with(".tar.gz") {
        headers.insert(header::CONTENT_TYPE, "application/gzip".parse().unwrap());
    } else if safe_filename.ends_with(".bin") {
        headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    } else {
        headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    }

    Ok((headers, bytes))
}

pub async fn summary(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let packages_guard = state.packages.read().await;

    let bytes = to_bytes::<Error>(&*packages_guard).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize database: {}", e))
    })?;

    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes.to_vec(),
    ))
}

pub async fn upload_package(
    State(state): State<AppState>,
    TypedMultipart(form): TypedMultipart<UploadPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {

    tokio::fs::create_dir_all("static").await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create static dir: {}", e))
    })?;

    let file_name = format!("{}-{}.tar.gz", form.name, form.version);

    tokio::fs::copy(form.file.contents.path(), &format!("static/{}", file_name))
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save large file: {}", e))
        })?;

    let new_package = Package {
        name: form.name,
        file_name,
        version: form.version,
        description: form.description,
    };

    {
        let mut packages_guard = state.packages.write().await;
        packages_guard.push(new_package);

        let bytes = to_bytes::<Error>(&*packages_guard).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize database: {}", e))
        })?;

        let mut file = fs::File::create("packages.bin").await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open database file: {}", e))
        })?;

        file.write_all(&bytes).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write database file: {}", e))
        })?;
    }

    Ok((StatusCode::CREATED, "Package uploaded!"))
}