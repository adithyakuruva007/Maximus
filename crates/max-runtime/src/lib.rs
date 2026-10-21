use max_core::errors::{MaxError, Result};
use max_core::models::{Message, Role};
use max_core::traits::ModelProvider;
use max_memory::BoundedStore;
use max_state::StateStore;
use max_tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const MEMORY_REVIEW_PROMPT: &str = "Review the conversation above and consider saving to memory if appropriate. Focus on user preferences, communication style, or persistent environment facts. If something stands out, save it using the memory tool. If nothing is worth saving, say 'Nothing to save.' and stop.";

pub struct Agent {
    provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
    state: Arc<StateStore>,
    memory_store: Arc<RwLock<BoundedStore>>,
    user_store: Arc<RwLock<BoundedStore>>,
    max_iterations: usize,
}

impl Agent {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
        state: Arc<StateStore>,
        memory_store: Arc<RwLock<BoundedStore>>,
        user_store: Arc<RwLock<BoundedStore>>,
    ) -> Self {
        Self {
            provider,
            tools,
            state,
            memory_store,
            user_store,
            max_iterations: 25,
        }
    }

    pub async fn run_turn(&self, session_id: &str, user_input: &str) -> Result<String> {
        let user_msg = Message::user(user_input);
        self.state.append_message(session_id, &user_msg)?;

        let mut messages = self.state.get_messages(session_id)?;

        // Ensure system prompt with frozen memory snapshot is at the top
        if !messages.iter().any(|m| m.role == Role::System) {
            let mem_guard = self.memory_store.read().await;
            let user_guard = self.user_store.read().await;

            let prompt = format!(
                "{}\n\n{}",
                mem_guard.render_system_prompt("MEMORY (personal notes)"),
                user_guard.render_system_prompt("USER PROFILE")
            );
            messages.insert(0, Message::system(prompt));
        }

        let tool_schemas = self.tools.get_schemas();
        let mut iterations = 0;
        let mut final_response = String::new();

        while iterations < self.max_iterations {
            iterations += 1;

            let resp = self.provider.complete(&messages, &tool_schemas).await?;

            if !resp.tool_calls.is_empty() {
                let asst_msg = Message::assistant_tool_calls(resp.tool_calls.clone(), resp.reasoning.clone());
                self.state.append_message(session_id, &asst_msg)?;
                messages.push(asst_msg);

                for tc in resp.tool_calls {
                    let tool_result = match self.tools.dispatch(&tc.function.name, &tc.function.arguments).await {
                        Ok(res) => res,
                        Err(err) => format!("Error executing tool '{}': {}", tc.function.name, err),
                    };

                    let tool_resp_msg = Message::tool_response(&tc.id, tool_result);
                    self.state.append_message(session_id, &tool_resp_msg)?;
                    messages.push(tool_resp_msg);
                }
            } else if let Some(text) = resp.text {
                final_response = text.clone();
                let asst_msg = Message::assistant(text);
                self.state.append_message(session_id, &asst_msg)?;
                break;
            } else {
                break;
            }
        }

        // Spawn background review harness fork
        self.spawn_background_review(messages.clone());

        if final_response.is_empty() {
            Err(MaxError::Provider("Iteration budget exhausted without completion".to_string()))
        } else {
            Ok(final_response)
        }
    }

    fn spawn_background_review(&self, messages: Vec<Message>) {
        let provider = Arc::clone(&self.provider);
        let mem_store = Arc::clone(&self.memory_store);
        let _user_store = Arc::clone(&self.user_store);

        tokio::spawn(async move {
            let mut review_messages = messages;
            review_messages.push(Message::user(MEMORY_REVIEW_PROMPT));

            if let Ok(resp) = provider.complete(&review_messages, &[]).await {
                if let Some(text) = resp.text {
                    let lower = text.to_lowercase();
                    if !lower.contains("nothing to save") {
                        let mut guard = mem_store.write().await;
                        let _ = guard.add(&text);
                    }
                }
            }
        });
    }
}
