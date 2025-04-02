use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

#[derive(Debug, Clone, Copy)]
pub enum MouseAction {
    Clicked,
    Pressed,
    Released,
    Moved,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent, MouseAction),
    Tick,
    Resize(u16, u16),
}

#[derive(Debug)]
pub struct EventHandler {
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or_else(|| Duration::from_secs(0));

                    if event::poll(timeout).unwrap() {
                        match event::read().unwrap() {
                            CrosstermEvent::Key(key) => {
                                if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                                    break;
                                }
                                sender.send(Event::Key(key)).unwrap();
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                let action = match mouse.kind {
                                    MouseEventKind::Down(_) => MouseAction::Pressed,
                                    MouseEventKind::Up(_) => MouseAction::Released,
                                    MouseEventKind::Drag(_) => MouseAction::Moved,
                                    MouseEventKind::ScrollDown => MouseAction::ScrollDown,
                                    MouseEventKind::ScrollUp => MouseAction::ScrollUp,
                                    _ => MouseAction::Moved,
                                };
                                sender.send(Event::Mouse(mouse, action)).unwrap();
                                
                                // Generate click events when mouse is released
                                if matches!(mouse.kind, MouseEventKind::Up(_)) {
                                    sender.send(Event::Mouse(mouse, MouseAction::Clicked)).unwrap();
                                }
                            }
                            CrosstermEvent::Resize(width, height) => {
                                sender.send(Event::Resize(width, height)).unwrap();
                            }
                            _ => {}
                        }
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.send(Event::Tick).unwrap();
                        last_tick = Instant::now();
                    }
                }
            })
        };

        Self { sender, receiver, handler }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.receiver.recv()
    }
}
