use kirzner::{AppState, BusinessStore, publish, router};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
};

const USAGE: &str = "Usage:\n  kirzner\n  kirzner article import <markdown-path>\n  \
    kirzner publish package --experiment <id> --version <n> --article <id-or-path> \
    --platform <generic|juejin|zhihu|csdn|segmentfault|cnblogs|v2ex|jianshu|oschina> \
    --out <dir>\n  \
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
    if args.len() == 3 && args[0] == "campaign" {
        let command = args[1].to_str().ok_or_else(usage)?;
        let file = args[2].to_str().ok_or_else(usage)?;
        let root = std::env::current_dir()?;
        let output = match command {
            "plan" => kirzner::campaign::campaign_plan(&root, file)?,
            "status" => kirzner::campaign::campaign_status(&root, file)?,
            "approve" => {
                if !std::io::IsTerminal::is_terminal(&std::io::stdin())
                    || !std::io::IsTerminal::is_terminal(&std::io::stdout())
                {
                    return Err("Approve from an interactive operator terminal".into());
                }
                let reviewed = kirzner::campaign::campaign_plan(&root, file)?;
                println!("{}", serde_json::to_string_pretty(&reviewed)?);
                eprint!("Approve this batch? [y/N] ");
                use std::io::Write;
                std::io::stderr().flush()?;
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if answer.trim().to_lowercase() != "y" {
                    return Err("Approval cancelled".into());
                }
                let digest = reviewed["digest"]
                    .as_str()
                    .ok_or("Missing reviewed digest")?;
                serde_json::to_value(kirzner::campaign::campaign_approve(&root, file, digest)?)?
            }
            _ => return Err(usage().into()),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
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
