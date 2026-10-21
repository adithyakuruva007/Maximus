use max_core::errors::{MaxError, Result};
use std::fs;
use std::path::PathBuf;

pub const MEMORY_CHAR_LIMIT: usize = 2200;
pub const USER_CHAR_LIMIT: usize = 1375;
pub const ENTRY_DELIMITER: &str = "\n§\n";

pub struct BoundedStore {
    path: PathBuf,
    limit: usize,
    entries: Vec<String>,
}

impl BoundedStore {
    pub fn open(path: impl Into<PathBuf>, limit: usize) -> Result<Self> {
        let path = path.into();
        let mut entries = Vec::new();

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            for entry in content.split("§") {
                let trimmed = entry.trim();
                if !trimmed.is_empty() {
                    entries.push(trimmed.to_string());
                }
            }
        }

        Ok(Self {
            path,
            limit,
            entries,
        })
    }

    pub fn total_chars(&self) -> usize {
        self.entries.iter().map(|e| e.chars().count()).sum()
    }

    pub fn render_system_prompt(&self, title: &str) -> String {
        let count = self.total_chars();
        let pct = if self.limit > 0 {
            (count * 100) / self.limit
        } else {
            0
        };
        let mut out = format!(
            "══════════════════════════════════════════════\n{} [{}% — {}/{} chars]\n══════════════════════════════════════════════\n",
            title, pct, count, self.limit
        );
        out.push_str(&self.entries.join("\n§\n"));
        out
    }

    pub fn add(&mut self, content: &str) -> Result<()> {
        let content_trimmed = content.trim();
        if content_trimmed.is_empty() {
            return Ok(());
        }

        let new_chars = content_trimmed.chars().count();
        if self.total_chars() + new_chars > self.limit {
            return Err(MaxError::Memory(format!(
                "Memory limit exceeded: {}/{} chars. Please consolidate existing entries before adding.",
                self.total_chars(), self.limit
            )));
        }

        // Avoid exact duplicates
        if !self.entries.iter().any(|e| e == content_trimmed) {
            self.entries.push(content_trimmed.to_string());
            self.persist()?;
        }
        Ok(())
    }

    pub fn replace(&mut self, old_text: &str, new_content: &str) -> Result<()> {
        let matches: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.contains(old_text))
            .map(|(i, _)| i)
            .collect();

        match matches.len() {
            0 => Err(MaxError::Memory(format!("Substring not found: '{}'", old_text))),
            1 => {
                let idx = matches[0];
                self.entries[idx] = new_content.trim().to_string();
                if self.total_chars() > self.limit {
                    return Err(MaxError::Memory(format!(
                        "Replace would exceed limit: {}/{} chars.",
                        self.total_chars(), self.limit
                    )));
                }
                self.persist()
            }
            n => Err(MaxError::Memory(format!(
                "Ambiguous match for '{}': matches {} entries. Please provide a more specific substring.",
                old_text, n
            ))),
        }
    }

    pub fn remove(&mut self, old_text: &str) -> Result<()> {
        let matches: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.contains(old_text))
            .map(|(i, _)| i)
            .collect();

        match matches.len() {
            0 => Err(MaxError::Memory(format!("Substring not found: '{}'", old_text))),
            1 => {
                self.entries.remove(matches[0]);
                self.persist()
            }
            n => Err(MaxError::Memory(format!(
                "Ambiguous match for '{}': matches {} entries.",
                old_text, n
            ))),
        }
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = self.entries.join(ENTRY_DELIMITER);
        fs::write(&self.path, serialized)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_add_and_limit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("MEMORY.md");
        let mut store = BoundedStore::open(&path, 50).unwrap();

        assert_eq!(store.total_chars(), 0);
        store.add("First entry").unwrap();
        assert_eq!(store.total_chars(), 11);

        // Substring replace
        store.replace("First", "Updated entry").unwrap();
        assert_eq!(store.entries[0], "Updated entry");

        // Exceeding limit
        let res = store.add("This is an extremely long entry that exceeds fifty characters limit");
        assert!(res.is_err());
    }
}

