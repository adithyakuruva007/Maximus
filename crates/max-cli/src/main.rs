use clap::{Parser, Subcommand};
use max_core::config::{get_max_home, Config};
use max_core::errors::{MaxError, Result};
use max_core::models::SessionMetadata;
use max_memory::{BoundedStore, SessionSearchTool, MEMORY_CHAR_LIMIT, USER_CHAR_LIMIT};
use max_providers::GenericOpenAIProvider;
use max_runtime::Agent;
use max_state::StateStore;
use max_tools::ToolRegistry;
use max_tui::TuiApp;
use std::io::{self, Write};
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
    /// Interactive setup wizard (configure provider, model, API keys, and user profile)
    Setup,
    /// Start interactive conversation in terminal
    Chat {
        #[arg(short, long)]
        prompt: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
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

fn run_setup_wizard() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║       ☤ Maximus Agent Interactive Setup      ║");
    println!("╚══════════════════════════════════════════════╝\n");

    let mut cfg = Config::load();

    println!("Select your AI Provider:");
    println!("  1) OpenAI (Default: gpt-4o)");
    println!("  2) OpenRouter (300+ models)");
    println!("  3) Nous Portal (Stardust Research / Nous)");
    println!("  4) Groq (Ultra-fast Llama / Mistral)");
    println!("  5) Ollama (Local & Free, offline)");
    println!("  6) Custom endpoint\n");

    print!("Enter choice [1-6] (default: 1): ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice = choice.trim();

    let (provider, default_base_url, default_model) = match choice {
        "2" => ("openrouter", "https://openrouter.ai/api/v1", "anthropic/claude-3.5-sonnet"),
        "3" => ("nous", "https://portal.stardustresearch.com/v1", "hermes-3-llama-3.1-70b"),
        "4" => ("groq", "https://api.groq.com/openai/v1", "llama-3.3-70b-versatile"),
        "5" => ("ollama", "http://localhost:11434/v1", "llama3.1"),
        "6" => {
            print!("Enter custom base URL: ");
            io::stdout().flush().unwrap();
            let mut url = String::new();
            io::stdin().read_line(&mut url).unwrap();
            let trimmed = url.trim().to_string();
            ("custom", trimmed.leak() as &str, "custom-model")
        }
        _ => ("openai", "https://api.openai.com/v1", "gpt-4o"),
    };

    cfg.provider = provider.to_string();
    cfg.base_url = Some(default_base_url.to_string());

    // API Key
    if provider != "ollama" {
        print!("Enter API Key (press Enter to keep existing): ");
        io::stdout().flush().unwrap();
        let mut key_input = String::new();
        io::stdin().read_line(&mut key_input).unwrap();
        let key_input = key_input.trim();
        if !key_input.is_empty() {
            cfg.api_key = Some(key_input.to_string());
        }
    }

    // Model selection
    print!("Enter model name (default: {}): ", default_model);
    io::stdout().flush().unwrap();
    let mut model_input = String::new();
    io::stdin().read_line(&mut model_input).unwrap();
    let model_input = model_input.trim();
    if !model_input.is_empty() {
        cfg.model = model_input.to_string();
    } else {
        cfg.model = default_model.to_string();
    }

    // User profile memory setup (USER.md)
    println!("\n--- User Profile Setup (Initializes USER.md memory) ---");
    print!("What should Maximus call you? (e.g. Alice / Aditya): ");
    io::stdout().flush().unwrap();
    let mut name_input = String::new();
    io::stdin().read_line(&mut name_input).unwrap();
    let name_input = name_input.trim();

    print!("Any preferred communication style? (e.g. 'concise code-focused', 'detailed explanations'): ");
    io::stdout().flush().unwrap();
    let mut style_input = String::new();
    io::stdin().read_line(&mut style_input).unwrap();
    let style_input = style_input.trim();

    let max_home = get_max_home();
    let user_md_path = max_home.join("memories").join("USER.md");
    if let Ok(mut user_store) = BoundedStore::open(&user_md_path, USER_CHAR_LIMIT) {
        if !name_input.is_empty() {
            let _ = user_store.add(&format!("User's name is {}.", name_input));
        }
        if !style_input.is_empty() {
            let _ = user_store.add(&format!("Communication style preference: {}.", style_input));
        }
    }

    cfg.save().map_err(|e| MaxError::Config(e.to_string()))?;
    println!("\n✅ Configuration saved successfully to {}!", get_max_home().join("config.json").display());
    println!("You can now run 'maximus chat' or 'maximus tui' anytime!\n");

    Ok(())
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
        Some(Commands::Setup) => {
            run_setup_wizard()?;
        }
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
            let cfg = Config::load();
            println!("=== Maximus Doctor Diagnostic ===");
            println!("OS: {}", std::env::consts::OS);
            println!("Architecture: {}", std::env::consts::ARCH);
            println!("Runtime home: {}", max_home.display());
            println!("Config file: {}", max_home.join("config.json").display());
            println!("Configured Provider: {}", cfg.provider);
            println!("Configured Model: {}", cfg.model);
            println!("Database: {}", if db_path.exists() { "Connected" } else { "Will create on first run" });
            println!("Memory dir: {}", if memory_path.parent().map(|p| p.exists()).unwrap_or(false) { "Initialized" } else { "Pending" });
            println!("Status: Ready.");
        }
        cmd => {
            let cfg = Config::load();
            let (cli_prompt, cli_model, cli_base_url) = match cmd {
                Some(Commands::Chat { prompt, model, base_url }) => (prompt, model, base_url),
                _ => (None, None, None),
            };

            let model = cli_model.unwrap_or(cfg.model);
            let base_url = cli_base_url.or(cfg.base_url).unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .or(cfg.api_key)
                .unwrap_or_else(|| "dummy_key".to_string());

            let provider = Arc::new(GenericOpenAIProvider::new(&cfg.provider, base_url, api_key, model.clone()));

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

            if let Some(p) = cli_prompt {
                println!("User > {}", p);
                match agent.run_turn(&session.session_id, &p).await {
                    Ok(resp) => println!("Agent > {}", resp),
                    Err(err) => eprintln!("Error > {}", err),
                }
            } else {
                println!("Interactive session started. (Type 'exit' to quit)\n");
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
