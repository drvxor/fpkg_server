use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use fpkg_server::{handlers, models::{AppState}, global, models};
use rkyv::{access, deserialize, Archived, rancor::Error};
use std::fs;

#[tokio::main]
async fn main() {
    let initial_packages = match fs::read(global::DB_PATH) {
        Ok(bytes) => {
            if bytes.is_empty() {
                Vec::new()
            } else {
                access::<Archived<Vec<models::Package>>, Error>(&bytes)
                    .and_then(|archived| deserialize::<Vec<models::Package>, Error>(archived))
                    .unwrap_or_else(|_| {
                        println!("Warning: Installed packages database was malformed. Starting fresh.");
                        Vec::new()
                    })
            }
        }
        Err(_) => {
            println!("No existing packages database found. Creating a new database state.");
            Vec::new()
        }
    };

    let shared_state = AppState {
        packages: Arc::new(RwLock::new(initial_packages)),
    };

    let app = Router::new()
        .route("/summary", get(handlers::summary))
        .route(
            "/upload",
            post(handlers::upload_package)
                .layer(DefaultBodyLimit::max(4 * 1024 * 1024 * 1024))
        )
        .route("/get/{*capture}", get(handlers::get_file))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(global::URL).await.unwrap();
    println!("Server running on http://{}", global::URL);
    axum::serve(listener, app).await.unwrap();
}