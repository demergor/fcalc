use core::{error, fmt};
use std::{
    cmp::{self, max, min},
    error::Error,
    fmt::Display,
    io::{self, BufWriter, StdoutLock, Write, stdout},
    ops,
};

use crate::{
    functions::{
        self, COEFF_DELIM, DELETION_PREFIX, FuncMap, Function, FunctionError,
        HandleResult, INSERTION_PREFIX,
    },
    io::{
        Key, State,
        io_handler::{
            ERASE_FROM_CURSOR, ERR_COLOR, HIDE_CURSOR, HIGHLIGHT_COLOR, HOME, IoError,
            Mode, RESET, RESET_COLORS, SHOW_CURSOR, SUCCESS_COLOR, SYNTAX_ERR_MSG,
        },
    },
    operation::Operation,
    opts,
    terminal::Terminal,
};

const DECL_HINT: &str = "<F> = declare a new function, <D> = delete existing function";
const NAME_MIN_LEN: usize = INSERTION_PREFIX.len();
const LINE_MIN_LEN: usize = 1;

pub struct FunctionHandler {
    fresh: bool,
    name_finished: bool,

    func_map: FuncMap,
    funcs: Vec<Function>,
    lines: Vec<Vec<char>>,
    func_name_buf: Vec<char>,
    cur_col: usize,
    cur_selection: Option<usize>,

    term_width: u16,
    term_height: u16,
}

impl FunctionHandler {
    pub fn new(bounds: &Terminal) -> Result<FunctionHandler, IoError> {
        Ok(FunctionHandler {
            fresh: true,
            name_finished: false,

            func_map: FuncMap::new()?,
            funcs: vec![Function::default()],
            lines: vec![Vec::new()],
            func_name_buf: functions::INSERTION_PREFIX.to_vec(),
            cur_col: 0,
            cur_selection: None,

            term_width: bounds.width,
            term_height: bounds.height,
        })
    }

    pub fn handle_decl(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('Q') => {
                self.fresh = true;
                return Ok(State::Continue(
                    Mode::Normal,
                    Some(String::from("Cancelled function declaration")),
                ));
            }
            Key::Escape => {
                if self.name_finished {
                    self.name_finished = false;
                    self.render_decl()?;
                    self.reset_cursor_pos()?;
                } else {
                    self.fresh = true;
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(String::from("Cancelled function declaration")),
                    ));
                }
            }
            Key::Char('H') => {
                if !self.name_finished {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(DECL_HINT.to_owned()),
                    ));
                }

                return Ok(State::Continue(
                    Mode::FunctionDecl,
                    Some(String::from(
                        "Define your formula using the 'normal' fold expression syntax",
                    )),
                ));
            }
            Key::Enter => {
                if let Some(mut selection) = self.cur_selection {
                    let needle: String = self
                        .func_name_buf
                        .iter()
                        .skip(INSERTION_PREFIX.len())
                        .take_while(|&&ch| ch != ':')
                        .collect();
                    let matches = self.match_vec(&needle, self.term_width as usize)?;
                    selection = selection.clamp(0, matches.len());

                    if matches.is_empty() {
                        self.render_decl()?;
                        return Ok(State::Continue(Mode::FunctionDecl, None));
                    }

                    self.func_name_buf.drain(INSERTION_PREFIX.len()..);
                    self.func_name_buf.extend(matches[selection].0.chars());

                    if self.func_name_buf.starts_with(INSERTION_PREFIX) {
                        self.name_finished = true;

                        self.funcs.clear();
                        self.funcs.push(matches[selection].1.clone());
                        self.lines.drain(1..);
                        self.funcs[0].write_chars(&mut self.lines[0])?;

                        self.render_decl()?;
                        self.reset_cursor_pos()?;
                    }
                }

                let mut submission: String = self.func_name_buf.iter().collect();
                if submission.starts_with(DELETION_PREFIX) {
                    self.fresh = true;
                    return match self.func_map.handle(submission) {
                        HandleResult::DeletionSuccess => Ok(State::Continue(
                            Mode::Normal,
                            Some(format!(
                                "{SUCCESS_COLOR}\
                                Function deletion successful!\
                                {RESET}"
                            )),
                        )),
                        HandleResult::DeletionFail => Ok(State::Continue(
                            Mode::Normal,
                            Some(format!(
                                "{ERR_COLOR}\
                                Function doesn't exist!\
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
                        _ => unreachable!(),
                    };
                }

                if !self.name_finished {
                    if self.func_name_buf.len() == INSERTION_PREFIX.len() {
                        return Ok(State::Continue(
                            Mode::FunctionDecl,
                            Some(String::from("Please provide a function name!")),
                        ));
                    }

                    self.name_finished = true;

                    self.funcs.clear();
                    self.lines.clear();

                    self.funcs.push(Function::default());
                    let buf = self.lines.push_mut(Vec::new());
                    self.funcs[0].write_chars(buf)?;

                    self.render_decl()?;
                    self.reset_cursor_pos()?;

                    return Ok(State::Continue(Mode::FunctionDecl, None));
                }

                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                if self.funcs.len() < 2 {
                    let (cur_func, cur_line) = self.cur_pair()?;
                    cur_func.write_chars(cur_line)?;
                    submission.push(':');
                    submission.extend(cur_line.iter());

                    return match self.func_map.handle(submission) {
                        HandleResult::Insertion => Ok(State::Continue(
                            Mode::Normal,
                            Some(String::from("Function declaration successful!")),
                        )),
                        HandleResult::Update => Ok(State::Continue(
                            Mode::Normal,
                            Some(String::from("Function update successful!")),
                        )),
                        HandleResult::GenericFail => Ok(State::Continue(
                            Mode::Normal,
                            Some(String::from("Malformed input!")),
                        )),
                        _ => unreachable!(),
                    };
                }

                let mut child_func =
                    self.funcs.pop().ok_or(InternalStateError::NoFunction)?;
                self.lines.pop();
                child_func.reduce();

                let (parent_func, parent_line) = self.cur_pair()?;
                parent_func.change_operands(&child_func);
                parent_func.write_chars(parent_line)?;

                self.render_decl()?;
                self.reset_cursor_pos()?;
            }
            Key::Char('C') => {
                if self.name_finished {
                    self.funcs.clear();
                    self.lines.clear();

                    self.funcs.push(Function::default());
                    self.lines.push(Vec::new());

                    self.render_decl()?;
                    self.reset_cursor_pos()?;
                } else {
                    self.func_name_buf.drain(INSERTION_PREFIX.len()..);
                    self.render_decl()?;
                    self.reset_cursor_pos()?;
                }
            }
            Key::Char('R') if self.name_finished => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                let cur_func = self.cur_pair()?.0;
                cur_func.reverse();

                self.render_decl()?;
                self.reset_cursor_pos()?;
            }
            Key::Char('E') if self.name_finished => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                let cur_func = self.cur_pair()?.0;
                let Some(new_func) = cur_func.new_from_cur_operands() else {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                };

                self.funcs.push(new_func);
                self.lines.push(Vec::new());

                self.render_decl()?;
                self.reset_cursor_pos()?;
            }
            Key::Char(ch)
                if let Ok(op) = Operation::try_from(ch)
                    && self.name_finished =>
            {
                let cur_col = self.cur_col;
                let (cur_func, cur_line) = self.cur_pair()?;
                cur_line[0] = op.as_char();

                if cur_func.reparse(cur_line, cur_col).is_err() {
                    self.reeval()?;
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(format!("Wrong usage of operator '{ch}'")),
                    ));
                }

                self.render_decl()?;
                self.reset_cursor_pos()?;
            }
            Key::Char(ch) if self.name_finished && ch == 'W' || ch == 'B' => {
                let get_operand_range = if ch == 'W' {
                    Function::next_operands
                } else {
                    Function::previous_operands
                };

                let (cur_func, cur_line) = self.cur_pair()?;
                let op_range = get_operand_range(cur_func);

                if !op_range.is_empty() {
                    return Ok(State::Continue(Mode::FunctionDecl, None));
                };

                let op_range = operands_range(cur_line, op_range);
                if op_range.is_empty() {
                    return Ok(State::Continue(Mode::FunctionDecl, None));
                }

                self.cursor_to(op_range.end - 1)?;
            }
            Key::Char(ch) => {
                let mut cur_col;
                let buf = if !self.name_finished {
                    cur_col = self.cur_col;
                    &mut self.func_name_buf
                } else {
                    cur_col = self.cur_col;
                    self.cur_pair()?.1
                };

                cur_col = min(cur_col, buf.len());
                buf.insert(cur_col, ch);
                let buflen = buf.len();
                cur_col += 1;

                if ch.is_whitespace()
                    || ch == COEFF_DELIM
                    || ch == '.'
                    || self.syntax_error()?
                {
                    let cur_line: String = if self.name_finished {
                        self.cur_pair()?.1.iter().collect()
                    } else {
                        self.func_name_buf.iter().collect()
                    };
                    print!("\r\x1b[2K{}", cur_line);
                    self.reeval()?;
                    self.cursor_to(cur_col)?;

                    return Ok(State::Continue(Mode::FunctionDecl, None));
                }

                self.render_decl()?;
                cur_col = cur_col.clamp(
                    if self.name_finished {
                        1
                    } else {
                        INSERTION_PREFIX.len()
                    },
                    max(buflen, 2),
                );
                self.cursor_to(cur_col)?;
            }
            Key::Backspace => {
                let cur_col = self.cur_col;
                let min_len;
                let buf = if self.name_finished {
                    min_len = LINE_MIN_LEN;
                    self.cur_pair()?.1
                } else {
                    min_len = NAME_MIN_LEN;
                    &mut self.func_name_buf
                };

                if cur_col <= min_len {
                    return Ok(State::Continue(
                        Mode::FunctionDecl,
                        Some(String::from("Nothing to delete!")),
                    ));
                }

                let cur_col = min(cur_col, buf.len()) - 1;
                buf.remove(cur_col);

                if self.name_finished && self.syntax_error()? {
                    let cur_line: String = self.cur_pair()?.1.iter().collect();
                    print!("\r\x1b[2K{}", cur_line);
                    self.reeval()?;
                    self.cursor_to(cur_col)?;

                    return Ok(State::Continue(Mode::FunctionDecl, None));
                }

                self.render_decl()?;
                self.cursor_to(cur_col)?;
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
                let cur_col = if self.name_finished {
                    min(self.cur_col + 1, self.cur_pair()?.1.len())
                } else {
                    min(self.cur_col + 1, self.func_name_buf.len())
                };

                self.cursor_to(cur_col)?;

                return Ok(State::Continue(Mode::FunctionDecl, None));
            }
            Key::ArrowLeft => {
                let leftmost_pos = if self.name_finished { 1 } else { 3 };
                self.cursor_to(if self.cur_col <= leftmost_pos {
                    1
                } else {
                    self.cur_col - 1
                })?;

                return Ok(State::Continue(Mode::FunctionDecl, None));
            }
        }

        Ok(State::Continue(Mode::FunctionDecl, None))
    }

    pub fn render_decl(&mut self) -> Result<(), Box<dyn Error>> {
        if self.fresh {
            self.init_decl()?;
        } else {
            self.flatten();
            self.ripple_update()?;
        }

        let mut out = BufWriter::new(stdout().lock());
        if !opts::DEBUG {
            write!(out, "{HIDE_CURSOR}{HOME}{ERASE_FROM_CURSOR}")?;
        }

        let func_name: String = self
            .func_name_buf
            .iter()
            .skip(INSERTION_PREFIX.len())
            .collect();

        if !self.name_finished {
            let line: String = self.func_name_buf.iter().collect();
            write!(out, "Enter the function name below:\n{line}")?;

            self.render_matches(func_name, &mut out)?;
            write!(out, "\x1b[{}G{SHOW_CURSOR}", self.cur_col + 1)?;
            out.flush()?;

            return Ok(());
        }

        let param_list: String = self.cur_pair()?.0.param_names().join(", ");
        write!(out, "Define {func_name}({param_list}) below:\n")?;

        let width = self.term_width as usize;
        let mut rem_height = self.term_height as usize
            - (param_list.chars().count() + width - 1) / width
            + 1;
        let to_skip: usize = {
            let mut idx = self.lines.len();
            while idx > 0 && rem_height > 0 {
                idx -= 1;
                rem_height -= (self.lines[idx].len() + width - 1) / width;
            }

            idx
        };

        let func_it = self.funcs.iter().skip(to_skip);
        let line_it = self.lines.iter().skip(to_skip);
        let mut it = func_it.zip(line_it).peekable();
        let mut first = true;

        while let Some((func, line)) = it.next() {
            let cur_op_range = func.cur_operands();
            if cur_op_range.is_empty() {
                let func_str: String = line.clone().iter().collect();
                write!(out, "{}{}", if first { "" } else { "\r\n" }, func_str)?;
                first = false;
                continue;
            };

            let mut line_cp = line.clone().to_vec();
            let highlight_range = operands_range(&line_cp, func.cur_operands());

            if !highlight_range.is_empty() && it.peek().is_some() {
                line_cp.splice(highlight_range.end..highlight_range.end, RESET.chars());
                line_cp.splice(
                    highlight_range.start..highlight_range.start,
                    HIGHLIGHT_COLOR.chars(),
                );
            }

            let func_str: String = line_cp.iter().collect();
            write!(out, "{}{}", if first { "" } else { "\r\n" }, func_str)?;
            first = false;
        }

        writeln!(out)?;
        self.render_matches(func_name, &mut out)?;
        write!(out, "\x1b[{}G{SHOW_CURSOR}", self.cur_col + 1)?;
        out.flush()?;

        Ok(())
    }

    fn render_matches(
        &mut self,
        needle: String,
        out: &mut BufWriter<StdoutLock>,
    ) -> Result<(), Box<dyn error::Error>> {
        if self.term_height < 3 {
            out.flush()?;
            return Ok(());
        }

        let width = self.term_width as usize;
        let replace: String = HIGHLIGHT_COLOR.to_owned() + &needle + RESET_COLORS;
        let matches: Vec<(String, Function, usize, usize)> = self
            .match_vec(&needle, width)?
            .iter()
            .map(|(name, val, height, acc_height)| {
                (
                    name.replace(&needle, &replace),
                    val.clone(),
                    *height,
                    *acc_height,
                )
            })
            .collect();

        if matches.is_empty() {
            write!(out, "\x1b[2;{}H", self.cur_col + 1)?;
            out.flush()?;

            return Ok(());
        }

        let mut rem_height = self.term_height as usize - 2;
        self.cur_selection = match self.cur_selection {
            Some(selection) => Some(cmp::min(selection + 1, matches.len()) - 1),
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

            write!(out, "{} = {}{RESET}", matches[i].0, matches[i].1)?;
            rem_height -= matches[i].2;
        }

        write!(out, "\x1b[2;{}H", self.cur_col + 1)?;
        out.flush()?;

        Ok(())
    }

    fn cur_pair(
        &mut self,
    ) -> Result<(&mut Function, &mut Vec<char>), InternalStateError> {
        if self.funcs.len() != self.lines.len() {
            return Err(InternalStateError::OutOfSync);
        }

        let cur_func = self
            .funcs
            .last_mut()
            .ok_or(InternalStateError::NoFunction)?;
        let cur_line = self
            .lines
            .last_mut()
            .ok_or(InternalStateError::NoFunction)?;

        Ok((cur_func, cur_line))
    }

    fn ripple_update(&mut self) -> Result<(), InternalStateError> {
        if self.funcs.len() != self.lines.len() {
            return Err(InternalStateError::OutOfSync);
        }

        let mut it = self.funcs.iter_mut().rev().zip(self.lines.iter_mut().rev());
        let (mut cur_func, mut cur_line) =
            it.next().ok_or(InternalStateError::NoFunction)?;
        cur_func.write_chars(&mut cur_line)?;

        while let Some((next_func, next_line)) = it.next() {
            next_func.change_operands(cur_func);
            next_func.write_chars(next_line)?;
            cur_func = next_func;
        }

        Ok(())
    }

    fn flatten(&mut self) {
        let len = self.funcs.len();
        if len >= 3
            && self.funcs[len - 1] == self.funcs[len - 2]
            && self.funcs[len - 1] == self.funcs[len - 3]
        {
            self.funcs.pop();
            self.lines.pop();
        }
    }

    fn match_vec(
        &mut self,
        needle: &String,
        width: usize,
    ) -> Result<Vec<(String, Function, usize, usize)>, InternalStateError> {
        let mut acc = 0;
        Ok(self
            .func_map
            .map
            .iter()
            .filter(|(key, _)| key.contains(needle))
            .map(|(key, val)| {
                let height = ((key.chars().count() + val.to_string().chars().count())
                    - 1)
                    / width
                    + 1;
                acc += height;
                (key.clone(), val.clone(), height, acc)
            })
            .collect())
    }

    fn init_decl(&mut self) -> Result<(), Box<dyn Error>> {
        self.funcs.clear();
        self.lines.clear();
        self.func_name_buf.clear();

        let root_func = Function::default();
        let mut buf = Vec::new();
        root_func.write_chars(&mut buf)?;

        self.func_name_buf.extend(INSERTION_PREFIX.iter());
        self.funcs.push(root_func);
        self.lines.push(buf);

        println!("{}", self.func_name_buf.iter().collect::<String>());

        self.cur_col = self.func_name_buf.len();
        self.name_finished = false;
        self.fresh = false;

        Ok(())
    }

    fn reeval(&mut self) -> Result<(), Box<dyn error::Error>> {
        let cur_col = self.cur_col;
        let (cur_func, cur_line) = self.cur_pair()?;

        match cur_func.reparse(cur_line, cur_col) {
            Err(FunctionError::ParseError(pos, ch)) => {
                print!(
                    "\x1b[{}G{ERR_COLOR}{ch}{RESET}\x1b[{}G",
                    pos + 1,
                    self.cur_col + 1
                );
                stdout().flush()?
            }
            _ => (),
        }

        Ok(())
    }

    fn syntax_error(&mut self) -> Result<bool, InternalStateError> {
        let cur_col = self.cur_col;
        let (cur_func, cur_line) = self.cur_pair()?;
        Ok(cur_func.reparse(cur_line, cur_col).is_err())
    }

    fn reset_cursor_pos(&mut self) -> Result<(), Box<dyn error::Error>> {
        if !self.name_finished {
            self.cursor_to(self.func_name_buf.len())?;
            return Ok(());
        }

        let (cur_func, cur_line) = self.cur_pair()?;
        let op_range = operands_range(&cur_line, cur_func.cur_operands());

        if !op_range.is_empty() {
            self.cur_col = op_range.end;
        } else {
            self.cur_col = self.cur_col.clamp(0, self.cur_pair()?.1.len());
        }

        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }

    fn cursor_to(&mut self, pos: usize) -> Result<(), io::Error> {
        self.cur_col = pos;
        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }
}

#[derive(Debug)]
enum InternalStateError {
    OutOfSync,
    NoFunction,
    WriteCharsFailed(fmt::Error),
}

impl From<fmt::Error> for InternalStateError {
    fn from(err: fmt::Error) -> Self {
        Self::WriteCharsFailed(err)
    }
}

impl Display for InternalStateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalStateError::OutOfSync => write!(
                f,
                "Functions and their string representations are out of sync!"
            ),
            InternalStateError::NoFunction => {
                write!(f, "`FunctionHandler` doesn't have a root function!")
            }
            InternalStateError::WriteCharsFailed(err) => {
                write!(f, "Writing chars failed: {err}")
            }
        }
    }
}

impl error::Error for InternalStateError {}

fn operands_range(slice: &[char], idx_range: ops::Range<usize>) -> ops::Range<usize> {
    if idx_range.is_empty() {
        return idx_range;
    }

    let mut cur_op_idx = 0;
    let mut in_operand = false;
    let mut range = 0..0;

    for (mut slice_idx, ch) in slice.iter().skip(1).enumerate() {
        slice_idx += 1;
        let previously_in_operand = in_operand;

        match ch {
            ch if ch.is_whitespace() => in_operand = false,
            _ if in_operand => (),
            &ch if ch.is_ascii_digit()
                || ch == functions::COEFF_DELIM
                || ch == Operation::Subtraction.as_char() =>
            {
                in_operand = true
            }
            _ => unreachable!(),
        }

        if !previously_in_operand && in_operand {
            cur_op_idx += 1;
        }

        if range.is_empty() && in_operand && cur_op_idx - 1 == idx_range.start {
            range = slice_idx..slice_idx + 1;
        }

        if !in_operand && cur_op_idx == idx_range.end {
            range.end = slice_idx;
            return range;
        }
    }

    range.end = slice.len();
    range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operands_range() {
        let mut test_func = Function::default();
        let test_input: Vec<char> =
            "/ 1'x.i'x 2.1'!n 3'var1 0'z 39'0 -69.67'... -1521'va-l -6666.1'x.i "
                .chars()
                .collect();
        test_func
            .reparse(&test_input, 0)
            .expect("(Re-)parsing failed unexpectedly!");

        let mut buf = Vec::new();
        test_func
            .write_chars(&mut buf)
            .expect("Writing chars failed unexpectedly!");

        assert_eq!(test_input[1..], buf[1..]);

        let actual = operands_range(&buf, 2..8);
        let expected_start = 17;
        let expected_end = test_input.len() - 1;

        assert_eq!(expected_start..expected_end, actual);
    }
}
