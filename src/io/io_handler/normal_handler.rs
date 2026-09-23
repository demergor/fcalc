use core::fmt;
use std::{
    cmp::{max, min},
    error::Error,
    fmt::Display,
    io::{self, stdout, BufWriter, Write},
};

use crate::{
    fold_expr::{FoldExpr, FoldExprError, ParseFoldExprError},
    io::{
        io_handler::{
            IoError, Mode, ERASE_FROM_CURSOR, ERR_COLOR, HIDE_CURSOR, HIGHLIGHT_COLOR,
            RESET, SHOW_CURSOR,
        },
        Key, State,
    },
    operation::{Operation, OperationExecutionError},
    opts,
    terminal::Terminal,
};

const SYNTAX_ERR_MSG: &str = "\x1b[41mResolve syntax errors first!\x1b[0m";

pub struct NormalHandler {
    fold_exprs: Vec<FoldExpr>,
    lines: Vec<Vec<char>>,
    cur_col: usize,
    term_width: u16,
    term_height: u16,
}

impl NormalHandler {
    pub fn new(bounds: &Terminal) -> Result<NormalHandler, IoError> {
        let root_expr = FoldExpr::default();
        let mut buf = Vec::new();

        if root_expr.write_chars(&mut buf).is_err() {
            return Err(IoError::HandlerConstructionError);
        };

        let buf_len = buf.len();

        Ok(NormalHandler {
            fold_exprs: vec![root_expr],
            lines: vec![buf],
            cur_col: buf_len,
            term_width: bounds.width,
            term_height: bounds.height,
        })
    }

    pub fn handle(
        &mut self,
        key: Key,
    ) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('=') => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                self.ripple_update()?;
                self.fold_exprs.truncate(1);
                self.lines.truncate(1);

                let (cur_expr, cur_line) = self.cur_pair()?;
                cur_expr.collapse()?;
                cur_expr.write_chars(cur_line)?;

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Enter => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                if self.fold_exprs.len() < 2 {
                    let (cur_expr, cur_line) = self.cur_pair()?;
                    cur_expr.collapse()?;
                    cur_expr.write_chars(cur_line)?;
                } else {
                    let child_expr = self.fold_exprs.pop();
                    self.lines.pop();

                    let (parent_expr, parent_line) = self.cur_pair()?;
                    parent_expr.change_operand(child_expr.unwrap().evaluate()?);
                    parent_expr.write_chars(parent_line)?;
                }

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char('q') => {
                return Ok(State::Quit(
                    self.fold_exprs
                        .first()
                        .ok_or(InternalStateError::NoFoldExpr)?
                        .evaluate()
                        .map_err(|_| InternalStateError::FoldExprError)?,
                ));
            }
            Key::Char('c') => {
                *self.cur_pair()?.0 = FoldExpr::default();
                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char('C') => {
                self.fold_exprs.clear();
                self.lines.clear();

                self.fold_exprs = vec![FoldExpr::default()];
                self.lines = vec![Vec::new()];

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char('r') => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                let cur_expr = self.cur_pair()?.0;
                cur_expr.reverse();

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char('e') => {
                if self.syntax_error()? {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                }

                let cur_expr = self.cur_pair()?.0;
                let Some(new_fold_expr) = cur_expr.new_from_cur_operand() else {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(SYNTAX_ERR_MSG.to_owned()),
                    ));
                };

                self.fold_exprs.push(new_fold_expr);
                self.lines.push(Vec::new());

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                let cur_col = self.cur_col;
                let (cur_expr, cur_line) = self.cur_pair()?;
                cur_line[0] = op.as_char();

                if let Err(err) = cur_expr.reparse(cur_line, cur_col) {
                    self.reeval()?;
                    return Ok(State::Continue(Mode::Normal, Some(err.to_string())));
                }

                self.render()?;
                self.reset_cursor_pos();
            }
            Key::Char(ch) if ch == 'w' || ch == 'b' => {
                let get_operand_idx = if ch == 'w' {
                    FoldExpr::next_operand
                } else {
                    FoldExpr::previous_operand
                };

                let (cur_expr, cur_line) = self.cur_pair()?;
                let Some(op_idx) = get_operand_idx(cur_expr) else {
                    return Ok(State::Continue(Mode::Normal, None));
                };

                let Some((_, op_end)) = nth_operand_pos(cur_line, op_idx) else {
                    return Ok(State::Continue(Mode::Normal, None));
                };

                self.cursor_to(op_end - 1)?;
            }
            Key::Char('V') => return Ok(State::Continue(Mode::VariableDecl, None)),
            Key::Char('v') => return Ok(State::Continue(Mode::VariableSelect, None)),
            Key::Char(ch) => {
                let mut cur_col = self.cur_col;
                self.cur_pair()?.1.insert(cur_col, ch);
                cur_col = if cur_col < 3 { 3 } else { cur_col + 1 };

                if ch.is_whitespace() || ch == '.' || self.syntax_error()? {
                    let cur_line = self.cur_pair()?.1.iter().collect::<String>();
                    print!("\r\x1b[2K{}", cur_line);
                    self.reeval()?;

                    return Ok(State::Continue(Mode::Normal, None));
                }

                self.render()?;
                cur_col = cur_col.clamp(2, max(self.cur_pair()?.1.len() - 1, 2));
                self.cursor_to(cur_col)?;
            }
            Key::Backspace => {
                if self.cur_col < 2 {
                    return Ok(State::Continue(
                        Mode::Normal,
                        Some(String::from("Nothing to delete!")),
                    ));
                }

                let cur_col = self.cur_col - 1;
                self.cur_pair()?.1.remove(cur_col);
                self.cur_col = cur_col;

                if !self.syntax_error()? {
                    let cur_line = self.cur_pair()?.1.iter().collect::<String>();
                    print!("\r\x1b[2K{}", cur_line);
                    self.reeval()?;

                    return Ok(State::Continue(Mode::Normal, None));
                }

                self.render()?;
            }
            Key::ArrowRight => {
                let cur_col = min(self.cur_col + 1, self.cur_pair()?.1.len());
                self.cursor_to(cur_col);

                return Ok(State::Continue(Mode::Normal, None));
            }
            Key::ArrowLeft => {
                self.cursor_to(if self.cur_col <= 1 {
                    1
                } else {
                    self.cur_col - 1
                });

                return Ok(State::Continue(Mode::Normal, None));
            }
            _ => (),
        }

        Ok(State::Continue(Mode::Normal, None))
    }

    pub fn render(&mut self) -> Result<(), RenderError> {
        self.flatten();
        self.ripple_update()?;
        let to_skip: usize = {
            let mut idx = self.lines.len();
            let mut height = self.term_height - 1;

            while idx > 0 && height > 0 {
                idx -= 1;
                if self.lines[idx].is_empty() {
                    panic!("Empty string representation of a fold expression!");
                }

                height -= (self.lines[idx].len() as u16 - 1) / self.term_width + 1;
            }

            idx
        };

        let mut out = BufWriter::new(std::io::stdout().lock());
        if !opts::DEBUG {
            write!(out, "\x1b[H{HIDE_CURSOR}{ERASE_FROM_CURSOR}")?;
        } else {
            writeln!(out)?;
        }

        writeln!(
            out,
            "\x1b[1m\x1b[4m{}{RESET}",
            self.fold_exprs[0].evaluate()?
        )?;

        let expr_it = self.fold_exprs.iter().skip(to_skip);
        let line_it = self.lines.iter().skip(to_skip);
        let mut pair_it = expr_it.zip(line_it).peekable();
        let mut first = true;

        while let Some((expr, line)) = pair_it.next() {
            let Some(operand_idx) = expr.cur_operand() else {
                let fold_expr_str: String = line.clone().iter().collect();
                write!(out, "{}{}", if first { "" } else { "\r\n" }, fold_expr_str)?;
                first = false;
                continue;
            };

            let mut line_cp = line.clone();
            let pos_opt = nth_operand_pos(&line_cp, operand_idx);

            if pos_opt.is_some() && pair_it.peek().is_some() {
                let (start, end) = pos_opt.unwrap();
                for ch in HIGHLIGHT_COLOR.chars().rev() {
                    line_cp.insert(start, ch)
                }

                let end = end + HIGHLIGHT_COLOR.len();
                for ch in RESET.chars().rev() {
                    line_cp.insert(end, ch)
                }
            }

            let fold_expr_str: String = line_cp.iter().collect();
            write!(out, "{}{}", if first { "" } else { "\r\n" }, fold_expr_str)?;
            first = false;
        }

        write!(
            out,
            "\r\x1b[{}C{SHOW_CURSOR}",
            self.cur_col as u16 % self.term_width
        )?;
        out.flush()?;

        self.cur_pair().unwrap().0.cur_operand_to_last();
        Ok(())
    }

    fn flatten(&mut self) {
        // One level of dupe-nesting allowed; might come in handy in certain situations
        while self.fold_exprs.len() >= 3 {
            let len = self.fold_exprs.len();
            if self.fold_exprs[len - 1] != self.fold_exprs[len - 2]
                || self.fold_exprs[len - 1] != self.fold_exprs[len - 3]
            {
                return;
            }

            self.fold_exprs.pop();
            self.lines.pop();
        }
    }

    fn ripple_update(&mut self) -> Result<(), RenderError> {
        let mut expr_it = self.fold_exprs.iter_mut().rev();
        let mut line_it = self.lines.iter_mut().rev();

        let cur_expr = expr_it.next().expect("No fold expression available!");
        let cur_line = line_it
            .next()
            .expect("Current fold expression is missing its string representation!");
        cur_expr.write_chars(cur_line)?;

        let mut last_result = cur_expr.evaluate().expect(concat!(
            "Can't evaluate `FoldExpr`s result ",
            "even though there is no parsing error!",
        ));

        loop {
            let next_expr = expr_it.next();
            let next_line = line_it.next();

            match (next_expr, next_line) {
                (Some(expr), Some(line)) => {
                    if !expr.change_operand(last_result) {
                        return Ok(());
                    }

                    expr.write_chars(line)?;

                    last_result = expr
                        .evaluate()
                        .map_err(|_| RenderError::FoldExprResultError)?;
                }
                (None, None) => return Ok(()),
                _ => return Err(RenderError::OutOfSync),
            }
        }
    }

    fn cur_pair(
        &mut self,
    ) -> Result<(&mut FoldExpr, &mut Vec<char>), InternalStateError> {
        let cur_expr = self
            .fold_exprs
            .last_mut()
            .ok_or(InternalStateError::NoFoldExpr)?;
        let cur_line = self.lines.last_mut().ok_or(InternalStateError::OutOfSync)?;

        Ok((cur_expr, cur_line))
    }

    fn reeval(&mut self) -> Result<(), Box<dyn Error>> {
        let cur_col = self.cur_col;
        let (cur_expr, cur_line) = self.cur_pair()?;

        match cur_expr.reparse(cur_line, cur_col) {
            Err(FoldExprError::ParseError(ParseFoldExprError::InvalidCharacter(
                ch,
                pos,
            ))) => {
                print!(
                    "\x1b[{}G{ERR_COLOR}{ch}{RESET}\x1b[{}G",
                    pos + 1,
                    cur_col + 1
                );
                stdout().flush()?;
            }
            _ => (),
        }

        Ok(())
    }

    fn syntax_error(&mut self) -> Result<bool, Box<dyn Error>> {
        let cur_col = self.cur_col;
        let (cur_expr, cur_line) = self.cur_pair()?;
        Ok(cur_expr.reparse(cur_line, cur_col).is_err())
    }

    fn reset_cursor_pos(&mut self) -> Result<(), Box<dyn Error>> {
        self.cur_col = self.cur_col.clamp(0, self.cur_pair()?.1.len());
        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }

    fn cursor_to(&mut self, pos: usize) -> Result<(), Box<dyn Error>> {
        self.cur_col = pos;
        print!("\x1b[{}G", self.cur_col + 1);
        stdout().flush()?;

        Ok(())
    }
}

#[derive(Debug)]
enum InternalStateError {
    FoldExprError,
    NoFoldExpr,
    OutOfSync,
}

impl Display for InternalStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FoldExprError => {
                write!(f, "Internal fold expression error occurred!")
            }
            Self::NoFoldExpr => write!(f, "No fold expression to work with!"),
            Self::OutOfSync => {
                write!(
                    f,
                    concat!(
                        "Fold expressions and their string representations ",
                        "are out of sync!"
                    )
                )
            }
        }
    }
}

impl Error for InternalStateError {}

#[derive(Debug)]
enum RenderError {
    Fmt(fmt::Error),
    FoldExprResultError,
    Io(io::Error),
    OutOfSync,
}

impl Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fmt(err) => write!(f, "`RenderError`: {err}"),
            Self::FoldExprResultError => {
                write!(f, "Evaluation of fold expression result failed!")
            }
            Self::Io(err) => write!(f, "`RenderError`: {err}"),
            Self::OutOfSync => {
                write!(
                    f,
                    "Fold expression count doesn't match string representation count!"
                )
            }
        }
    }
}

impl Error for RenderError {}

impl From<OperationExecutionError> for RenderError {
    fn from(_: OperationExecutionError) -> Self {
        Self::FoldExprResultError
    }
}

impl From<fmt::Error> for RenderError {
    fn from(err: fmt::Error) -> Self {
        Self::Fmt(err)
    }
}

impl From<std::io::Error> for RenderError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

fn nth_operand_pos(slice: &[char], n: usize) -> Option<(usize, usize)> {
    let mut cur_op_idx = 0;
    let mut in_operand = false;
    let mut negative = false;
    let mut start = None;

    for (slice_idx, ch) in slice.iter().enumerate() {
        match ch {
            ch if ch.is_whitespace() => in_operand = false,
            ch if *ch == Operation::Subtraction.as_char() => negative = true,
            ch if ch.is_ascii_digit() && in_operand == false => {
                in_operand = true;
                negative = false;
                cur_op_idx += 1;
            }
            _ => (),
        }

        if start.is_none()
            && ((negative && cur_op_idx == n) || (in_operand && cur_op_idx - 1 == n))
        {
            start = Some(slice_idx);
        }

        if in_operand && cur_op_idx - 1 == n {
            let mut slice_idx = slice_idx + 1;
            while slice_idx < slice.len() && !slice[slice_idx].is_whitespace() {
                slice_idx += 1;
            }

            return Some((start.unwrap(), slice_idx));
        }
    }

    None
}
