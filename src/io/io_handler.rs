mod function_handler;
mod normal_handler;
mod variable_handler;

use core::fmt;
use std::{
    error::Error,
    fmt::Display,
    io::{self, stdout, Write},
};

use function_handler::FunctionHandler;
use normal_handler::NormalHandler;
use variable_handler::VariableHandler;

use crate::{io::Key, opts, terminal::Terminal};

pub const ERR_COLOR: &str = "\x1b[41m";
pub const SUCCESS_COLOR: &str = "\x1b[42m\x1b[30m";
pub const ERASE_FROM_CURSOR: &str = "\x1b[0J";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const HIGHLIGHT_COLOR: &str = "\x1b[92m";
pub const HOME: &str = "\x1b[H";
pub const RESET: &str = "\x1b[0m";
pub const SHOW_CURSOR: &str = "\x1b[?25h";

const MSG_FMT: &str = "\x1b[3m";

pub struct IoHandler {
    mode: Mode,
    normal_handler: NormalHandler,
    // func_handler: FunctionHandler,
    var_handler: VariableHandler,
}

impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        Ok(IoHandler {
            mode: Mode::Normal,
            normal_handler: NormalHandler::new(bounds)?,
            // func_handler: FunctionHandler::new(bounds)?,
            var_handler: VariableHandler::new()?,
        })
    }

    pub fn update(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        let state = match self.mode {
            Mode::Normal => self.normal_handler.handle(key)?,
            Mode::FunctionDecl => todo!(), // self.func_handler.handle_decl(key)?,
            Mode::FunctionSelect => todo!(), // self.func_handler.handle_select(key)?,
            Mode::VariableDecl => self.var_handler.handle_decl(key)?,
            Mode::VariableSelect => todo!(), // self.var_handler.handle_select(key)?,
        };

        if state == State::ForceQuit {
            return self.normal_handler.handle(Key::Char('q'));
        }

        let state_cp = state.clone();
        let State::Continue(mode, msg) = state else {
            return Ok(state);
        };

        if self.mode != mode {
            match mode {
                Mode::Normal => self.normal_handler.render()?,
                Mode::FunctionDecl => todo!(), // self.func_handler.render_decl()?,
                Mode::FunctionSelect => todo!(), //self.func_handler.render_select()?,
                Mode::VariableDecl => self.var_handler.render_decl()?,
                Mode::VariableSelect => todo!() // self.var_handler.render_select()?,
            }
        }

        self.mode = mode;
        if let Some(msg) = msg {
            display_msg(msg)?;
        }

        Ok(state_cp)
    }
}

#[derive(Clone, PartialEq)]
pub enum State {
    Continue(Mode, Option<String>),
    Quit(f64),
    ForceQuit,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    Normal,
    FunctionDecl,
    FunctionSelect,
    VariableDecl,
    VariableSelect,
}

#[derive(Debug)]
pub enum IoError {
    HandlerConstructionError,
    Io(io::Error),
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandlerConstructionError => {
                write!(f, "Error construtcting `IoHandler`!")
            }
            Self::Io(err) => write!(f, "`IoHandler`: {err}"),
        }
    }
}

impl Error for IoError {}

fn display_msg(msg: String) -> Result<(), io::Error> {
    if opts::DEBUG {
        println!("{MSG_FMT}{msg}{RESET}");
    } else {
        print!(
            "{HIDE_CURSOR}\x1b7{HOME}\x1b[2K{MSG_FMT}{msg}\x1b8{SHOW_CURSOR}{RESET}"
        );
    }

    stdout().flush()?;

    Ok(())
}
