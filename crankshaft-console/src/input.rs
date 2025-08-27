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

/// Returns true if the user wants to go to the next task.
pub fn is_next_task(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            code: KeyCode::Char('j') | KeyCode::Down,
            ..
        })
    )
}

/// Returns true if the user wants to go to the previous task.
pub fn is_previous_task(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            code: KeyCode::Char('k') | KeyCode::Up,
            ..
        })
    )
}

/// Returns true if the user wants to see the logs of the selected task.
pub fn is_view_logs(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            ..
        })
    )
}

/// Returns true if the user wants to see the task view.
pub fn is_view_tasks(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            code: KeyCode::Char('t'),
            ..
        })
    )
}

/// Returns true if the user wants to cancel the selected task.
pub fn is_cancel_task(input: &Event) -> bool {
    matches!(
        input,
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            ..
        })
    )
}
