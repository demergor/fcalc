mod fold_expr;
mod functions;
mod io;
mod operation;
mod opts;
mod terminal;
mod variables;

use std::{
    env, error::Error, fs, path::PathBuf, sync::{OnceLock, atomic::{AtomicBool, Ordering}},
};

use crate::{
    io::{InputParser, IoHandler, Key},
    terminal::Terminal,
};

static SIGINT: AtomicBool = AtomicBool::new(false);
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static VAR_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static FUNC_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

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

    while let io::State::Continue(_, _) = state
        && !SIGINT.load(Ordering::Relaxed)
    {
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

pub fn config_path() -> &'static PathBuf {
    CONFIG_PATH.get_or_init(|| env::home_dir().unwrap().join(".config/fcalc/"))
}

pub fn var_config_path() -> &'static PathBuf {
    VAR_CONFIG_PATH.get_or_init(|| config_path().join("variables.txt"))
}

pub fn func_config_path() -> &'static PathBuf {
    FUNC_CONFIG_PATH.get_or_init(|| config_path().join("functions.txt"))
}
