//! Raw assistant-message stripping (issue #196).
//!
//! Some agent harnesses attach the assistant's final turn to their `Stop`
//! lifecycle event — Claude Code sends it as a top-level `last_assistant_message`
//! string. The text can contain code, secrets, or content from paths ai-memory
//! never otherwise sees, so the raw field is removed before spooling, transport,
//! tracing, or storage. Optional capture remains disabled.

/// Raw top-level field names known to carry assistant messages.
const ASSISTANT_MESSAGE_FIELDS: &[&str] = &["last_assistant_message"];

/// Unconditionally remove every known assistant-message field from a raw hook
/// payload's top-level object, returning whether anything was removed.
///
/// This defense is applied on both sides of the wire (client pre-spool and
/// server pre-envelope) and for every agent/event. Only top-level keys are
/// inspected, matching where supported harnesses place the field.
pub fn strip_assistant_message_raw(raw: &mut serde_json::Value) -> bool {
    let Some(object) = raw.as_object_mut() else {
        return false;
    };
    let mut removed = false;
    for field in ASSISTANT_MESSAGE_FIELDS {
        if object.remove(*field).is_some() {
            removed = true;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The strip is unconditional: it removes the raw field regardless of agent
    /// and reports the removal, so a client that never verified the agent still
    /// cannot leak it.
    #[test]
    fn strip_removes_field_and_reports_change() {
        let mut raw = serde_json::json!({
            "session_id": "s1",
            "last_assistant_message": "SENTINEL_ASSISTANT_MESSAGE",
        });
        assert!(strip_assistant_message_raw(&mut raw));
        assert!(raw.get("last_assistant_message").is_none());
        assert_eq!(raw.get("session_id").and_then(|v| v.as_str()), Some("s1"));
    }

    #[test]
    fn strip_is_noop_when_absent_or_not_object() {
        let mut without = serde_json::json!({"session_id": "s1"});
        assert!(!strip_assistant_message_raw(&mut without));
        assert_eq!(without, serde_json::json!({"session_id": "s1"}));

        let mut array = serde_json::json!(["last_assistant_message"]);
        assert!(!strip_assistant_message_raw(&mut array));
        assert_eq!(array, serde_json::json!(["last_assistant_message"]));
    }
}
