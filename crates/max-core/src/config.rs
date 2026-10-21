use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_iterations: usize,
    pub environment: String,
    pub memory_provider: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "claude-3-5-sonnet-20241022".to_string(),
            provider: "anthropic".to_string(),
            api_key: None,
            base_url: None,
            max_iterations: 30,
            environment: "local".to_string(),
            memory_provider: None,
        }
    }
}

pub fn get_max_home() -> PathBuf {
    if let Ok(home) = std::env::var("MAX_HOME") {
        return PathBuf::from(home);
    }
    if let Some(user_home) = dirs::home_dir() {
        return user_home.join(".maximus");
    }
    PathBuf::from(".maximus")
}
