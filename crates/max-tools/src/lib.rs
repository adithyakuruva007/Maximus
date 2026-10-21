use async_trait::async_trait;
use max_core::errors::{MaxError, Result};
use max_core::traits::Tool;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file from the local filesystem."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| MaxError::Tool("Missing 'path' argument".to_string()))?;

        fs::read_to_string(path_str).map_err(|e| MaxError::Tool(e.to_string()))
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file on the local filesystem (creates parent directories if missing)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write."
                },
                "content": {
                    "type": "string",
                    "description": "Content to write into the file."
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| MaxError::Tool("Missing 'path' argument".to_string()))?;
        let content = args
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| MaxError::Tool("Missing 'content' argument".to_string()))?;

        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| MaxError::Tool(e.to_string()))?;
        }
        fs::write(path, content).map_err(|e| MaxError::Tool(e.to_string()))?;
        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path_str))
    }
}

pub struct TerminalTool;

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Execute bash shell commands in the workspace environment."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command string to execute in bash."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let cmd = args
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| MaxError::Tool("Missing 'command' argument".to_string()))?;

        let output = Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .output()
            .await
            .map_err(|e| MaxError::Tool(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("Exit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", output.status.code().unwrap_or(-1), stdout, stderr))
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(ReadFileTool));
        reg.register(Arc::new(WriteFileTool));
        reg.register(Arc::new(TerminalTool));
        reg
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters_schema()
                    }
                })
            })
            .collect()
    }

    pub async fn dispatch(&self, name: &str, args_json: &str) -> Result<String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| MaxError::Tool(format!("Tool '{}' not found", name)))?;

        let parsed_args: Value = serde_json::from_str(args_json)
            .unwrap_or_else(|_| json!({ "raw": args_json }));

        tool.execute(parsed_args).await
    }
}
