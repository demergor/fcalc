use core::fmt;
use std::{
    cmp::min, error::Error, fmt::Display, io::{self, BufWriter, Read, Write, stdin, stdout},
};

use crate::{
    fold_expr::FoldExpr,
    io::Key,
    operation::OperationExecutionError,
    terminal::{self, Terminal},
};

pub struct IoHandler {
    fold_exprs: Vec<FoldExpr>,
    lines: Vec<Vec<char>>,
    cur_col: usize,
    cur_row: u16,
    syntax_error: bool,

    term_width: u16,
    term_height: u16,
}

impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        const CLEAR_LINE: &str = "\r\x1b[2K";
        const TIMEOUT_MS: i32 = 100;

        let mut buf = [0; 1];
        let mut stdout = std::io::stdout();
        let mut stdin = stdin();

        write!(stdout, "{}Determining cursor position...", CLEAR_LINE)?;
        while Terminal::byte_ready(10)? {
            stdin.read_exact(&mut buf)?;
        }

        write!(stdout, "\x1b[6n")?;
        stdout.flush()?;

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if buf[0] != terminal::ESC {
            return Err(IoError::MissingRowInformation);
        }

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if buf[0] != b'[' {
            return Err(IoError::MissingRowInformation);
        }

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if !buf[0].is_ascii_digit() {
            return Err(IoError::MissingRowInformation);
        }

        let mut cur_row = u16::from(buf[0] - b'0');
        while buf[0] != b';' {
            if !Terminal::byte_ready(TIMEOUT_MS)? {
                return Err(IoError::MissingRowInformation);
            }

            stdin.read_exact(&mut buf)?;
            if !buf[0].is_ascii_digit() {
                return Err(IoError::MissingRowInformation);
            }

            cur_row = 10 * cur_row + u16::from(buf[0] - b'0');
        }

        write!(stdout, "{}Clearing stdin...", CLEAR_LINE)?;
        stdout.flush()?;

        while buf[0] != b'R' {
            if !Terminal::byte_ready(TIMEOUT_MS)? {
                return Err(IoError::MissingRowInformation);
            }

            stdin.read_exact(&mut buf)?;
        }

        write!(stdout, "{CLEAR_LINE}")?;
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
            cur_row,
            syntax_error: false,
            term_width: bounds.width,
            term_height: bounds.height,
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
                        .collapse();
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
            Key::Char('e') => {
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
                        _ => {},
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
            Key::Char('E') => {
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
                        _ => {},
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
                self.cur_col = min(
                    self.cur_col + 1,
                    self.lines
                        .last()
                        .ok_or(InternalStateError::OutOfSync)?
                        .len(),
                )
            }
            Key::ArrowLeft => {
                self.cur_col = if self.cur_col == 0 {
                    0
                } else {
                    self.cur_col - 1
                }
            }
            Key::Char(ch) => {
                let cur_col = self.cur_col;
                let (cur_expr, cur_line) = self.cur_pair()?;
                cur_line.insert(cur_col, ch);
                // TODO: Re-parse cur_expr based on cur_line

                print!("{ch}");
                stdout().flush()?;

                self.ripple_update()?;
                self.render()?;
            }
            _ => return Ok(State::Continue),
        }

        Ok(State::Continue)
    }

    fn render(&mut self) -> Result<(), RenderError> {
        self.ripple_update()?;
        let up_count = min(
            self.cur_row,
            self.lines.iter().fold(0, |acc, line| {
                acc + (line.len() as u16 - 1) / self.term_width + 1
            }),
        ) - 1;

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
        cur_expr.write_chars(cur_line, false);

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

                    expr.write_chars(line, true);
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

enum State {
    Continue,
    Quit(f64),
}

#[derive(Debug)]
enum InternalStateError {
    FoldExprError,
    NoFoldExpr,
    OutOfSync,
    SyntaxError,
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
            Self::SyntaxError => {
                write!(f, "Fold expression parsing failed due to syntax error!")
            }
        }
    }
}

impl Error for InternalStateError {}

enum IoError {
    HandlerConstructionError,
    Io(io::Error),
    MissingRowInformation,
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

#[derive(Debug)]
enum RenderError {
    FoldExprResultError,
    NoFoldExpr,
    OutOfSync,
    Io(io::Error),
}

impl Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FoldExprResultError => {
                write!(f, "Evaluation of fold expression result failed!")
            }
            Self::NoFoldExpr => {
                write!(f, "No fold expression to render!")
            }
            Self::OutOfSync => {
                write!(
                    f,
                    "Fold expression count doesn't match string representation count!"
                )
            }
            Self::Io(err) => write!(f, "RenderError: {err}"),
        }
    }
}

impl Error for RenderError {}

impl From<OperationExecutionError> for RenderError {
    fn from(_: OperationExecutionError) -> Self {
        Self::FoldExprResultError
    }
}

impl From<std::io::Error> for RenderError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
