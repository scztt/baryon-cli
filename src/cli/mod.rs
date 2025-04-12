mod commands;

use crate::{
    core::http::CacheSettings,
    core::{repository::HTTPRepository, settings::Settings},
    Result,
};
use clap::{Parser, Subcommand};
use std::time::Duration;

use commands::list::{self, from_json, ListArgs};

#[derive(Parser)]
#[command(version, about = "Baryon Package Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List(ListArgs),
    ListRaw { json: String },
    // Install { package: String, version: String },
    // InstallRaw { json: String },
}

pub(crate) async fn cli() -> Result<()> {
    let cli = Cli::parse();
    let settings = Settings {
        global_repository_path: "~/.baryon/repository".to_string(),
        repository_url: "https://example.com/repo.json".to_string(),
        cache_settings: CacheSettings {
            cache_path: "~/.baryon/cache".to_string(),
            cache_timeout: Duration::new(60, 0),
        },
    };
    let repo = HTTPRepository::new(&settings);

    match cli.command {
        Commands::List(args) => list::do_cli(args, &settings, &repo)
            .await
            .map(|r| {
                (serde_json::to_string_pretty(&r)
                    .unwrap_or_else(|_| "Error serializing result".to_string()),)
            })
            .map_err(|e| miette::Report::msg(format!("Error: {}", e))),

        Commands::ListRaw { json } => {
            let obj = from_json(&json)?;
            list::do_raw(&obj, &repo)
                .await
                .map(|r| {
                    (serde_json::to_string_pretty(&r)
                        .unwrap_or_else(|_| "Error serializing result".to_string()),)
                })
                .map_err(|e| miette::Report::msg(format!("Error: {}", e)))
        }
    }?;

    Ok(())
}
