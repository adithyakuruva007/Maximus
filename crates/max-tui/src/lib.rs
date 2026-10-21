use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use max_core::errors::{MaxError, Result};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

pub struct TuiApp {
    pub title: String,
    pub input: String,
    pub history: Vec<String>,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            title: "Maximus Autonomous Agent".to_string(),
            input: String::new(),
            history: Vec::new(),
        }
    }

    pub fn run_preview(&mut self) -> Result<()> {
        enable_raw_mode().map_err(|e| MaxError::Io(e))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| MaxError::Io(e))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| MaxError::Io(e))?;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let header = Paragraph::new(" Maximus ☤ - High-Performance Agent (Rust)")
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).title("Header"));
            f.render_widget(header, chunks[0]);

            let chat_content = "Type your instruction and press Enter. (Press 'q' or Esc to exit TUI)";
            let body = Paragraph::new(chat_content)
                .block(Block::default().borders(Borders::ALL).title("Conversation"));
            f.render_widget(body, chunks[1]);

            let input_box = Paragraph::new("> ")
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_box, chunks[2]);
        }).map_err(|e| MaxError::Io(e))?;

        // Wait for single key event
        loop {
            if let Ok(Event::Key(key)) = event::read() {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }

        disable_raw_mode().map_err(|e| MaxError::Io(e))?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(|e| MaxError::Io(e))?;
        terminal.show_cursor().map_err(|e| MaxError::Io(e))?;

        Ok(())
    }
}
