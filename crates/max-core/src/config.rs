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
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            api_key: None,
            base_url: Some("https://api.openai.com/v1".to_string()),
            max_iterations: 30,
            environment: "local".to_string(),
            memory_provider: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = get_max_home().join("config.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = get_max_home();
        std::fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(dir.join("config.json"), content)?;
        Ok(())
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
