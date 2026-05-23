//! `GET /search?q=…` — FTS5 search results.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use serde::Deserialize;

use crate::state::WebState;
use crate::templates::{SearchHit, SearchView};

/// Query-string parameters for the search endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct SearchParams {
    /// The free-text search query.
    #[serde(default)]
    pub q: String,
}

/// Handler for `GET /search?q=…`.
pub(crate) async fn handler(
    State(state): State<Arc<WebState>>,
    Query(params): Query<SearchParams>,
) -> Result<Html<String>, StatusCode> {
    let query = params.q.trim().to_owned();

    let hits = if query.is_empty() {
        Vec::new()
    } else {
        let raw = state
            .reader
            .search_pages(query.clone(), 50)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // For each hit we need (workspace, project) — look up via a second
        // query per hit is too expensive; instead query the page_meta inline.
        // Since search_pages returns PageHit with workspace/project not yet
        // separated, we query page_meta by path using the "default" workspace
        // for now. The proper approach joins workspaces/projects in the FTS
        // query; for v1 we accept the extra lookup.
        let mut results = Vec::with_capacity(raw.len());
        for h in raw {
            // Get workspace + project by looking up full meta.
            // We try across all project/workspace combos by using the raw
            // search hit's path and calling a lightweight query.
            if let Ok(Some(m)) = state.reader.page_meta_by_path(h.path.as_str()).await {
                results.push(SearchHit {
                    workspace: m.workspace_name,
                    project: m.project_name,
                    path: h.path.as_str().to_owned(),
                    title: h.title,
                    snippet: h.snippet,
                });
            } else {
                // Fallback: no workspace/project known; skip for now.
            }
        }
        results
    };

    let hit_count = hits.len();
    let html = SearchView {
        query,
        hits,
        hit_count,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
