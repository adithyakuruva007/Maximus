use chrono::Utc;
use max_core::errors::{MaxError, Result};
use max_core::models::{Message, Role, SessionMetadata};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct StateStore {
    conn: Arc<Mutex<Connection>>,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| MaxError::Database(e.to_string()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| MaxError::Database(e.to_string()))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| MaxError::Database(e.to_string()))?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                parent_session_id TEXT,
                model TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT,
                tool_calls TEXT,
                tool_call_id TEXT,
                reasoning TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS fts_messages USING fts5(
                content,
                session_id UNINDEXED,
                role UNINDEXED
            );

            CREATE TRIGGER IF NOT EXISTS trg_messages_ai AFTER INSERT ON messages
            WHEN new.content IS NOT NULL
            BEGIN
                INSERT INTO fts_messages(content, session_id, role)
                VALUES (new.content, new.session_id, new.role);
            END;",
        )
        .map_err(|e| MaxError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn create_session(&self, meta: &SessionMetadata) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, parent_session_id, model, provider, created_at, updated_at, input_tokens, output_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                meta.session_id,
                meta.parent_session_id,
                meta.model,
                meta.provider,
                meta.created_at.to_rfc3339(),
                meta.updated_at.to_rfc3339(),
                meta.total_input_tokens,
                meta.total_output_tokens,
            ],
        )
        .map_err(|e| MaxError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn append_message(&self, session_id: &str, msg: &Message) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tool_calls_json = msg.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default());
        let role_str = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        };

        conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_calls, tool_call_id, reasoning, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                role_str,
                msg.content,
                tool_calls_json,
                msg.tool_call_id,
                msg.reasoning,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| MaxError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT role, content, tool_calls, tool_call_id, reasoning
                 FROM messages WHERE session_id = ?1 ORDER BY id ASC",
            )
            .map_err(|e| MaxError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([session_id], |row| {
                let role_str: String = row.get(0)?;
                let content: Option<String> = row.get(1)?;
                let tool_calls_str: Option<String> = row.get(2)?;
                let tool_call_id: Option<String> = row.get(3)?;
                let reasoning: Option<String> = row.get(4)?;

                let role = match role_str.as_str() {
                    "system" => Role::System,
                    "user" => Role::User,
                    "tool" => Role::Tool,
                    _ => Role::Assistant,
                };

                let tool_calls = tool_calls_str.and_then(|s| serde_json::from_str(&s).ok());

                Ok(Message {
                    role,
                    content,
                    tool_calls,
                    tool_call_id,
                    reasoning,
                })
            })
            .map_err(|e| MaxError::Database(e.to_string()))?;

        let mut messages = Vec::new();
        for r in rows {
            messages.push(r.map_err(|e| MaxError::Database(e.to_string()))?);
        }
        Ok(messages)
    }

    pub fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT session_id, snippet(fts_messages, 0, '[MATCH]', '[/MATCH]', '...', 10)
                 FROM fts_messages WHERE fts_messages MATCH ?1 LIMIT ?2",
            )
            .map_err(|e| MaxError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| MaxError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| MaxError::Database(e.to_string()))?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_fts5_search() {
        let store = StateStore::in_memory().unwrap();
        let session = SessionMetadata::default();
        store.create_session(&session).unwrap();

        let msg1 = Message::user("We are migrating to a Rust agent runtime");
        let msg2 = Message::assistant("Maximus is now running with SQLite and FTS5");
        store.append_message(&session.session_id, &msg1).unwrap();
        store.append_message(&session.session_id, &msg2).unwrap();

        let msgs = store.get_messages(&session.session_id).unwrap();
        assert_eq!(msgs.len(), 2);

        let hits = store.search_messages("Rust", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, session.session_id);
    }
}

