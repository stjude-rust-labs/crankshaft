//! The main binary for the console TUI.
mod conn;
mod input;
mod state;
mod term;
mod view;

use Event::*;
use KeyCode::*;
use anyhow::Context;
use anyhow::Result;
use conn::Connection;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use futures_util::StreamExt;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use state::State;
use term::init_crossterm;
use tonic::transport::Uri;
use view::View;
use view::styles::Styles;

use crate::view::bold;

/// The main function.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let (mut terminal, _cleanup) = init_crossterm().context("failed to initialize terminal")?;
    let mut conn = Connection::new(Uri::from_static("http://localhost:8080"));
    let mut state = State::default();
    let view = View::Tasks;
    let mut input = Box::pin(input::EventStream::new());
    let styles = Styles::new();

    loop {
        tokio::select! {biased;
            input = input.next() =>{
                let input = input.context("keyboard input stream ended early")??;

                if input::should_ignore_key_event(&input){
                    continue;
                }

                if input::should_quit(&input){
                    return Ok(());
                }

                match input {
                        Key(KeyEvent {
                            code: Char('t'), ..
                        }) => state.current_view = View::Tasks,
                        Key(KeyEvent{
                            code: Char('r'),..
                        })=> state.current_view = View::Resources,
                        _ => (),
                    }
            },
            instrument_message = conn.next_message(&mut state)=> {
                state.update(&styles, view, instrument_message);
            }
        };

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Percentage(95),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let header_text = conn.render(&styles);
            let header = Paragraph::new(header_text).wrap(Wrap { trim: true });
            let view_controls = Paragraph::new(Line::from(vec![
                Span::raw("views: "),
                bold("t"),
                Span::raw(" = tasks, "),
                bold("r"),
                Span::raw(" = resources"),
            ]))
            .wrap(Wrap { trim: true });

            f.render_widget(header, chunks[0]);
            f.render_widget(view_controls, chunks[1]);

            state.current_view.render(f, &mut state);
        })?;
    }
}
