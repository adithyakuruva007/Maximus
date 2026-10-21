use async_trait::async_trait;
use max_core::errors::{MaxError, Result};
use max_core::traits::ExecutionEnvironment;
use std::fs;
use std::path::Path;
use tokio::process::Command;

pub struct LocalEnvironment;

#[async_trait]
impl ExecutionEnvironment for LocalEnvironment {
    async fn execute(&self, command: &str, timeout_secs: u64) -> Result<String> {
        let child = Command::new("bash")
            .arg("-c")
            .arg(command)
            .output();

        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child)
            .await
            .map_err(|_| MaxError::Tool("Execution timed out".to_string()))?
            .map_err(|e| MaxError::Tool(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(format!("{}\n{}", stdout, stderr))
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        fs::read_to_string(path).map_err(|e| MaxError::Tool(e.to_string()))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| MaxError::Tool(e.to_string()))?;
        }
        fs::write(p, content).map_err(|e| MaxError::Tool(e.to_string()))
    }
}
