use core::{error, fmt};
use std::{
    cmp,
    error::Error,
    fmt::Display,
    io::{BufWriter, StdoutLock, Write, stdout},
    ops,
};

use crate::{
    functions::{self, FuncMap, Function},
    io::io_handler::{
        ERASE_FROM_CURSOR, HIDE_CURSOR, HIGHLIGHT_COLOR, HOME, RESET_COLORS,
        SHOW_CURSOR,
    },
    operation::Operation,
    opts,
    terminal::Terminal,
};

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
    pub fn new(bounds: &Terminal) -> Result<FunctionHandler, Box<dyn Error>> {
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

        let func_name = self.func_name_buf.iter().collect();
        if self.name_finished {
            write!(out, "Enter the function name below:\n{func_name}")?;
            self.render_matches(func_name, &mut out)?;
            out.flush()?;

            return Ok(());
        }

        let param_list: String = self.cur_pair()?.0.param_names().join(", ");
        let cur_line: String = self.cur_pair()?.1.iter().collect();
        write!(out, "Define {func_name}({param_list}) below:\n{cur_line}")?;

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

            let mut line_cp = line.clone();
            // TODO: Continue implementation from here
            // let start_pos = operands_range(slice, idx_range);
        }

        write!(out, "{SHOW_CURSOR}")?;
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

    // TODO: Figure out how to handle the functions
    // Maybe have the name and function separated?
    // "VAR some_func_name" -> define function separately
    fn init_decl(&mut self) -> Result<(), Box<dyn Error>> {
        self.funcs.clear();
        self.lines.clear();
        self.func_name_buf.clear();

        let root_func = Function::default();
        let mut buf = Vec::new();
        root_func.write_chars(&mut buf)?;

        let mut root_line = functions::INSERTION_PREFIX.to_vec();
        root_line.extend(buf.iter());

        self.funcs.push(root_func);
        self.lines.push(root_line);

        self.cur_col = self.cur_pair()?.1.len();
        self.name_finished = false;
        self.fresh = false;

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

// TODO: Test this
fn operands_range(slice: &[char], idx_range: ops::Range<usize>) -> ops::Range<usize> {
    let mut cur_op_idx = 0;
    let mut in_operand = false;
    let mut range = 0..0;

    for (mut slice_idx, ch) in slice.iter().skip(1).enumerate() {
        slice_idx += 1;
        match ch {
            ch if ch.is_whitespace() => in_operand = false,
            _ if in_operand => (),
            &ch if ch.is_ascii_digit()
                || ch == functions::COEFF_DELIM
                || ch == Operation::Subtraction.as_char() =>
            {
                cur_op_idx += 1
            }
            _ => unreachable!(),
        }

        if range.is_empty() && cur_op_idx == idx_range.start {
            range = slice_idx..slice_idx + 1;
        }

        if cur_op_idx == idx_range.end {
            range.end = slice_idx;
            return range;
        }
    }

    range.end = slice.len();
    range
}
