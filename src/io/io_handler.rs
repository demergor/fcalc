mod normal_handler;
mod function_handler;
mod variable_handler;

use core::fmt;
use std::{error::Error, fmt::Display, io};

use normal_handler::NormalHandler;
use function_handler::FunctionHandler;
use variable_handler::VariableHandler;

use crate::{io::Key, terminal::Terminal};

pub const ERR_COLOR: &str = "\x1b[41m";
pub const ERASE_FROM_CURSOR: &str = "\x1b[0J";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const HIGHLIGHT_COLOR: &str = "\x1b[92m";
pub const RESET: &str = "\x1b[0m";
pub const SHOW_CURSOR: &str = "\x1b[?25h";


pub struct IoHandler {
    mode: Mode,
    msg: Option<String>,
    normal_handler: NormalHandler,
    func_handler: FunctionHandler,
    var_handler: VariableHandler,
}


impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        Ok(IoHandler {
            mode: Mode::Normal,
            msg: None,
            normal_handler: NormalHandler::new(bounds)?,
            func_handler: FunctionHandler::new(),
            var_handler: VariableHandler::new()?,
        })
    }

    pub fn update(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        let state = match self.mode {
            Mode::Normal => self.normal_handler.handle(key, self.msg)?,
            _ => todo!(),
            /*
            Mode::Function => self.func_handler.handle(key, self.msg)?,
            Mode::Variable => self.var_handler.handle(key, self.msg)?,
            */
        };

        if let Continue(mode, msg) = state {
            self.mode = mode;
            self.msg = msg;
        }

        Ok(state)
    }
}

#[derive(PartialEq)]
pub enum State {
    Continue(Mode, Option<String>),
    Quit(f64),
}

#[derive(Eq, PartialEq)]
enum Mode {
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

