use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use clap::{Parser};
use std::sync::Arc;
use tokio::sync::RwLock;
use fpkg_server::{handlers, models::{AppState}, global, models};
use rkyv::{access, deserialize, Archived, rancor::Error};
use std::fs;

#[derive(Parser)]
#[command(name = "fpkg_server")]
#[command(about = "Fast Package Server", long_about = None)]
struct Cli {
    #[arg(short, long)]
    url: Option<String>,

    #[arg(short, long)]
    token: Option<String>
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

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

    let url = cli.url.unwrap_or_else(|| String::from("localhost:8080"));

    let shared_state = AppState {
        packages: Arc::new(RwLock::new(initial_packages)),
        auth_token: cli.token.unwrap_or_else(|| String::from("12345678"))
    };

    if shared_state.auth_token.len() < 8 {
        eprintln!("Authentication Token required to be 8 or more characters long!");
        return;
    }

    let app = Router::new()
        .route("/summary", get(handlers::summary))
        .route("/checksum", get(handlers::checksum))
        .route("/upload", post(handlers::upload_package).layer(DefaultBodyLimit::max(4 * 1024 * 1024 * 1024))
        )
        // .route(
        //     "/update",
        //     post(handlers::update_package)
        //         .layer(DefaultBodyLimit::max(4 * 1024 * 1024 * 1024))
        // )
        .route("/get/{*capture}", get(handlers::get_file))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(&url).await.unwrap();
    println!("Server running on http://{}", url);
    axum::serve(listener, app).await.unwrap();
}