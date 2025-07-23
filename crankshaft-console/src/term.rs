//! The `term` module handles terminal initialization and cleanup.
use std::io::Write;
use std::io::{self};

use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;

/// Initializes the terminal.
pub(crate) fn init_crossterm() -> Result<(
    ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    Cleanup,
)> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    stdout.flush()?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = ratatui::Terminal::new(backend)?;
    Ok((terminal, Cleanup))
}

/// A cleanup struct that restores the terminal state when dropped.
pub(crate) struct Cleanup;

impl Drop for Cleanup {
    /// Restores the terminal state.
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.execute(DisableMouseCapture);
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = stdout.flush();
    }
}
