mod fold_expr;
mod io;
mod operation;
mod opts;
mod terminal;
mod variables;

use std::{
    env,
    error::Error,
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    io::{InputParser, IoHandler, Key},
    terminal::Terminal,
};

static SIGINT: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(_: libc::c_int) {
    SIGINT.store(true, Ordering::Relaxed);
}

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_sigint as *const () as libc::sighandler_t,
        );
    }

    fs::create_dir_all(env::home_dir().unwrap().join(".config/fcalc"))?;

    let term = Terminal::new()?;
    let mut io_handler = IoHandler::new(&term)?;
    let mut input_parser = InputParser::new();
    let mut state = io_handler.update(Key::Char('='))?;

    while let io::State::Continue(_, _) = state && !SIGINT.load(Ordering::Relaxed) {
        input_parser.poll_key();
        while let Some(key) = input_parser.pending_keys.pop_front() {
            if let io::State::Quit(_) = state {
                break; 
            }

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
