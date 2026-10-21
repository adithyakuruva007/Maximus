use clap::{Parser, Subcommand};
use max_core::config::get_max_home;
use max_core::errors::Result;
use max_core::models::SessionMetadata;
use max_memory::{BoundedStore, SessionSearchTool, MEMORY_CHAR_LIMIT, USER_CHAR_LIMIT};
use max_providers::GenericOpenAIProvider;
use max_runtime::Agent;
use max_state::StateStore;
use max_tools::ToolRegistry;
use max_tui::TuiApp;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser, Debug)]
#[command(name = "maximus", author, version, about = "Maximus: High-performance autonomous AI agent runtime written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start interactive conversation in terminal
    Chat {
        #[arg(short, long)]
        prompt: Option<String>,
        #[arg(short, long, default_value = "gpt-4o")]
        model: String,
        #[arg(long, default_value = "https://api.openai.com/v1")]
        base_url: String,
    },
    /// Open the full terminal user interface (TUI)
    Tui,
    /// Inspect or manage agent memories
    Memory {
        #[arg(short, long)]
        show: bool,
    },
    /// Show current version and runtime diagnostics
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let max_home = get_max_home();
    let db_path = max_home.join("state.db");
    let memory_path = max_home.join("memories").join("MEMORY.md");
    let user_path = max_home.join("memories").join("USER.md");

    match cli.command {
        Some(Commands::Tui) => {
            let mut app = TuiApp::new();
            app.run_preview()?;
        }
        Some(Commands::Memory { show }) => {
            println!("Maximus Memory Stores (Path: {})", max_home.join("memories").display());
            let mem = BoundedStore::open(&memory_path, MEMORY_CHAR_LIMIT)?;
            let user = BoundedStore::open(&user_path, USER_CHAR_LIMIT)?;

            println!("{}", mem.render_system_prompt("MEMORY.md"));
            println!("{}", user.render_system_prompt("USER.md"));

            if !show {
                println!("\nUse 'maximus chat' to interact with the agent or trigger memory updates.");
            }
        }
        Some(Commands::Doctor) => {
            println!("=== Maximus Doctor Diagnostic ===");
            println!("OS: {}", std::env::consts::OS);
            println!("Architecture: {}", std::env::consts::ARCH);
            println!("Runtime home: {}", max_home.display());
            println!("Database: {}", if db_path.exists() { "Connected" } else { "Will create on first run" });
            println!("Memory dir: {}", if memory_path.parent().map(|p| p.exists()).unwrap_or(false) { "Initialized" } else { "Pending" });
        }
        cmd => {
            let (prompt, model, base_url) = match cmd {
                Some(Commands::Chat { prompt, model, base_url }) => (prompt, model, base_url),
                _ => (None, "gpt-4o".to_string(), "https://api.openai.com/v1".to_string()),
            };
            let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy_key".to_string());
            let provider = Arc::new(GenericOpenAIProvider::new("openai", base_url, api_key, model.clone()));

            std::fs::create_dir_all(&max_home)?;
            let state = Arc::new(StateStore::open(&db_path)?);

            let mut tools = ToolRegistry::with_defaults();
            tools.register(Arc::new(SessionSearchTool::new(Arc::clone(&state))));
            let tools_arc = Arc::new(tools);

            let mem_store = Arc::new(RwLock::new(BoundedStore::open(&memory_path, MEMORY_CHAR_LIMIT)?));
            let user_store = Arc::new(RwLock::new(BoundedStore::open(&user_path, USER_CHAR_LIMIT)?));

            let agent = Agent::new(provider, tools_arc, Arc::clone(&state), mem_store, user_store);
            let mut session = SessionMetadata::default();
            session.model = model;
            state.create_session(&session)?;

            println!("☤ Maximus Agent initialized (Session: {})", session.session_id);

            if let Some(p) = prompt {
                println!("User > {}", p);
                match agent.run_turn(&session.session_id, &p).await {
                    Ok(resp) => println!("Agent > {}", resp),
                    Err(err) => eprintln!("Error > {}", err),
                }
            } else {
                println!("Interactive session started. (Type 'exit' to quit)\n");
                use std::io::{self, Write};
                loop {
                    print!("User > ");
                    io::stdout().flush().unwrap();
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_err() || input.trim() == "exit" {
                        break;
                    }
                    let prompt_text = input.trim();
                    if prompt_text.is_empty() {
                        continue;
                    }

                    match agent.run_turn(&session.session_id, prompt_text).await {
                        Ok(resp) => println!("\nAgent > {}\n", resp),
                        Err(err) => eprintln!("\nError > {}\n", err),
                    }
                }
            }
        }
    }

    Ok(())
}
