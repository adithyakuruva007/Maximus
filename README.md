# Maximus ☤

> **High-Performance Autonomous AI Agent Runtime in Rust**  
> Inspired by the self-improvement loop of Max Agent and the ultra-lightweight trait architecture of [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw).

---

## Key Highlights

- **Single Self-Contained Binary**: Under 15MB release footprint, zero Python or Node.js runtime dependencies.
- **Microsecond Cold Starts**: Instant startup (<10ms) and minimal baseline memory footprint (<5MB RAM).
- **Closed Learning Loop**: Embedded background review harness (`ReviewHarness`) evaluates completed sessions to persist memories and refine skills automatically.
- **Three-Tier Persistent Memory System**:
  - **Tier 1 (Curated Notes)**: Bounded `MEMORY.md` (2,200 chars) & `USER.md` (1,375 chars) frozen snapshot injection preserving LLM prefix caches.
  - **Tier 2 (Session Archive)**: SQLite with FTS5 (`state.db`) for zero-cost semantic search across past conversations (`session_search`).
  - **Tier 3 (Pluggable Providers)**: Trait-based provider system for Honcho, Mem0, and external backends.
- **Native Ratatui TUI**: Responsive terminal user interface without Node.js or webview wrappers.

---

## Workspace Structure

```
maximus/
├── Cargo.toml
├── crates/
│   ├── max-core/         # Base domain models, traits, error handling
│   ├── max-state/        # SQLite FTS5 database store (WAL mode)
│   ├── max-memory/       # Bounded stores (MEMORY.md/USER.md) & session search
│   ├── max-providers/    # OpenAI, Nous Portal, Anthropic async streaming clients
│   ├── max-tools/        # Builtin file tools, terminal sandbox, tool registry
│   ├── max-sandbox/      # Local, Docker, and Cloud execution environments
│   ├── max-runtime/      # ReAct agent loop & background review harness
│   ├── max-channels/     # CLI, Telegram, Discord, Slack adapters
│   ├── max-tui/          # Ratatui + crossterm terminal interface
│   └── max-cli/          # Root CLI binary (`maximus`)
└── .github/
    └── workflows/
        └── build-rust.yml # GitHub Actions release build matrix
```

---

## Cloud Compilation (GitHub Actions)

This repository is configured to build standalone binaries in GitHub Actions without consuming local machine resources:

1. Push commits to `main` or trigger manually from the **Actions** tab on GitHub.
2. The workflow builds release binaries for:
   - Linux x86_64 (`maximus-x86_64-unknown-linux-gnu.tar.gz`)
   - macOS Apple Silicon (`maximus-aarch64-apple-darwin.tar.gz`)
   - Windows x86_64 (`maximus-x86_64-pc-windows-msvc.zip`)
3. Download the compiled release artifact directly from the workflow run summary!

---

## Local Usage

```bash
# Diagnostic check
maximus doctor

# Interactive chat session
maximus chat

# Run full TUI
maximus tui

# Inspect persistent memories
maximus memory
```
