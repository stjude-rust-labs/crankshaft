use anyhow::Result;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io;

pub(crate) fn init_crossterm() -> Result<(ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>, Cleanup)> {
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let terminal = ratatui::Terminal::new(backend)?;
    Ok((terminal, Cleanup))
}

pub(crate) struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
