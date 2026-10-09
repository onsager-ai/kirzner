use kirzner::{AppState, BusinessStore, router, semon::SemonConnection};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = PathBuf::from(std::env::var("KIRZNER_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data)?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("capture-codex") {
        if args.len() != 3 {
            return Err("Usage: kirzner capture-codex /native/codex/sessions /native/codex/history.jsonl (history may be absent)".into());
        }
        if !SemonConnection::from_env(&data)
            .capture_codex(&PathBuf::from(&args[1]), &PathBuf::from(&args[2]))?
        {
            return Err("Semon capture failed; business records are unaffected".into());
        }
        println!(
            "Semon capture pass completed. Inspect Semon's own report; completion alone is not proof of full session coverage."
        );
        return Ok(());
    }
    if !args.is_empty() {
        return Err("Usage: kirzner [capture-codex SESSIONS HISTORY]".into());
    }
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}", data.join("kirzner.sqlite3").display()));
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
