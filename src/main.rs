use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use fpkg_server::{handlers, models::{AppState, Package}, global};

#[tokio::main]
async fn main() {
    let initial_packages = match tokio::fs::read_to_string("packages.json").await {
        Ok(json_content) => {
            serde_json::from_str::<Vec<Package>>(&json_content).unwrap_or_else(|_| {
                println!("Warning: packages.json was malformed. Starting fresh.");
                Vec::new()
            })
        }
        Err(_) => {
            println!("No existing packages.json found. Creating a new database state.");
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