use std::{
    cmp::min, error::Error, io::{Write, stdout},
};

use crate::{
    io::{
        Key, State, io_handler::{
            ERASE_FROM_CURSOR, ERR_COLOR, HOME, IoError, Mode, RESET, SUCCESS_COLOR,
        },
    }, opts, variables::{HandleResult, VarMap},
};

const DECL_HINT: &str = "\"VAR \" prefix: declare a new variable, \
    \"DEL \": delete an existing one\x1b[0m\n";

pub struct VariableHandler {
    cur_col: usize,
    var_map: VarMap,
    input_buf: Vec<char>,
}

impl VariableHandler {
    pub fn new() -> Result<VariableHandler, IoError> {
        let var_map = VarMap::new()?;

        let mut var_handler = VariableHandler {
            cur_col: 0,
            var_map,
            input_buf: Vec::new(),
        };
        var_handler.reset_buf();

        Ok(var_handler)
    }

    pub fn handle_decl(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => return Ok(State::ForceQuit),
            Key::Char('q') => {
                self.reset_buf();
                return Ok(State::Continue(
                    Mode::Normal,
                    Some(String::from("Cancelled variable declaration")),
                ));
            }
            Key::Char('h') => {
                return Ok(State::Continue(
                        Mode::VariableDecl,
                        Some(DECL_HINT.to_owned())
                ));
            }
            Key::Enter => {
                let submission: String = self.input_buf.iter().collect();
                self.reset_buf();

                return match self.var_map.handle(submission) {
                    HandleResult::Insertion => Ok(State::Continue(
                        Mode::Normal,
                        Some(format!(
                            "{SUCCESS_COLOR}\
                            Variable declaration successful!\
                            {RESET}"
                        )),
                    )),
                    HandleResult::RemovalSuccess => Ok(State::Continue(
                        Mode::Normal,
                        Some(format!(
                            "{SUCCESS_COLOR}\
                            Variable removal successful!\
                            {RESET}"
                        )),
                    )),
                    HandleResult::RemovalFail => Ok(State::Continue(
                        Mode::Normal,
                        Some(format!(
                            "{ERR_COLOR}\
                            Variable removal failed!\
                            {RESET}"
                        )),
                    )),
                    HandleResult::Update => Ok(State::Continue(
                        Mode::Normal,
                        Some(format!(
                            "{SUCCESS_COLOR}\
                            Variable update successful!\
                            {RESET}"
                        )),
                    )),
                    HandleResult::GenericFail => Ok(State::Continue(
                        Mode::Normal,
                        Some(format!(
                            "{ERR_COLOR}\
                            Malformed input!\
                            {RESET}"
                        )),
                    )),
                };
            }
            Key::Char(ch) => {
                self.input_buf.insert(self.cur_col, ch);
                self.render_decl()?;
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
            }
            Key::Backspace => {
                if self.cur_col == 0 || self.input_buf.is_empty() {
                    return Ok(State::Continue(
                        Mode::VariableDecl,
                        Some(String::from("Nothing to delete!")),
                    ));
                }

                self.cur_col -= 1;
                self.input_buf.remove(self.cur_col);

                self.render_decl()?;
                self.cursor_to(self.cur_col)?;
            }
            Key::ArrowRight => {
                self.render_decl()?;
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
            }
            Key::ArrowLeft => {
                self.render_decl()?;
                self.cursor_to(if self.cur_col == 0 {
                    0
                } else {
                    self.cur_col - 1
                })?;
            }
            _ => (),
        }

        Ok(State::Continue(Mode::VariableDecl, None))
    }

    pub fn render_decl(&mut self) -> Result<(), Box<dyn Error>> {
        if !opts::DEBUG {
            print!("{HOME}{ERASE_FROM_CURSOR}");
        }

        println!(concat!(
            "Enter your variable in this format: ",
            "\x1b[32mpi\x1b[33m:\x1b[34m3.14\x1b[0m",
        ));

        print!("{}", self.input_buf.iter().collect::<String>());
        stdout().flush()?;

        Ok(())
    }

    fn cursor_to(&mut self, pos: usize) -> Result<(), Box<dyn Error>> {
        self.cur_col = pos;
        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }

    fn reset_buf(&mut self) {
        self.input_buf = vec!['V', 'A', 'R', ' '];
        self.cur_col = self.input_buf.len();
    }
}
