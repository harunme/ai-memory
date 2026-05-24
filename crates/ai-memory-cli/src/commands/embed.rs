//! `ai-memory embed` — thin HTTP client for the M9 embedding backfill.

use anyhow::Result;
use serde::Serialize;

use crate::cli::EmbedArgs;
use crate::config::Config;
use crate::http_client::{ServerEndpoint, post_json};

/// Request sent to `POST /admin/embed`.
#[derive(Serialize)]
struct EmbedRequest {
    workspace: String,
    project: String,
    reembed: bool,
    dry_run: bool,
}

/// Run the `embed` subcommand.
///
/// Sends the request to the server over HTTP and prints the JSON
/// response. In dry-run mode the server counts pages that would be
/// embedded without calling the embedder or writing anything.
///
/// # Errors
/// Returns an error if the server is unreachable or returns a non-2xx
/// response.
pub async fn run(config: &Config, args: EmbedArgs) -> Result<()> {
    let endpoint = ServerEndpoint::from_config(config);
    let project = super::resolve_project_name(config, args.project.as_deref())?;
    let report: serde_json::Value = post_json(
        &endpoint,
        "/admin/embed",
        &EmbedRequest {
            workspace: args.workspace,
            project,
            // The CLI flag was historically named `force`; the server
            // field is `reembed` — map them here.
            reembed: args.force,
            dry_run: args.dry_run,
        },
    )
    .await?;

    if args.dry_run {
        let would = report["would_embed"].as_u64().unwrap_or(0);
        let skipped = report["skipped"].as_u64().unwrap_or(0);
        println!("dry-run: would embed {would} page(s), {skipped} already up-to-date");
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
