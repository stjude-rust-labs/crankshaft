//! The module for keyboard input handling.
pub use crossterm::event::*;

/// function to check if the key event is a release or repeat event
pub fn should_ignore_key_event(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            kind: KeyEventKind::Release | KeyEventKind::Repeat,
            ..
        })
    )
}

/// function to check if quit keys are pressed
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
