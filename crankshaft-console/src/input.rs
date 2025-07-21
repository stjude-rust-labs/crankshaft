// TODO(eliza): support Ratatui backends other than crossterm?
// This would probably involve using `spawn_blocking` to drive their blocking
// input-handling mechanisms in the background...
pub use crossterm::event::*;

/// Crossterm on windows reports key release and repeat events which have the
/// effect of duplicating key presses. This function filters out those events.
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
