//! `log.md` append helper.
//!
//! The log is the chronological ledger Karpathy's gist insists on — a
//! grep-able audit trail of "what happened, when". Lines use the exact
//! prefix `## [YYYY-MM-DDTHH:MM:SSZ] <event> | <title>` so unix tools
//! (`grep "^## \["`) can parse it without a markdown library.

use std::io::Write;
use std::path::Path;

use ai_memory_core::{ProjectId, WorkspaceId};
use jiff::{Timestamp, ToSpan, tz::TimeZone};
use tracing::debug;

use crate::payload::HookEvent;

/// Append one line to `<wiki_root>/<workspace_id>/<project_id>/log.md`.
///
/// POSIX `O_APPEND` writes of less than `PIPE_BUF` (4 KiB) are atomic, so
/// concurrent appenders do not interleave.
///
/// # Errors
/// Propagates any I/O failure from opening or writing the file.
pub fn append_event(
    wiki_root: &Path,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
    when: Timestamp,
    event: HookEvent,
    title: &str,
) -> std::io::Result<()> {
    let log_path = wiki_root
        .join(workspace_id.to_string())
        .join(project_id.to_string())
        .join("log.md");
    let line = format_line(when, event, title);
    debug!(path = %log_path.display(), bytes = line.len(), "appending log entry");

    if let Some(parent) = log_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    file.write_all(line.as_bytes())?;
    file.sync_data()?;
    Ok(())
}

fn format_line(when: Timestamp, event: HookEvent, title: &str) -> String {
    let stamp = when.to_zoned(TimeZone::UTC).strftime("%Y-%m-%dT%H:%M:%SZ");
    let kind = match event {
        HookEvent::SessionStart => "session-start",
        HookEvent::UserPrompt => "user-prompt",
        HookEvent::PreToolUse => "pre-tool-use",
        HookEvent::PostToolUse => "post-tool-use",
        HookEvent::PreCompact => "pre-compact",
        HookEvent::Notification => "notification",
        HookEvent::Stop => "stop",
        HookEvent::SessionEnd => "session-end",
        HookEvent::Other => "other",
    };
    let one_line: String = title
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .take(120)
        .collect();
    format!("## [{stamp}] {kind} | {one_line}\n")
}

#[allow(dead_code)] // Helper kept for parity with the original Karpathy CLAUDE.md grep pattern.
fn now() -> Timestamp {
    Timestamp::now() - 0.seconds()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::DateTime;
    use tempfile::TempDir;

    #[test]
    fn formats_line_with_expected_prefix() {
        let when: Timestamp = DateTime::new(2026, 5, 21, 12, 34, 56, 0)
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp();
        let line = format_line(when, HookEvent::SessionStart, "hello world");
        assert_eq!(
            line,
            "## [2026-05-21T12:34:56Z] session-start | hello world\n",
        );
    }

    #[test]
    fn append_creates_file_and_grows() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let ws = WorkspaceId::new();
        let proj = ProjectId::new();
        let now = Timestamp::now();
        append_event(root, ws, proj, now, HookEvent::SessionStart, "first").unwrap();
        append_event(root, ws, proj, now, HookEvent::UserPrompt, "second").unwrap();
        let log_path = root
            .join(ws.to_string())
            .join(proj.to_string())
            .join("log.md");
        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("session-start | first"));
        assert!(contents.contains("user-prompt | second"));
        // Two lines.
        assert_eq!(contents.matches("\n## [").count(), 1);
    }
}
