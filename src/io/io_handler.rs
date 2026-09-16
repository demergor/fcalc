use core::fmt;
use std::{
    cmp::min,
    error::Error,
    fmt::Display,
    io::{self, stdout, BufWriter, Write},
};

use crate::{
    fold_expr::{FoldExpr, FoldExprError, ParseFoldExprError},
    io::Key,
    operation::{Operation, OperationExecutionError},
    terminal::Terminal,
};

pub struct IoHandler {
    fold_exprs: Vec<FoldExpr>,
    lines: Vec<Vec<char>>,
    cur_col: usize,
    cur_row: u16,
    syntax_error: bool,

    term_width: u16,
}

impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        let root_expr = FoldExpr::default();
        let mut buf = Vec::new();

        if root_expr.write_chars(&mut buf, false).is_err() {
            return Err(IoError::HandlerConstructionError);
        };

        let buf_back = buf.len() - 1;

        Ok(IoHandler {
            fold_exprs: vec![root_expr],
            lines: vec![buf],
            cur_col: buf_back,
            cur_row: 0,
            syntax_error: false,
            term_width: bounds.width,
        })
    }

    pub fn update(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Enter => {
                if self.syntax_error {
                    return Ok(State::Continue);
                }

                if self.fold_exprs.len() < 2 {
                    self.fold_exprs
                        .last_mut()
                        .ok_or(InternalStateError::FoldExprError)?
                        .collapse()?;
                } else {
                    let collapsed =
                        self.fold_exprs[self.fold_exprs.len() - 1].evaluate()?;
                    let idx = self.fold_exprs.len() - 2;
                    self.fold_exprs[idx].change_operand(collapsed);
                    self.fold_exprs.pop();
                    self.lines.pop();
                }

                self.render()?;
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
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                if self.cur_pair()?.0.change_operation(op) {
                    self.ripple_update()?;
                    self.render()?;
                }
            }
            Key::Char('e') => {
                let (cur_expr, cur_line) = self.cur_pair()?;
                let Some(child_expr) = cur_expr.new_from_cur_operand() else {
                    return Ok(State::Continue);
                };

                cur_expr.write_chars(cur_line, true)?;
                let mut buf = Vec::new();
                child_expr.write_chars(&mut buf, false)?;

                self.fold_exprs.push(child_expr);
                self.lines.push(buf);
            }
            Key::Char('w') => {
                let (cur_expr, cur_line) = self.cur_pair()?;
                let Some(next_op) = cur_expr.next_operand() else {
                    return Ok(State::Continue);
                };

                let mut cur_col = 1;
                let mut op_idx = 0;
                let mut seen_digit = false;

                while cur_col < cur_line.len() && op_idx != next_op + 1 {
                    match cur_line[cur_col] {
                        ch if ch.is_whitespace() => {
                            seen_digit = false;
                        }
                        ch if !seen_digit && ch.is_digit(10) => {
                            seen_digit = true;
                            op_idx += 1;
                        }
                        _ => {}
                    }

                    cur_col += 1;
                }

                while cur_col < cur_line.len() && !cur_line[cur_col].is_whitespace() {
                    cur_col += 1;
                }

                self.cur_col = cur_col;
                print!("\r\x1b[{}C", self.cur_col + 1);
                stdout().flush()?;
            }
            Key::Char('b') => {
                let (cur_expr, cur_line) = self.cur_pair()?;
                let Some(prev_op) = cur_expr.previous_operand() else {
                    return Ok(State::Continue);
                };

                let mut cur_col = 1;
                let mut op_idx = 0;
                let mut seen_digit = false;

                while cur_col < cur_line.len() && op_idx != prev_op + 1 {
                    match cur_line[cur_col] {
                        ch if ch.is_whitespace() => {
                            seen_digit = false;
                        }
                        ch if !seen_digit && ch.is_digit(10) => {
                            seen_digit = true;
                            op_idx += 1;
                        }
                        _ => {}
                    }

                    cur_col += 1;
                }

                while cur_col < cur_line.len() && !cur_line[cur_col].is_whitespace() {
                    cur_col += 1;
                }

                self.cur_col = cur_col;
                print!("\r\x1b[{}C", self.cur_col + 1);
                stdout().flush()?;
            }
            Key::Char('r') => {
                if self.syntax_error {
                    return Ok(State::Continue);
                }

                self.fold_exprs
                    .last_mut()
                    .ok_or(InternalStateError::NoFoldExpr)?
                    .reverse();
                self.render()?;
            }
            Key::ArrowRight => {
                self.cur_col = min(self.cur_col + 1, self.cur_pair()?.1.len())
            }
            Key::ArrowLeft => {
                self.cur_col = if self.cur_col == 0 {
                    0
                } else {
                    self.cur_col - 1
                }
            }
            Key::Char(ch) => {
                self.syntax_error = true;
                let mut cur_col = self.cur_col;
                let (cur_expr, cur_line) = self.cur_pair()?;

                cur_line.insert(cur_col, ch);
                cur_col += 1;
                print!("\r\x1b[2K{}", cur_line.iter().collect::<String>());

                match cur_expr.reparse(cur_line) {
                    Err(FoldExprError::ParseError(
                        ParseFoldExprError::InvalidCharacter(_, pos),
                    )) => print!(
                        "\r\x1b[{pos}C\x1b[41m\x1b[C\x1b[0m\r\x1b[{}C",
                        self.cur_col
                    ),
                    Err(FoldExprError::ParseError(
                        ParseFoldExprError::OperationError(_),
                    )) => print!("\r\x1b[41m\x1b[C\x1b[0m\r\x1b[{}C", self.cur_col),
                    Err(err) => return Err(err.into()),
                    Ok(_) => self.syntax_error = false,
                };

                stdout().flush()?;
                self.cur_col = cur_col;

                if !self.syntax_error {
                    self.ripple_update()?;
                    self.render()?;
                }
            }
            Key::Backspace => {
                let cur_col = self.cur_col;
                let (cur_expr, cur_line) = self.cur_pair()?;

                if cur_col == 0 {
                    ()
                } else {
                    cur_line.remove(cur_col - 1);
                }

                if cur_expr.reparse(cur_line).is_err() {
                    self.syntax_error = false;
                } else {
                    self.syntax_error = true;
                }
            }
            _ => return Ok(State::Continue),
        }

        Ok(State::Continue)
    }

    fn render(&mut self) -> Result<(), RenderError> {
        self.ripple_update()?;
        let mut up_count = min(
            self.cur_row,
            self.lines.iter().fold(0, |acc, line| {
                acc + (line.len() as u16 - 1) / self.term_width + 1
            }),
        );

        if up_count != 0 {
            up_count -= 1;
        }

        let first_to_print: usize = if up_count == self.cur_row {
            0
        } else {
            let mut idx = self.lines.len();
            let mut up_count_cp = up_count;

            while idx > 0 && up_count_cp > 0 {
                idx -= 1;
                if self.lines[idx].is_empty() {
                    panic!("Empty string representation of a fold expression!");
                }

                up_count_cp -= (self.lines[idx].len() as u16 - 1) / self.term_width + 1;
            }

            idx
        };

        const HIDE_CURSOR: &str = "\x1b[?25l";
        const SHOW_CURSOR: &str = "\x1b[?25h";
        const ERASE_FROM_CURSOR: &str = "\x1b[0J";

        let mut out = BufWriter::new(std::io::stdout().lock());
        write!(out, "{HIDE_CURSOR}")?;
        write!(out, "\x1b[{up_count}A\r{ERASE_FROM_CURSOR}")?;

        let mut first = true;
        for i in first_to_print..self.lines.len() {
            let fold_expr_str: String = self.lines[i].iter().collect();
            write!(out, "{}{}", if first { "" } else { "\r\n" }, fold_expr_str)?;
            first = false;
        }

        write!(
            out,
            "\r\x1b[{}C{SHOW_CURSOR}",
            self.cur_col as u16 % self.term_width
        )?;
        out.flush()?;

        Ok(())
    }

    fn ripple_update(&mut self) -> Result<(), RenderError> {
        let mut expr_it = self.fold_exprs.iter_mut().rev();
        let mut line_it = self.lines.iter_mut().rev();

        let cur_expr = expr_it.next().expect("No fold expression available!");
        let cur_line = line_it
            .next()
            .expect("Current fold expression is missing its string representation!");
        cur_expr.write_chars(cur_line, false)?;

        let mut last_result = cur_expr.evaluate().expect(concat!(
            "Can't evaluate `FoldExpr`s result ",
            "even though there is no parsing error!",
        ));

        loop {
            let next_expr = expr_it.next();
            let next_line = line_it.next();

            match (next_expr, next_line) {
                (Some(expr), Some(line)) => {
                    if !cur_expr.change_operand(last_result) {
                        return Ok(());
                    }

                    expr.write_chars(line, true)?;
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
}

#[derive(PartialEq)]
pub enum State {
    Continue,
    Quit(f64),
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
pub enum IoError {
    HandlerConstructionError,
    Io(io::Error),
    MissingRowInformation,
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
            Self::MissingRowInformation => write!(f, "Couldn't fetch row information!"),
        }
    }
}

impl Error for IoError {}

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
