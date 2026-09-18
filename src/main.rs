mod fold_expr;
mod io;
mod operation;
mod terminal;

use std::error::Error;

use crate::{io::{InputParser, IoHandler, Key}, terminal::Terminal};


fn main() -> Result<(), Box<dyn Error>> {
    let term = Terminal::new()?;
    let mut io_handler = IoHandler::new(&term)?;
    let mut input_parser = InputParser::new();
    let mut state = io_handler.update(Key::Enter)?;

    while state == io::State::Continue {
        input_parser.poll_key();
        if let Some(key) = input_parser.pending_keys.pop_front() {
            state = io_handler.update(key)?;
        }
    }

    drop(term);
    match state {
        io::State::Quit(last_result) => println!("{last_result}"),
        _ => (),
    }

    Ok(())
}
