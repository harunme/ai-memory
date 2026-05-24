//! YAML-frontmatter aware markdown parser and emitter.
//!
//! We deliberately do *not* use `gray_matter` here: it parses fine but
//! loses comments and key ordering on re-serialise, which is exactly the
//! "duplicate frontmatter on already-frontmatter'd files" class of bug
//! basic-memory hit (#528). Going through `serde_yaml` directly keeps the
//! round-trip predictable.

use ai_memory_core::PagePath;
use serde::{Deserialize, Serialize};

use crate::error::WikiResult;

/// A parsed markdown document with detached frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Markdown {
    /// Frontmatter as JSON for cheap querying (and stable serialisation).
    /// `Null` when the source had no frontmatter at all.
    pub frontmatter: serde_json::Value,
    /// Body excluding the frontmatter block (and the closing `---\n`).
    pub body: String,
}

/// Parse markdown text into [`Markdown`].
///
/// Recognises only the canonical `---\n<yaml>\n---\n` block at the very
/// start of the document. Anything else is treated as body.
///
/// # Errors
/// Returns [`WikiError::Yaml`] if the frontmatter block exists but does
/// not parse as YAML.
pub fn parse(input: &str) -> WikiResult<Markdown> {
    let trimmed = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    if let Some(rest) = trimmed.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---\n")
    {
        let fm_str = &rest[..end];
        let body = rest[end + 5..].to_string();
        let fm_yaml: serde_yaml::Value = serde_yaml::from_str(fm_str)?;
        let fm_json: serde_json::Value = serde_json::to_value(fm_yaml)?;
        return Ok(Markdown {
            frontmatter: fm_json,
            body,
        });
    }
    Ok(Markdown {
        frontmatter: serde_json::Value::Null,
        body: input.to_string(),
    })
}

/// Emit a [`Markdown`] back to a string. Frontmatter is serialised through
/// `serde_yaml` (so it round-trips deterministically); a `Null` or empty
/// object frontmatter is omitted entirely.
///
/// # Errors
/// Returns [`WikiError::Yaml`] if frontmatter cannot be serialised.
pub fn emit(md: &Markdown) -> WikiResult<String> {
    let has_fm = match &md.frontmatter {
        serde_json::Value::Null => false,
        serde_json::Value::Object(m) => !m.is_empty(),
        _ => true,
    };
    let mut out = String::with_capacity(md.body.len() + 32);
    if has_fm {
        let yaml = serde_yaml::to_string(&md.frontmatter)?;
        out.push_str("---\n");
        out.push_str(&yaml);
        if !yaml.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("---\n");
    }
    out.push_str(&md.body);
    Ok(out)
}

/// Derive a page title.
///
/// Priority: frontmatter.title (string) → first `# ` heading in body →
/// path stem with the `.md` suffix stripped.
#[must_use]
pub fn derive_title(frontmatter: &serde_json::Value, body: &str, path: &PagePath) -> String {
    if let Some(t) = frontmatter.get("title").and_then(serde_json::Value::as_str) {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    let s = path.as_str();
    let stem = s.rsplit_once('/').map_or(s, |(_, name)| name);
    stem.strip_suffix(".md").unwrap_or(stem).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let src = "---\ntitle: Hello\ntags:\n  - a\n  - b\n---\nThe body.\n";
        let md = parse(src).unwrap();
        assert_eq!(md.frontmatter["title"], "Hello");
        assert_eq!(md.frontmatter["tags"][0], "a");
        assert_eq!(md.body, "The body.\n");
    }

    #[test]
    fn parses_bom_prefixed_frontmatter() {
        let src = "\u{FEFF}---\ntitle: Hello\n---\nBody\n";
        let md = parse(src).unwrap();
        assert_eq!(md.frontmatter["title"], "Hello");
        assert_eq!(md.body, "Body\n");
    }

    #[test]
    fn malformed_frontmatter_returns_error() {
        let src = "---\ntitle: [unterminated\n---\nBody\n";
        assert!(parse(src).is_err());
    }

    #[test]
    fn unterminated_frontmatter_marker_is_body() {
        let src = "---\ntitle: Hello\nBody\n";
        let md = parse(src).unwrap();
        assert!(md.frontmatter.is_null());
        assert_eq!(md.body, src);
    }

    #[test]
    fn parses_body_without_frontmatter() {
        let src = "Just a body, no frontmatter.\n";
        let md = parse(src).unwrap();
        assert!(md.frontmatter.is_null());
        assert_eq!(md.body, src);
    }

    #[test]
    fn round_trip_emit_then_parse() {
        let original = Markdown {
            frontmatter: serde_json::json!({ "title": "X", "tags": ["a"] }),
            body: "Line 1\nLine 2\n".into(),
        };
        let emitted = emit(&original).unwrap();
        let parsed = parse(&emitted).unwrap();
        assert_eq!(parsed.frontmatter["title"], "X");
        assert_eq!(parsed.body, original.body);
    }

    #[test]
    fn emit_omits_empty_frontmatter() {
        let md = Markdown {
            frontmatter: serde_json::Value::Object(serde_json::Map::new()),
            body: "Hello\n".into(),
        };
        assert_eq!(emit(&md).unwrap(), "Hello\n");
    }

    #[test]
    fn title_priority_frontmatter_then_heading_then_stem() {
        let path = PagePath::new("notes/foo.md").unwrap();
        // Frontmatter wins.
        let fm = serde_json::json!({ "title": "Explicit" });
        assert_eq!(derive_title(&fm, "# Other\nbody", &path), "Explicit");
        // Heading wins over stem.
        assert_eq!(
            derive_title(&serde_json::Value::Null, "# From Body\n", &path),
            "From Body"
        );
        // Stem fallback.
        assert_eq!(
            derive_title(&serde_json::Value::Null, "no heading", &path),
            "foo"
        );
    }
}
