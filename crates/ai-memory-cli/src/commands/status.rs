//! `ai-memory status` — report runtime config and persisted counts.
//!
//! Thin HTTP client. Calls `GET /admin/status` on the configured
//! server; renders the response as human text or JSON. Never opens
//! the store directly — the server is the source of truth.

use anyhow::Result;
use serde::Deserialize;

use crate::cli::StatusArgs;
use crate::config::Config;
use crate::http_client::{ServerEndpoint, get_json};

/// Server-shaped response. Mirrors `ai_memory_mcp::admin::StatusReport`.
#[derive(Debug, Deserialize)]
struct Report {
    /// Server binary version.
    version: String,
    /// Server-side data directory path.
    data_dir: String,
    /// Server bind address.
    bind: String,
    /// Server-side SQLite path.
    db_path: String,
    /// Lifetime counts.
    counts: Counts,
}

#[derive(Debug, Deserialize)]
struct Counts {
    pages_latest: u64,
    pages_all: u64,
    sessions: u64,
    observations: u64,
}

/// Run the `status` subcommand.
///
/// # Errors
/// Returns an error if the server is unreachable, returns non-2xx, or
/// the response can't be parsed.
pub async fn run(config: &Config, args: StatusArgs) -> Result<()> {
    let ep = ServerEndpoint::from_config(config);
    let report: Report = get_json(&ep, "/admin/status", &[]).await?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "version": report.version,
                "data_dir": report.data_dir,
                "bind": report.bind,
                "db_path": report.db_path,
                "counts": {
                    "pages_latest": report.counts.pages_latest,
                    "pages_all": report.counts.pages_all,
                    "sessions": report.counts.sessions,
                    "observations": report.counts.observations,
                },
                "client": { "server_url": ep.url, "auth": ep.auth_token.is_some() },
            }))?
        );
    } else {
        println!("ai-memory {} (server)", report.version);
        println!("  server:       {}", ep.url);
        println!("  data-dir:     {}", report.data_dir);
        println!("  db:           {}", report.db_path);
        println!("  bind:         {}", report.bind);
        println!(
            "  pages:        {} (all versions: {})",
            report.counts.pages_latest, report.counts.pages_all
        );
        println!("  sessions:     {}", report.counts.sessions);
        println!("  observations: {}", report.counts.observations);
    }
    Ok(())
}
