use std::{
    cmp::min,
    error::Error,
    io::{stdout, BufWriter, Write},
};

use crate::{
    io::{
        io_handler::{
            IoError, Mode, ERASE_FROM_CURSOR, ERR_COLOR, HIGHLIGHT_COLOR, HOME, RESET,
            RESET_COLORS, SUCCESS_COLOR,
        },
        Key, State,
    },
    opts,
    terminal::Terminal,
    variables::{HandleResult, VarMap},
};

const DECL_HINT: &str = "\"VAR \" prefix: declare a new variable, \
    \"DEL \": delete an existing one\x1b[0m\n";

pub struct VariableHandler {
    fresh: bool,

    var_map: VarMap,
    input_buf: Vec<char>,

    cur_col: usize,
    cur_selection: usize,

    term_width: u16,
    term_height: u16,
}

impl VariableHandler {
    pub fn new(bounds: &Terminal) -> Result<VariableHandler, IoError> {
        let var_map = VarMap::new()?;

        Ok(VariableHandler {
            fresh: true,
            var_map,
            input_buf: Vec::new(),
            cur_col: 0,
            cur_selection: 0,
            term_width: bounds.width,
            term_height: bounds.height,
        })
    }

    pub fn handle_decl(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => return Ok(State::ForceQuit),
            Key::Char('q') => {
                self.fresh = true;
                return Ok(State::Continue(
                    Mode::Normal,
                    Some(String::from("Cancelled variable declaration")),
                ));
            }
            Key::Char('H') => {
                return Ok(State::Continue(
                    Mode::VariableDecl,
                    Some(DECL_HINT.to_owned()),
                ));
            }
            Key::Enter => {
                let submission: String = self.input_buf.iter().collect();
                self.fresh = true;

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

    pub fn handle_select(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => return Ok(State::ForceQuit),
            Key::Char('q') => {
                self.fresh = true;
                return Ok(State::Continue(
                    Mode::Normal,
                    Some(String::from("Cancelled variable selection")),
                ));
            }
            Key::Enter => {
                let input: String = self.input_buf.iter().collect();
                let matches = self.match_vec(&input, self.term_width.into());
                self.fresh = true;

                if matches.is_empty() || self.cur_selection >= matches.len() {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(String::from("No matching variable found!")),
                    ));
                }

                return Ok(State::Continue(
                    Mode::NormalCarry(matches[self.cur_selection].1),
                    Some(format!("Inserted {}", matches[self.cur_selection].0)),
                ));
            }
            Key::Char(ch) => {
                self.input_buf.insert(self.cur_col, ch);
                self.render_select()?;
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
                self.cur_selection = 0;
            }
            Key::Backspace => {
                if self.cur_col == 0 || self.input_buf.is_empty() {
                    return Ok(State::Continue(
                        Mode::VariableSelect,
                        Some(String::from("Nothing to delete!")),
                    ));
                }

                self.cur_col -= 1;
                self.input_buf.remove(self.cur_col);

                self.render_select()?;
                self.cursor_to(self.cur_col)?;
                self.cur_selection = 0;
            }
            Key::ArrowUp => {
                self.cur_selection = if self.cur_selection == 0 {
                    0
                } else {
                    self.cur_selection - 1
                };
                self.render_select()?;
            }
            Key::ArrowDown => {
                self.cur_selection += 1;
                self.render_select()?;
            }
            Key::ArrowRight => {
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
                return Ok(State::Continue(Mode::VariableSelect, None));
            }
            Key::ArrowLeft => {
                self.cursor_to(if self.cur_col == 0 {
                    0
                } else {
                    self.cur_col - 1
                })?;
            }
            _ => (),
        }

        Ok(State::Continue(Mode::VariableSelect, None))
    }

    pub fn render_decl(&mut self) -> Result<(), Box<dyn Error>> {
        if self.fresh {
            self.init_decl();
        }

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

    pub fn render_select(&mut self) -> Result<(), Box<dyn Error>> {
        if self.fresh {
            self.init_select();
        }

        let mut out = BufWriter::new(stdout().lock());
        if !opts::DEBUG {
            write!(out, "{HOME}{ERASE_FROM_CURSOR}")?;
        }

        let width = self.term_width as usize;
        let search: String = self.input_buf.iter().collect();
        write!(out, "Enter the variable name below:\n{search}")?;

        if self.term_height < 3 {
            out.flush()?;
            return Ok(());
        }

        let replace: String = HIGHLIGHT_COLOR.to_owned() + &search + RESET_COLORS;
        let matches: Vec<(String, f64, usize, usize)> = self
            .match_vec(&search, width)
            .iter()
            .map(|(name, val, height, acc_height)| {
                (name.replace(&search, &replace), *val, *height, *acc_height)
            })
            .collect();

        if matches.is_empty() {
            write!(out, "\x1b[2;{}H", self.cur_col + 1)?;
            out.flush()?;

            return Ok(());
        }

        let mut rem_height = self.term_height as usize - 2;
        self.cur_selection = min(self.cur_selection + 1, matches.len()) - 1;

        let mut ceiling = matches[self.cur_selection].3;
        let mut first_idx = 0;

        while ceiling >= rem_height && first_idx < matches.len() {
            ceiling -= matches[first_idx].2;
            first_idx += 1;
        }

        writeln!(out)?;
        for i in first_idx..matches.len() {
            if rem_height <= matches[i].2 {
                break;
            }

            writeln!(out)?;
            if self.cur_selection == i {
                write!(out, "=> \x1b[1m")?;
            }

            write!(out, "{} : {}\x1b[0m", matches[i].0, matches[i].1)?;
            rem_height -= matches[i].2;
        }

        write!(out, "\x1b[2;{}H", self.cur_col + 1)?;
        out.flush()?;

        Ok(())
    }

    fn cursor_to(&mut self, pos: usize) -> Result<(), Box<dyn Error>> {
        self.cur_col = pos;
        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }

    fn init_decl(&mut self) {
        self.input_buf = vec!['V', 'A', 'R', ' '];
        self.cur_col = 4;
        self.fresh = false;
    }

    fn init_select(&mut self) {
        self.input_buf.clear();
        self.cur_col = 0;
        self.cur_selection = 0;
        self.fresh = false;
    }

    fn match_vec(
        &self,
        search: &str,
        width: usize,
    ) -> Vec<(String, f64, usize, usize)> {
        let mut acc = 0;

        self.var_map
            .map
            .iter()
            .filter(|(key, _)| key.contains(search))
            .map(|(key, val)| {
                let height = ((key.chars().count() + val.to_string().chars().count())
                    - 1)
                    / width
                    + 1;
                acc += height;
                (String::from(key), *val, height, acc)
            })
            .collect()
    }
}
