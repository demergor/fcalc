use std::{
    cmp::min,
    error::Error,
    io::{BufWriter, StdoutLock, Write, stdout},
};

use crate::{
    io::{
        Key, State,
        io_handler::{
            ERASE_FROM_CURSOR, ERR_COLOR, HIGHLIGHT_COLOR, HOME, IoError, Mode, RESET,
            RESET_COLORS, SUCCESS_COLOR,
        },
    },
    opts,
    terminal::Terminal,
    variables::{DELETION_PREFIX, HandleResult, INSERTION_PREFIX, VarMap},
};

const DECL_HINT: &str = "<V> = declare new variable, <D> = delete existing variable";

pub struct VariableHandler {
    fresh: bool,

    var_map: VarMap,
    input_buf: Vec<char>,

    cur_col: usize,
    cur_selection: Option<usize>,

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
            cur_selection: None,
            term_width: bounds.width,
            term_height: bounds.height,
        })
    }

    pub fn handle_decl(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => return Ok(State::ForceQuit),
            Key::Char('q') | Key::Escape => {
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
                if let Some(mut selection) = self.cur_selection {
                    let needle: String = self
                        .input_buf
                        .iter()
                        .skip(INSERTION_PREFIX.len())
                        .take_while(|&&ch| ch != ':')
                        .collect();
                    let matches = self.match_vec(&needle, self.term_width as usize);
                    selection = selection.clamp(0, matches.len());

                    if matches.is_empty() {
                        self.render_decl()?;
                        return Ok(State::Continue(Mode::VariableDecl, None));
                    }

                    self.input_buf.drain(INSERTION_PREFIX.len()..);
                    self.input_buf.extend(matches[selection].0.chars());

                    if self.input_buf.starts_with(INSERTION_PREFIX) {
                        self.input_buf.push(':');
                        self.input_buf
                            .extend(matches[selection].1.to_string().chars());
                    }

                    self.cur_selection = None;
                    self.render_decl()?;
                    self.cursor_to(self.input_buf.len())?;

                    return Ok(State::Continue(Mode::VariableDecl, None));
                }

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
            Key::Char('C') => {
                self.input_buf.drain(4..);
                self.cur_selection = None;
                self.render_decl()?;
                self.cursor_to(self.input_buf.len())?;
            }
            Key::Char(ch) if ch == 'V' || ch == 'D' => {
                let prefix;
                let other;

                if ch == 'V' {
                    prefix = INSERTION_PREFIX;
                    other = DELETION_PREFIX;
                } else {
                    prefix = DELETION_PREFIX;
                    other = INSERTION_PREFIX;
                };

                if self.input_buf.starts_with(prefix) {
                    self.render_decl()?;
                    return Ok(State::Continue(Mode::VariableDecl, None));
                }

                if self.input_buf.starts_with(other) {
                    self.input_buf[..prefix.len()].copy_from_slice(prefix);
                    self.render_decl()?;

                    return Ok(State::Continue(Mode::VariableDecl, None));
                }

                let old_buf_len = self.input_buf.len();
                let mut buf: Vec<char> = prefix.to_vec();
                buf.extend(self.input_buf.iter().skip_while(|ch| ch.is_whitespace()));
                self.input_buf = buf;

                self.render_decl()?;
                self.cursor_to(if self.input_buf.len() < old_buf_len {
                    self.cur_col.clamp(0, self.input_buf.len())
                } else {
                    self.cur_col + self.input_buf.len() - old_buf_len
                })?;
            }
            Key::Char(ch) => {
                self.input_buf.insert(self.cur_col, ch);
                self.cur_selection = None;
                self.render_decl()?;
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
            }
            Key::Backspace => {
                assert_eq!(INSERTION_PREFIX.len(), DELETION_PREFIX.len());
                if self.cur_col <= INSERTION_PREFIX.len() {
                    return Ok(State::Continue(
                        Mode::VariableDecl,
                        Some(String::from("Nothing to delete!")),
                    ));
                }

                self.cur_col -= 1;
                self.input_buf.remove(self.cur_col);

                self.cur_selection = None;
                self.render_decl()?;
                self.cursor_to(self.cur_col)?;
            }
            Key::ArrowUp => {
                self.cur_selection = match self.cur_selection {
                    Some(selection) if selection > 0 => Some(selection - 1),
                    Some(0) => None,
                    _ => None,
                };
                self.render_decl()?;
            }
            Key::ArrowDown => {
                self.cur_selection = match self.cur_selection {
                    Some(selection) => Some(selection + 1),
                    None => Some(0),
                };
                self.render_decl()?;
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
        }

        Ok(State::Continue(Mode::VariableDecl, None))
    }

    pub fn handle_select(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => return Ok(State::ForceQuit),
            Key::Char('q') | Key::Escape => {
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

                if matches.is_empty() || self.cur_selection.unwrap() >= matches.len() {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(String::from("No matching variable found!")),
                    ));
                }

                return Ok(State::Continue(
                    Mode::NormalCarry(matches[self.cur_selection.unwrap()].1),
                    Some(format!(
                        "Inserted {}",
                        matches[self.cur_selection.unwrap()].0
                    )),
                ));
            }
            Key::Char(ch) => {
                self.input_buf.insert(self.cur_col, ch);
                self.cur_selection = Some(0);
                self.render_select()?;
                self.cursor_to(min(self.cur_col + 1, self.input_buf.len()))?;
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

                self.cur_selection = Some(0);
                self.render_select()?;
                self.cursor_to(self.cur_col)?;
            }
            Key::ArrowUp => {
                self.cur_selection = if self.cur_selection.unwrap() == 0 {
                    Some(0usize)
                } else {
                    Some(self.cur_selection.unwrap() - 1)
                };
                self.render_select()?;
            }
            Key::ArrowDown => {
                self.cur_selection = Some(self.cur_selection.unwrap() + 1);
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
        }

        Ok(State::Continue(Mode::VariableSelect, None))
    }

    pub fn render_decl(&mut self) -> Result<(), Box<dyn Error>> {
        if self.fresh {
            self.init_decl();
        }

        let mut out = BufWriter::new(stdout().lock());
        if !opts::DEBUG {
            write!(out, "{HOME}{ERASE_FROM_CURSOR}")?;
        }

        if self.input_buf.starts_with(INSERTION_PREFIX) {
            writeln!(
                out,
                "{}",
                format!(
                    "Accepted format: \
                    \x1b[2m{}\x1b[0m\
                    \x1b[32mstd_gravity\x1b[33m:\x1b[34m9.80665\x1b[0m",
                    INSERTION_PREFIX.iter().collect::<String>(),
                )
            )?;
        } else {
            writeln!(
                out,
                "{}",
                format!(
                    "Accepted format: \
                    \x1b[2m{}\x1b[0m\
                    \x1b[32mstd_gravity\x1b[0m",
                    DELETION_PREFIX.iter().collect::<String>(),
                )
            )?;
        }

        write!(out, "{}", self.input_buf.iter().collect::<String>())?;
        let needle: String = self
            .input_buf
            .iter()
            .skip(INSERTION_PREFIX.len())
            .take_while(|ch| **ch != ':')
            .collect();
        self.render_matches(needle, &mut out)?;
        out.flush()?;

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

        let needle: String = self.input_buf.iter().collect();
        write!(out, "Enter the variable name below:\n{needle}")?;
        self.render_matches(needle, &mut out)?;
        out.flush()?;

        Ok(())
    }

    pub fn render_matches(
        &mut self,
        needle: String,
        out: &mut BufWriter<StdoutLock>,
    ) -> Result<(), Box<dyn Error>> {
        if self.term_height < 3 {
            out.flush()?;
            return Ok(());
        }

        let width = self.term_width as usize;
        let replace: String = HIGHLIGHT_COLOR.to_owned() + &needle + RESET_COLORS;
        let matches: Vec<(String, f64, usize, usize)> = self
            .match_vec(&needle, width)
            .iter()
            .map(|(name, val, height, acc_height)| {
                (name.replace(&needle, &replace), *val, *height, *acc_height)
            })
            .collect();

        if matches.is_empty() {
            write!(out, "\x1b[2;{}H", self.cur_col + 1)?;
            out.flush()?;

            return Ok(());
        }

        let mut rem_height = self.term_height as usize - 2;
        self.cur_selection = match self.cur_selection {
            Some(selection) => Some(min(selection + 1, matches.len()) - 1),
            None => None,
        };

        let mut first_idx = 0;
        let mut ceiling = if let Some(selection) = self.cur_selection {
            matches[selection].3
        } else {
            0
        };

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
            if let Some(selection) = self.cur_selection
                && selection == i
            {
                write!(out, "=> \x1b[1m")?;
            }

            write!(out, "{} = {}\x1b[0m", matches[i].0, matches[i].1)?;
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
        self.cur_selection = None;
        self.fresh = false;
    }

    fn init_select(&mut self) {
        self.input_buf.clear();
        self.cur_col = 0;
        self.cur_selection = Some(0);
        self.fresh = false;
    }

    fn match_vec(
        &self,
        needle: &str,
        width: usize,
    ) -> Vec<(String, f64, usize, usize)> {
        let mut acc = 0;

        self.var_map
            .map
            .iter()
            .filter(|(key, _)| key.contains(needle))
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
