//! The `input` module handles keyboard input.
pub use crossterm::event::*;

/// Returns true if the key event should be ignored.
pub fn should_ignore_key_event(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            kind: KeyEventKind::Release | KeyEventKind::Repeat,
            ..
        })
    )
}

/// Returns true if the quit keys are pressed.
pub fn should_quit(input: &Event) -> bool {
    use Event::*;
    use KeyCode::*;
    match input {
        Key(KeyEvent {
            code: Char('q'), ..
        }) => true,
        Key(KeyEvent {
            code: Char('c'),
            modifiers,
            ..
        })
        | Key(KeyEvent {
            code: Char('d'),
            modifiers,
            ..
        }) if modifiers.contains(KeyModifiers::CONTROL) => true,
        _ => false,
    }
}
