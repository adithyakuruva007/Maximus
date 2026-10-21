use async_trait::async_trait;
use max_core::errors::Result;
use max_core::traits::Tool;
use max_state::StateStore;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct SessionSearchTool {
    state: Arc<StateStore>,
}

impl SessionSearchTool {
    pub fn new(state: Arc<StateStore>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str {
        "session_search"
    }

    fn description(&self) -> &str {
        "Search through past conversation history using full-text search (FTS5). Pass 'query' to search, or leave empty to browse recent sessions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query terms to locate relevant past messages."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of search results to return (default: 5)."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        if query.trim().is_empty() {
            return Ok(json!({
                "mode": "browse",
                "message": "Browsing past sessions (empty query provided)"
            }).to_string());
        }

        let hits = self.state.search_messages(query, limit)?;
        let formatted_hits: Vec<Value> = hits
            .into_iter()
            .map(|(session_id, snippet)| {
                json!({
                    "session_id": session_id,
                    "snippet": snippet
                })
            })
            .collect();

        Ok(json!({
            "query": query,
            "results_count": formatted_hits.len(),
            "results": formatted_hits
        }).to_string())
    }
}
