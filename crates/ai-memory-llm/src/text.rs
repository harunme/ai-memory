//! Small text helpers shared by provider implementations.

/// Truncate to at most `max_bytes` without splitting a UTF-8 codepoint.
pub(crate) fn truncate_with_ellipsis(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }

    let mut end = 0;
    for (idx, ch) in s.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    format!("{}…", &s[..end])
}

/// Return a suffix no longer than `max_bytes`, aligned to a UTF-8 boundary.
pub(crate) fn suffix_within_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let start = s
        .char_indices()
        .map(|(idx, _)| idx)
        .find(|idx| s.len() - idx <= max_bytes)
        .unwrap_or(s.len());
    &s[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_ascii_prefix() {
        assert_eq!(truncate_with_ellipsis("abcdef", 3), "abc…");
        assert_eq!(truncate_with_ellipsis("abc", 3), "abc");
    }

    #[test]
    fn truncate_never_splits_utf8() {
        let s = format!("{}é", "x".repeat(1023));
        let truncated = truncate_with_ellipsis(&s, 1024);
        assert!(truncated.ends_with('…'));
        assert_eq!(truncated.chars().last(), Some('…'));
    }

    #[test]
    fn suffix_never_splits_utf8() {
        let s = format!("é{}", "x".repeat(1023));
        assert_eq!(suffix_within_bytes(&s, 1024), "x".repeat(1023));
    }
}
