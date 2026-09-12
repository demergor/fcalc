use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use crate::operation::{Operation, OperationExecutionError, ParseOperationError};

#[derive(Debug)]
pub struct FoldExpr {
    operation: Operation,
    result: Option<f64>,
    parent: Option<usize>,
    children: Vec<usize>,
}

#[derive(Debug)]
pub struct FoldExprArena {
    buf: Vec<FoldExpr>,
}

impl FoldExprArena {
    pub fn evaluate(&mut self, id: usize) -> Result<f64, OperationExecutionError> {
        if let Some(cached_result) = self.buf[id].result {
            return Ok(cached_result);
        }

        let mut operands = Vec::with_capacity(self.buf[id].children.len());
        for child_id in self.buf[id].children.clone() {
            operands.push(self.evaluate(child_id)?);
        }

        let result = self.buf[id].operation.execute(&operands)?;
        self.buf[id].result = Some(result);

        Ok(result)
    }

    pub fn add(
        &mut self,
        slice: &[char],
        parent_id: Option<usize>,
    ) -> Result<(), FoldExprError> {
        match parent_id {
            Some(id) if id >= self.buf.len() => {
                return Err(FoldExprError::IdAccessError(id))
            }
            _ => (),
        }

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
        let mut comp_div = 1.0;
        let mut first_digit = true;
        let mut reverse = false;
        let mut pos = 0;

        while let Some(ch) = it.next() {
            pos += 1;
            match ch {
                ch if ch.is_whitespace() => {
                    if !first_digit {
                        operands.push(cur / comp_div);
                        cur = 0.0;
                        already_float = false;
                        comp_div = 1.0;
                        first_digit = true;
                    } else {
                        continue;
                    }
                }
                '.' if !already_float => already_float = true,
                'r' => {
                    if let Some(next) = it.next() {
                        return Err(
                            ParseFoldExprError::InvalidCharacter(*next, pos).into()
                        );
                    }

                    reverse = true;
                }
                ch if let Some(digit) = ch.to_digit(10) => {
                    cur = cur * 10.0 + digit as f64;
                    first_digit = false;
                    comp_div *= if already_float { 10.0 } else { 1.0 };
                }
                ch => return Err(ParseFoldExprError::InvalidCharacter(*ch, pos).into()),
            }
        }

        if !first_digit {
            operands.push(cur / comp_div);
        }

        if reverse {
            operands.reverse();
        }

        let fold_expr = FoldExpr {
            operation,
            result: None,
            parent: parent_id,
            children: Vec::new(),
        };

        let id = self.buf.len();
        self.buf.push(fold_expr);

        if let Some(parent_id) = parent_id {
            self.buf[parent_id].children.push(id);
        };

        Ok(())
    }

    pub fn remove(&mut self, id: usize) -> Result<(), FoldExprError> {
        if id >= self.buf.len() {
            return Err(FoldExprError::IdAccessError(id));
        }

        self.buf.swap_remove(id);

        if self.buf[id].parent.is_none() {
            return Ok(());
        }

        let removed_id = self.buf.len();
        let parent_id = self.buf[id].parent.unwrap();

        for child_id in &mut self.buf[parent_id].children {
            if *child_id == removed_id {
                *child_id = id;
                return Ok(());
            }
        }

        Err(FoldExprError::ParentChildViolation(id, parent_id))
    }
}

#[derive(Debug)]
pub enum FoldExprError {
    IdAccessError(usize),
    ParentChildViolation(usize, usize),
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
            Self::IdAccessError(id) => {
                write!(f, "Error accessing fold expression with ID {id}")
            },
            Self::ParentChildViolation(parent_id, child_id) => { 
                write!(
                    f,
                    "Detected inconsistency in parent-child relationship between parent\
                     fold expression with ID {parent_id} and child fold expression with\
                     ID {child_id}"
                )

            },
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
