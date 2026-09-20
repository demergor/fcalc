mod normal_handler;
mod function_handler;
mod variable_handler;

pub use normal_handler::NormalHandler;
pub use function_handler::FunctionHandler;
pub use variable_handler::VariableHandler;

use crate::terminal::Terminal;

struct IoHandler {
    mode: Mode,
    normal_handler: NormalHandler,
    func_handler: FunctionHandler,
    var_handler: VariableHandler,
}


impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        Ok(IoHandler {
            mode: Mode::Normal,
            normal_handler: NormalHandler::new(),
            func_handler: FunctionHandler::new(),
            var_handler: VariableHandler::new(),
        })
    }

    pub fn update(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        let state = match self.mode => {
            Mode::Normal => normal_handler.handle(key)?,
            Mode::Function => func_handler.handle(key)?,
            Mode::Variable => var_handler.handle(key)?,
        };

        if let Continue(mode) = state {
            self.mode = mode;
        }

        Ok(state)
    }
}

#[derive(PartialEq)]
pub enum State {
    Continue(Mode),
    Quit(f64),
}

enum Mode {
    Normal,
    Function,
    Variable,
}
