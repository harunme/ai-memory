//! Native hook stdin parsing regressions, including PowerShell's UTF-8 BOM.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ai-memory")
}

fn run_hook(data_dir: &Path, payload: &[u8]) -> Output {
    run_hook_event(data_dir, "pre-tool-use", payload)
}

fn run_hook_event(data_dir: &Path, event: &str, payload: &[u8]) -> Output {
    let mut child = Command::new(bin())
        .args(["--data-dir"])
        .arg(data_dir)
        .args([
            "hook",
            "--event",
            event,
            "--agent",
            "claude-code",
            "--server-url",
            "http://127.0.0.1:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn native hook");
    child
        .stdin
        .take()
        .expect("hook stdin")
        .write_all(payload)
        .expect("write hook payload");
    child.wait_with_output().expect("wait for native hook")
}

fn spool_entries(data_dir: &Path) -> Vec<std::fs::DirEntry> {
    std::fs::read_dir(data_dir.join("hook-spool"))
        .expect("hook spool")
        .collect::<Result<Vec<_>, _>>()
        .expect("spool entries")
        .into_iter()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect()
}

fn spooled_body(data_dir: &Path) -> String {
    let entries = spool_entries(data_dir);
    assert_eq!(entries.len(), 1);
    let entry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(entries[0].path()).expect("read spool entry"))
            .expect("parse spool entry");
    entry["body"].as_str().expect("spooled body").to_owned()
}

#[test]
fn native_hook_accepts_plain_and_bom_prefixed_json() {
    let payload = br#"{"session_id":"windows-test","cwd":"C:\\dev\\project","tool_name":"Read","tool_input":{"file_path":"README.md"}}"#;

    for with_bom in [false, true] {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut stdin = Vec::new();
        if with_bom {
            stdin.extend_from_slice(&[0xef, 0xbb, 0xbf]);
        }
        stdin.extend_from_slice(payload);

        let output = run_hook(tmp.path(), &stdin);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"{}\n");
        assert!(
            output.stderr.is_empty(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(spooled_body(tmp.path()).as_bytes(), payload);
    }
}

#[test]
fn malformed_native_hook_payload_warns_without_leaking_or_spooling() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let output = run_hook(
        tmp.path(),
        b"\xef\xbb\xbf{\"secret\":\"SENTINEL_PRIVATE_PAYLOAD\"",
    );

    assert!(output.status.success());
    assert_eq!(output.stdout, b"{}\n");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert_eq!(
        stderr,
        "ai-memory hook warning: could not parse event payload as JSON; nothing was captured\n"
    );
    assert!(!stderr.contains("SENTINEL_PRIVATE_PAYLOAD"));
    assert!(!tmp.path().join("hook-spool").exists());
}

#[test]
fn stop_hook_strips_last_assistant_message_from_spool_and_stderr() {
    // A well-formed Stop payload carrying Claude Code's `last_assistant_message`
    // must be spooled WITHOUT that raw field (#196). Optional capture remains
    // disabled, so the field must not reach the spool, wire, or stderr.
    let tmp = tempfile::tempdir().expect("tempdir");
    let payload = br#"{"session_id":"stop-strip","cwd":"/tmp/project","last_assistant_message":"SENTINEL_ASSISTANT_MESSAGE"}"#;

    let output = run_hook_event(tmp.path(), "stop", payload);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"{}\n");
    assert!(
        output.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body = spooled_body(tmp.path());
    assert!(
        !body.contains("SENTINEL_ASSISTANT_MESSAGE"),
        "spooled body still carries the assistant message: {body}"
    );
    assert!(
        !body.contains("last_assistant_message"),
        "spooled body still carries the raw field key: {body}"
    );
    // Unrelated fields survive so the Stop event is still ingested.
    assert!(
        body.contains("stop-strip"),
        "session_id was dropped: {body}"
    );
}

#[test]
fn spool_files_never_leak_the_assistant_field_on_disk() {
    // Byte-level check across the whole spool file (not just the parsed body):
    // neither the value nor the raw key may survive anywhere in the entry.
    let tmp = tempfile::tempdir().expect("tempdir");
    let payload =
        br#"{"session_id":"disk-scan","last_assistant_message":"SENTINEL_ASSISTANT_MESSAGE"}"#;

    let output = run_hook_event(tmp.path(), "stop", payload);
    assert!(output.status.success());

    let entries = spool_entries(tmp.path());
    assert_eq!(entries.len(), 1);
    let bytes = std::fs::read(entries[0].path()).expect("read spool entry");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        !text.contains("SENTINEL_ASSISTANT_MESSAGE"),
        "assistant message leaked into the spool file bytes"
    );
    assert!(
        !text.contains("last_assistant_message"),
        "raw assistant field key leaked into the spool file bytes"
    );
}
