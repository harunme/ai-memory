//! `askama` template definitions and per-route view-models.

use askama::Template;

// ---------------------------------------------------------------------------
// Humanise helper
// ---------------------------------------------------------------------------

/// Format an ISO-8601 timestamp string as a relative human-readable string
/// (e.g. "3 hours ago", "2 days ago"). Falls back to the raw string on any
/// parse error.
#[must_use]
pub(crate) fn humanize(iso: &str) -> String {
    let Ok(then) = iso.parse::<jiff::Timestamp>() else {
        return iso.to_owned();
    };
    let now = jiff::Timestamp::now();
    // Compute elapsed seconds using microsecond arithmetic to avoid Span API.
    let diff_us = now.as_microsecond() - then.as_microsecond();
    let secs = diff_us.abs() / 1_000_000;
    if secs < 60 {
        return "just now".to_owned();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins} minute{} ago", if mins == 1 { "" } else { "s" });
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" });
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days} day{} ago", if days == 1 { "" } else { "s" });
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months} month{} ago", if months == 1 { "" } else { "s" });
    }
    let years = months / 12;
    format!("{years} year{} ago", if years == 1 { "" } else { "s" })
}

// ---------------------------------------------------------------------------
// projects.html
// ---------------------------------------------------------------------------

/// One card on the project-list page.
pub(crate) struct ProjectCard {
    /// Workspace name.
    pub workspace: String,
    /// Project name.
    pub project: String,
    /// Number of latest pages.
    pub page_count: u64,
    /// Humanised timestamp (e.g. "3 hours ago"), or empty string.
    pub last_updated_relative: String,
    /// Link target (`/web/w/{ws}/{proj}`).
    pub href: String,
}

/// View-model for `GET /`.
#[derive(Template)]
#[template(path = "projects.html")]
pub(crate) struct ProjectsView {
    /// All project cards, sorted by most recently active first.
    pub projects: Vec<ProjectCard>,
}

// ---------------------------------------------------------------------------
// project.html
// ---------------------------------------------------------------------------

/// One entry in the sidebar or recent-list.
pub(crate) struct PageRow {
    /// Relative wiki path.
    pub path: String,
    /// Page title.
    pub title: String,
    /// Semantic kind badge text.
    pub kind: String,
    /// Humanised updated timestamp.
    pub updated_relative: String,
}

/// A folder in the sidebar tree (groups pages by first path segment).
pub(crate) struct Folder {
    /// Folder name (first path segment, without trailing slash).
    pub name: String,
    /// Pages inside this folder.
    pub pages: Vec<PageRow>,
}

/// View-model for `GET /w/:workspace/:project`.
#[derive(Template)]
#[template(path = "project.html")]
pub(crate) struct ProjectView {
    /// Workspace name.
    pub workspace: String,
    /// Project name.
    pub project: String,
    /// Sidebar folder tree.
    pub folders: Vec<Folder>,
    /// N most-recent pages for the right column.
    pub recent: Vec<PageRow>,
}

// ---------------------------------------------------------------------------
// page.html
// ---------------------------------------------------------------------------

/// View-model for `GET /w/:workspace/:project/p/*path`.
#[derive(Template)]
#[template(path = "page.html")]
pub(crate) struct PageView {
    /// Workspace name.
    pub workspace: String,
    /// Project name.
    pub project: String,
    /// Relative wiki path.
    pub path: String,
    /// Page title.
    pub title: String,
    /// Semantic kind.
    pub kind: String,
    /// Memory tier.
    pub tier: String,
    /// Whether the page is pinned.
    pub pinned: bool,
    /// Humanised updated timestamp.
    pub updated_relative: String,
    /// Humanised created timestamp.
    pub created_relative: String,
    /// Path of the page this supersedes, or empty string.
    pub supersedes_path: String,
    /// Rendered markdown body (HTML, trusted).
    pub body_html: String,
}

// ---------------------------------------------------------------------------
// search.html
// ---------------------------------------------------------------------------

/// One FTS5 search hit.
pub(crate) struct SearchHit {
    /// Workspace name.
    pub workspace: String,
    /// Project name.
    pub project: String,
    /// Relative wiki path.
    pub path: String,
    /// Page title.
    pub title: String,
    /// FTS5 snippet (HTML-marked with `<mark>` tags).
    pub snippet: String,
}

/// View-model for `GET /search?q=…`.
#[derive(Template)]
#[template(path = "search.html")]
pub(crate) struct SearchView {
    /// The raw query string.
    pub query: String,
    /// FTS5 search hits.
    pub hits: Vec<SearchHit>,
    /// Pre-computed hit count for display (avoids needing `|length` filter).
    pub hit_count: usize,
}

// ---------------------------------------------------------------------------
// not_found.html
// ---------------------------------------------------------------------------

/// View-model for 404 responses.
#[derive(Template)]
#[template(path = "not_found.html")]
pub(crate) struct NotFoundView {}
