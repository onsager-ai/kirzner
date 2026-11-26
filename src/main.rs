use kirzner::{AppState, BusinessStore, publish, router};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
};

const USAGE: &str = "Usage:\n  kirzner\n  kirzner article import <markdown-path>\n  \
    kirzner publish package --experiment <id> --version <n> --article <id-or-path> \
    --platform <generic|juejin|zhihu|csdn> --out <dir>\n  \
    kirzner publish record --experiment <id> --version <n> --package-digest <sha256> \
    --url <https-url>";

enum Cli<'a> {
    Import(&'a Path),
    Package(publish::PackageRequest<'a>),
    Record {
        experiment_id: &'a str,
        version: u32,
        digest: &'a str,
        url: &'a str,
    },
}

fn usage() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, USAGE)
}

fn text(value: &OsStr) -> io::Result<&str> {
    value.to_str().ok_or_else(usage)
}

fn flags<'a>(args: &'a [OsString], expected: &[&str]) -> io::Result<BTreeMap<&'a str, &'a OsStr>> {
    let mut found = BTreeMap::new();
    if args.len() != expected.len() * 2 {
        return Err(usage());
    }
    for pair in args.chunks_exact(2) {
        let key = text(&pair[0])?;
        if !expected.contains(&key) || found.insert(key, pair[1].as_os_str()).is_some() {
            return Err(usage());
        }
    }
    Ok(found)
}

fn parse_cli(args: &[OsString]) -> io::Result<Option<Cli<'_>>> {
    match args {
        [] => Ok(None),
        [noun, verb, path] if noun == "article" && verb == "import" => {
            Ok(Some(Cli::Import(Path::new(path))))
        }
        [noun, verb, args @ ..] if noun == "publish" && verb == "package" => {
            let args = flags(
                args,
                &[
                    "--experiment",
                    "--version",
                    "--article",
                    "--platform",
                    "--out",
                ],
            )?;
            Ok(Some(Cli::Package(publish::PackageRequest {
                experiment_id: text(args["--experiment"])?,
                version: text(args["--version"])?.parse().map_err(|_| usage())?,
                article: args["--article"],
                platform: text(args["--platform"])?,
                out: Path::new(args["--out"]),
            })))
        }
        [noun, verb, args @ ..] if noun == "publish" && verb == "record" => {
            let args = flags(
                args,
                &["--experiment", "--version", "--package-digest", "--url"],
            )?;
            Ok(Some(Cli::Record {
                experiment_id: text(args["--experiment"])?,
                version: text(args["--version"])?.parse().map_err(|_| usage())?,
                digest: text(args["--package-digest"])?,
                url: text(args["--url"])?,
            }))
        }
        _ => Err(usage()),
    }
}

impl Cli<'_> {
    async fn run(self, store: &BusinessStore, data: &Path) -> Result<(), publish::PublishError> {
        match self {
            Self::Import(path) => {
                let article = publish::import_article(store, data, path).await?;
                println!("{} {}", article.id, article.source_sha256);
            }
            Self::Package(request) => {
                let result = publish::package(store, data, request).await?;
                println!(
                    "{} {} {}",
                    result.attempt_id,
                    result.digest,
                    result.path.display()
                );
            }
            Self::Record {
                experiment_id,
                version,
                digest,
                url,
            } => {
                let action = publish::record(store, experiment_id, version, digest, url).await?;
                println!("{} {}", action.id, action.reference);
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let cli = parse_cli(&args)?;
    let data = PathBuf::from(std::env::var("KIRZNER_DATA_DIR").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data)?;
    let url = format!("sqlite://{}", data.join("kirzner.sqlite3").display());
    let store = BusinessStore::connect(&url).await?;
    if let Some(cli) = cli {
        cli.run(&store, &data).await?;
        return Ok(());
    }
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
