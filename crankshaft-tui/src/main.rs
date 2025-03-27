use std::env;
use std::error::Error;
use std::fmt;

mod app;
mod screens;
mod ui;
mod renderer;

#[derive(Debug)]
struct CommandError;

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Command not recognized. Launch crankshaft with `crankshaft launch`.")
    }
}

impl Error for CommandError {}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "launch" {
        let _ = app::run();
    } else {
        return Err(Box::new(CommandError));
    }

    Ok(())
}
