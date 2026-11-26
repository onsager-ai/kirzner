use kirzner::{AppState, BusinessStore, router};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = PathBuf::from(std::env::var("KIRZNER_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data)?;
    let url = format!("sqlite://{}", data.join("kirzner.sqlite3").display());
    let store = BusinessStore::connect(&url).await?;
    let port = std::env::var("KIRZNER_PORT")
        .unwrap_or_else(|_| "4317".into())
        .parse::<u16>()?;
    let frontend = PathBuf::from(
        std::env::var("KIRZNER_FRONTEND_DIR").unwrap_or_else(|_| "frontend/dist".into()),
    );
    if !frontend.join("index.html").is_file() {
        eprintln!(
            "Frontend build absent. Run npm --prefix frontend ci and npm --prefix frontend run build. The API is available."
        );
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Kirzner ({}) http://127.0.0.1:{port}", store.backend());
    axum::serve(listener, router(AppState::new(store, data), frontend)).await?;
    Ok(())
}
