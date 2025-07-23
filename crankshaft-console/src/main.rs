//! The main binary for the console TUI.

/// The `conn` module handles connections to the crankshaft server.
mod conn;
/// The `input` module handles keyboard input.
mod input;
/// The `state` module manages the application state.
mod state;
/// The `term` module handles terminal initialization and cleanup.
mod term;
/// The `view` module renders the user interface.
mod view;

use Event::*;
use KeyCode::*;
use color_eyre::Section;
use color_eyre::SectionExt;
use color_eyre::eyre::eyre;
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
async fn main() -> color_eyre::Result<()> {
    let (mut terminal, _cleanup) = init_crossterm().unwrap();
    let mut conn = Connection::new(Uri::from_static("http://localhost:8080"));
    let mut state = State::default();
    let view = View::Tasks;
    let mut input = Box::pin(input::EventStream::new());
    let styles = Styles::new();

    loop {
        tokio::select! {biased;
            input = input.next() =>{
                let input = input
                .ok_or_else(|| eyre!("keyboard input stream ended early"))
                .with_section(|| "this is probably a bug".header("Note:"))??;

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
            instrument_message = conn.next_message(&mut state)=>{
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
