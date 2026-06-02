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
        dependencies: form.dependencies,
        source_based: form.source_based,
        binary_based: form.binary_based,
        build_cmd: form.build_cmd,
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

// pub async fn update_package(
//     State(state): State<AppState>,
//     TypedMultipart(form): TypedMultipart<UpdatePayload>,
// ) -> Result<impl IntoResponse, (StatusCode, String)> {
//
//     let packages = match fs::read(global::DB_PATH).await {
//         Ok(bytes) => {
//             if bytes.is_empty() {
//                 Vec::new()
//             } else {
//                 access::<Archived<Vec<models::Package>>, Error>(&bytes)
//                     .and_then(|archived| deserialize::<Vec<models::Package>, Error>(archived))
//                     .unwrap_or_else(|_| {
//                         println!("Warning: Installed packages database was malformed. Starting fresh.");
//                         Vec::new()
//                     })
//             }
//         }
//         Err(_) => {
//             Vec::new()
//         }
//     };
//
//     let original_package_raw = packages.iter().find(|p| p.name == form.original_name);
//     if !original_package_raw.is_some(){
//         return Err((StatusCode::NOT_FOUND, String::from("Package not found!")));
//     }
//     let original_package = original_package_raw.unwrap().clone();
//
//     let name = if form.name.is_some() { form.name.unwrap() } else { original_package.name };
//     let version = if form.version.is_some() { form.version.unwrap() } else { original_package.version };
//     let description = if form.description.is_some() { form.description.unwrap() } else { original_package.description };
//     let file_name = if form.file.is_some() {format!("{}-{}.tar.gz", name, version)} else { original_package.file_name.clone() };
//
//     if form.file.is_some() {
//         tokio::fs::create_dir_all("static").await.map_err(|e| {
//             (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create static dir: {}", e))
//         })?;
//
//         tokio::fs::remove_file(original_package.file_name).await.map_err(|_e| println!("{}", _e)).unwrap();
//
//         tokio::fs::copy(form.file.unwrap().contents.path(), &format!("static/{}", file_name))
//             .await
//             .map_err(|e| {
//                 (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save large file: {}", e))
//             })?;
//     }
//
//     let new_package_data = Package {
//         name,
//         file_name,
//         version,
//         description,
//     };
//
//     {
//         let mut packages_guard = state.packages.write().await;
//         packages_guard.retain(|p| p.name != new_package_data.file_name);
//         packages_guard.push(new_package_data);
//
//         let bytes = to_bytes::<Error>(&*packages_guard).map_err(|e| {
//             (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize database: {}", e))
//         })?;
//
//         let mut file = fs::File::create("packages.bin").await.map_err(|e| {
//             (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open database file: {}", e))
//         })?;
//
//         file.write_all(&bytes).await.map_err(|e| {
//             (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write database file: {}", e))
//         })?;
//     }
//
//     Ok((StatusCode::CREATED, "Package updated!"))
// }