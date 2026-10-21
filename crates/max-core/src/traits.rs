use async_trait::async_trait;
use serde_json::Value;

use crate::errors::Result;
use crate::models::{Message, ToolCall};

/// Core tool trait that all native tools, MCP tools, and external tools implement.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<String>;
}

/// Abstract Memory Provider trait (Builtin Markdown, Honcho, Mem0, etc.)
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&mut self, session_id: &str) -> Result<()>;
    fn system_prompt_block(&self) -> Option<String>;
    async fn prefetch(&self, query: &str) -> Result<Option<String>>;
    async fn sync_turn(&self, user_msg: &str, asst_msg: &str, session_id: &str) -> Result<()>;
    async fn on_session_end(&self, messages: &[Message]) -> Result<()>;
}

/// Model response containing generated text and/or tool calls.
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub reasoning: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

/// Core Model Provider trait (Nous Portal, Anthropic, OpenAI, Ollama, etc.)
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, messages: &[Message], tools: &[Value]) -> Result<ModelResponse>;
}

/// Execution Sandbox Environment trait (Local, Docker, Modal, etc.)
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    async fn execute(&self, command: &str, timeout_secs: u64) -> Result<String>;
    async fn read_file(&self, path: &str) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<()>;
}

/// Channel adapter trait (Telegram, Discord, Slack, CLI)
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn send_message(&self, recipient: &str, message: &str) -> Result<()>;
}
