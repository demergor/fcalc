use core::fmt;
use std::{
    error::Error, fmt::{Display, Formatter, Write},
};

use crate::operation::{Operation, OperationExecutionError, ParseOperationError};

#[derive(Clone, Debug, PartialEq)]
pub struct FoldExpr {
    pub operation: Operation,
    operands: Vec<f64>,
    cur_operand: Option<usize>,
}

impl FoldExpr {
    pub fn reparse(
        &mut self,
        slice: &[char],
        cursor_pos: usize,
    ) -> Result<(), FoldExprError> {
        let (operation, operands, cur_operand) = parse(slice, cursor_pos)?;
        self.operation = operation;
        self.operands = operands;

        match cur_operand {
            Some(idx) if idx <= self.operands.len() => self.cur_operand = cur_operand,
            None if !self.operands.is_empty() => {
                self.cur_operand = Some(self.operands.len() - 1)
            }
            _ => self.cur_operand = None,
        };

        if self.evaluate().is_err() {
            return Err(FoldExprError::ParseError(
                ParseFoldExprError::InvalidCharacter(self.operation.as_char(), 0),
            ));
        }

        Ok(())
    }

    pub fn evaluate(&self) -> Result<f64, OperationExecutionError> {
        self.operation.execute(&self.operands)
    }

    pub fn reverse(&mut self) {
        self.operands.reverse();
    }

    pub fn write_chars(&self, buf: &mut Vec<char>) -> std::fmt::Result {
        buf.clear();
        let mut cw = CharWriter { buf };
        write!(cw, "{}", self.operation)?;

        match self.cur_operand {
            Some(idx) if idx < self.operands.len() => (),
            None if self.operands.is_empty() => {
                write!(cw, " ")?;
                return Ok(());
            }
            Some(idx) => panic!(
                "Invalid index stored as current operand to `FoldExpr`: \
                index is {idx}, but only {} operands exist!",
                self.operands.len()
            ),
            None => panic!(
                "No current operand assigned even though candidate exists: \
                Number of available operands: {}",
                self.operands.len()
            ),
        };

        if !self.operation.is_unary() {
            write!(cw, " ")?;
        }

        for i in 0..self.operands.len() {
            write!(cw, "{} ", self.operands[i])?;
        }

        print!("{buf:?}");

        Ok(())
    }

    pub fn collapse(&mut self) -> Result<(), OperationExecutionError> {
        let result = self.evaluate()?;
        self.operation = Operation::Addition;
        self.operands.clear();
        self.operands.push(result);
        self.cur_operand = Some(0);

        Ok(())
    }

    pub fn change_operand(&mut self, new_val: f64) -> bool {
        let Some(cur_operand) = self.cur_operand else {
            panic!("No current operand to change in fold expression!");
        };

        let dirty = new_val != self.operands[cur_operand];
        if dirty {
            self.operands[cur_operand] = new_val;
        }

        dirty
    }

    pub fn cur_operand(&self) -> Option<usize> {
        self.cur_operand
    }

    pub fn next_operand(&mut self) -> Option<usize> {
        let Some(op_idx) = self.cur_operand else {
            assert!(!self.operands.is_empty());
            return None;
        };

        self.cur_operand = Some((op_idx + 1) % self.operands.len());

        self.cur_operand
    }

    pub fn previous_operand(&mut self) -> Option<usize> {
        let Some(id) = self.cur_operand else {
            assert!(!self.operands.is_empty());
            return None;
        };

        self.cur_operand = Some(if id == 0 {
            self.operands.len() - 1
        } else {
            id - 1
        });

        self.cur_operand
    }

    pub fn cur_operand_to_last(&mut self) {
        if self.cur_operand.is_none() {
            return;
        }

        self.cur_operand = Some(self.operands.len() - 1);
    }

    pub fn new_from_cur_operand(&self) -> Option<FoldExpr> {
        let Some(cur_op) = self.cur_operand else {
            return None;
        };

        Some(FoldExpr {
            operation: Operation::Addition,
            operands: vec![self.operands[cur_op]],
            cur_operand: Some(0),
        })
    }
}

impl Default for FoldExpr {
    fn default() -> Self {
        Self {
            operation: Operation::Addition,
            operands: vec![0.0],
            cur_operand: Some(0),
        }
    }
}

impl Display for FoldExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[1m{}\x1b[0m", self.operation)?;
        let mut first = true;

        for op in &self.operands {
            if first && !self.operation.is_unary() {
                write!(f, " ")?;
                first = false;
            }

            write!(f, "{op}")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum FoldExprError {
    ParseError(ParseFoldExprError),
}

impl From<ParseFoldExprError> for FoldExprError {
    fn from(err: ParseFoldExprError) -> Self {
        Self::ParseError(err)
    }
}

impl From<ParseOperationError> for FoldExprError {
    fn from(err: ParseOperationError) -> Self {
        Self::ParseError(ParseFoldExprError::OperationError(err))
    }
}

impl Display for FoldExprError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(err) => write!(f, "Error parsing `FoldExpr`: {err}"),
        }
    }
}

impl Error for FoldExprError {}

#[derive(Debug)]
pub enum ParseFoldExprError {
    InvalidCharacter(char, usize),
    OperationError(ParseOperationError),
}

impl From<ParseOperationError> for ParseFoldExprError {
    fn from(e: ParseOperationError) -> Self {
        Self::OperationError(e)
    }
}

impl Display for ParseFoldExprError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter(ch, pos) => write!(
                f,
                "Parsing of fold expression failed due to invalid character '{ch}' \
                appearing at position {pos} after the operator (the operator itself \
                marks position 0)"
            ),
            Self::OperationError(e) => write!(
                f,
                "Parsing of fold expression failed at operation parsing step: {e}"
            ),
        }
    }
}

impl Error for ParseFoldExprError {}

fn parse(
    slice: &[char],
    cursor_pos: usize,
) -> Result<(Operation, Vec<f64>, Option<usize>), FoldExprError> {
    let start = slice.iter().position(|ch| !ch.is_whitespace());
    let end = slice.iter().rposition(|ch| !ch.is_whitespace());

    let slice = match (start, end) {
        (Some(start), Some(end)) => &slice[start..=end],
        _ => &[],
    };

    let mut it = slice.iter();
    let operation: Operation = match it.next() {
        Some(ch) => Operation::try_from(*ch)?,
        None => return Err(ParseOperationError::EmptyInput.into()),
    };

    let mut operands = Vec::new();
    let mut cur: f64 = 0.0;
    let mut already_float = false;
    let mut already_negative = false;
    let mut comp_div = 1.0;
    let mut first_digit = true;
    let mut pos = 0;
    let mut cur_operand = None;

    while let Some(ch) = it.next() {
        pos += 1;
        match ch {
            ch if ch.is_whitespace() => {
                if !first_digit {
                    operands.push(if already_negative {
                        -cur / comp_div
                    } else {
                        cur / comp_div
                    });
                    cur = 0.0;
                    already_float = false;
                    already_negative = false;
                    comp_div = 1.0;
                    first_digit = true;
                } else {
                    continue;
                }
            }
            '.' if !already_float => already_float = true,
            '-' if !already_negative => {
                already_negative = true;
            }
            ch if let Some(digit) = ch.to_digit(10) => {
                cur = cur * 10.0 + digit as f64;
                first_digit = false;
                comp_div *= if already_float { 10.0 } else { 1.0 };
            }
            ch => return Err(ParseFoldExprError::InvalidCharacter(*ch, pos).into()),
        }

        if pos == cursor_pos && ch.is_ascii_digit() {
            cur_operand = Some(operands.len());
        }
    }

    if !first_digit {
        operands.push(if already_negative {
            -cur / comp_div
        } else {
            cur / comp_div
        });
    }

    if operands.len() <= 1 || operation != Operation::Addition {
        return Ok((operation, operands, cur_operand));
    } 

    if operands.iter().all(|x| *x == 0.0) {
        operands.dedup();
    } else {
        operands.retain(|x| *x != 0.0);
    }

    if let Some(op_idx) = cur_operand {
        cur_operand = Some(op_idx.clamp(0, operands.len() - 1));
    }

    Ok((operation, operands, cur_operand))
}

struct CharWriter<'a> {
    buf: &'a mut Vec<char>,
}

impl fmt::Write for CharWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.extend(s.chars());
        Ok(())
    }
}
