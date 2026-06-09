use axum::extract::Path;
use axum::http::StatusCode;
use tokio::fs;
use axum::extract::State;
use axum::response::IntoResponse;
use crate::models::{AppState, Package, UploadPayload, UpdatePayload};
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
    if safe_filename.ends_with(".tar.zst") {
        headers.insert(header::CONTENT_TYPE, "application/zstd".parse().unwrap());
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

pub async fn checksum(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let packages_guard = state.packages.read().await;

    let bytes = to_bytes::<Error>(&*packages_guard).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize database: {}", e))
    })?;

    let hash = blake3::hash(&bytes);

    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        hash.as_bytes().to_vec(),
    ))
}


pub async fn upload_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    TypedMultipart(form): TypedMultipart<UploadPayload>
) -> Result<impl IntoResponse, (StatusCode, String)> {

    if let Some(auth_token) = headers.get("x-fpkg-upload-token") {
        if auth_token.to_str().unwrap_or("invalid-chars") != state.auth_token {return Ok((StatusCode::FORBIDDEN, "Invalid authentication token!"));}
    }

    tokio::fs::create_dir_all("static").await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create static dir: {}", e))
    })?;

    let file_name = format!("{}-{}.tar.zst", form.name, form.version);

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
        dependencies: form.dependencies,
        source_based: form.source_based,
        binary_based: form.binary_based,
        build_cmd: form.build_cmd,
        manually_installed: false,
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

pub async fn update_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    TypedMultipart(form): TypedMultipart<UpdatePayload>
) -> Result<impl IntoResponse, (StatusCode, String)> {

    if let Some(auth_token) = headers.get("x-fpkg-update-token") {
        if auth_token.to_str().unwrap_or("invalid-chars") != state.auth_token {return Ok((StatusCode::FORBIDDEN, "Invalid authentication token!"));}
    }

    let mut packages_guard = state.packages.write().await;

    let original_package_idx = packages_guard.iter().position(|p| p.name == form.original_name);

    let idx = match original_package_idx {
        Some(index) => index,
        None => return Err((StatusCode::NOT_FOUND, String::from("Package not found!"))),
    };

    let original_package = &packages_guard[idx];

    let name = form.name.unwrap_or_else(|| original_package.name.clone());
    let version = form.version.unwrap_or_else(|| original_package.version.clone());
    let description = form.description.unwrap_or_else(|| original_package.description.clone());

    let file_name = if form.file.is_some() {
        format!("{}-{}.tar.zst", name, version)
    } else {
        original_package.file_name.clone()
    };

    if form.file.is_some() {
        tokio::fs::create_dir_all("static").await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create static dir: {}", e))
        })?;

        let old_path = format!("static/{}", original_package.file_name);
        let _ = tokio::fs::remove_file(old_path).await;

        tokio::fs::copy(form.file.unwrap().contents.path(), &format!("static/{}", file_name))
            .await
            .map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save large file: {}", e))
            })?;
    }

    let new_package_data = Package {
        name,
        file_name,
        version,
        description,
        dependencies: original_package.dependencies.clone(),
        source_based: original_package.source_based,
        binary_based: original_package.binary_based,
        build_cmd: original_package.build_cmd.clone(),
        manually_installed: false,
    };

    packages_guard.remove(idx);
    packages_guard.push(new_package_data);

    let bytes = to_bytes::<Error>(&*packages_guard).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize database: {}", e))
    })?;

    let mut file = fs::File::create("packages.bin").await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open database file: {}", e))
    })?;

    file.write_all(&bytes).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write database file: {}", e))
    })?;

    Ok((StatusCode::OK, "Package updated!"))
}
