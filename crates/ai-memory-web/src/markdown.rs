//! Markdown → HTML rendering with `syntect` syntax highlighting.
//!
//! Stub at scaffold time; full implementation in the next step.

use pulldown_cmark::{Options, Parser, html};

/// Render a markdown body to HTML using GFM-ish defaults.
///
/// v1: trust the wiki source (the wiki is on-disk markdown the
/// project owner writes/consolidates; not user-uploaded content from
/// arbitrary callers). Syntax highlighting is deferred.
#[must_use]
pub fn render(body: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(body, opts);
    let mut out = String::with_capacity(body.len() + body.len() / 4);
    html::push_html(&mut out, parser);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_markdown() {
        let html = render("# Hello\n\nworld");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<p>world</p>"));
    }

    #[test]
    fn renders_tables() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let html = render(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }
}
