//! `ai-memory commit` — manual wiki git commit.

use ai_memory_store::Store;
use ai_memory_wiki::Wiki;
use anyhow::{Context, Result};

use crate::cli::CommitArgs;
use crate::config::Config;

/// Run the `commit` subcommand.
///
/// # Errors
/// Returns an error if the wiki/git layer fails.
pub fn run(config: &Config, args: CommitArgs) -> Result<()> {
    let store = Store::open(&config.data_dir)
        .with_context(|| format!("opening store at {}", config.data_dir.display()))?;
    let wiki = Wiki::new(&config.data_dir, store.writer.clone())?;
    match wiki.commit_all(&args.message)? {
        Some(oid) => println!("committed: {oid}"),
        None => println!("nothing to commit (working tree clean)"),
    }
    Ok(())
}
