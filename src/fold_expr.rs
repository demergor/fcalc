use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use crate::operation::{Operation, OperationExecutionError, ParseOperationError};

#[derive(Debug)]
pub struct FoldExpr {
    pub operation: Operation,
    result: Option<f64>,
    parent: Option<usize>,
    children: Vec<usize>,
}

#[derive(Debug)]
pub struct FoldExprArena {
    pub buf: Vec<FoldExpr>,
}

impl FoldExprArena {
    pub fn new() -> FoldExprArena {
        FoldExprArena { buf: Vec::new() }
    }

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
    ) -> Result<usize, FoldExprError> {
        match parent_id {
            Some(id) if id >= self.buf.len() => {
                return Err(FoldExprError::IdAccessError(id))
            }
            _ => (),
        }

        let (operation, operands) = parse(slice)?;
        let fold_expr_id = self.buf.len();
        self.buf.push(FoldExpr {
            operation,
            result: None,
            parent: parent_id,
            children: Vec::new(),
        });

        for operand in operands {
            self.buf.push(FoldExpr {
                operation: Operation::Addition,
                result: Some(operand),
                parent: Some(fold_expr_id),
                children: Vec::new(),
            });
        }

        for i in fold_expr_id + 1..self.buf.len() {
            self.buf[fold_expr_id].children.push(i);
        }

        if let Some(parent_id) = parent_id {
            self.buf[parent_id].children.push(fold_expr_id);
        };

        Ok(fold_expr_id)
    }

    /// Removes the `FoldExpression` with given @param id and returns the previous ID of 
    /// the `FoldExpression` that is now associated with @param id
    pub fn remove(&mut self, id: usize) -> Result<usize, FoldExprError> {
        if id >= self.buf.len() {
            return Err(FoldExprError::IdAccessError(id));
        }

        self.buf.swap_remove(id);
        let removed_id = self.buf.len();
        let Some(parent_id) = self.buf[id].parent else {
            return Ok(removed_id);
        };

        // The former last element now has a new ID: the parent needs to be updated
        for child_id in &mut self.buf[parent_id].children {
            if *child_id == removed_id {
                *child_id = id;
                return Ok(removed_id);
            }
        }

        Err(FoldExprError::ParentChildViolation(id, parent_id))
    }

    pub fn update(&mut self, slice: &[char], id: usize) -> Result<(), FoldExprError> {
        if id >= self.buf.len() {
            return Err(FoldExprError::IdAccessError(id));
        }

        let (operation, operands) = parse(slice)?;
        self.buf[id].operation = operation;

        for child_id in self.buf[id].children.clone() {
            self.remove(child_id)?;
        }

        self.buf[id].children.clear();
        for i in self.buf.len()..self.buf.len() + operands.len() {
            self.buf.push(FoldExpr {
                operation: Operation::Addition,
                result: None,
                parent: Some(id),
                children: Vec::new(),
            });
            self.buf[id].children.push(i);
        }

        Ok(())
    }

    pub fn root_id(&self) -> Option<usize> {
        if self.buf.is_empty() {
            return None;
        }

        let mut cur_id = 0;
        while let Some(parent_id) = self.buf[cur_id].parent {
            cur_id = parent_id;
        }

        Some(cur_id)
    }

    pub fn init(&mut self, op: Operation) -> usize {
        self.buf.clear();
        self.buf.push(FoldExpr {
            operation: op,
            result: None,
            parent: None,
            children: Vec::new(),
        });

        self.buf.len() - 1
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
            }
            Self::ParentChildViolation(parent_id, child_id) => {
                write!(
                    f,
                    "Detected inconsistency in parent-child relationship between parent\
                     fold expression with ID {parent_id} and child fold expression with\
                     ID {child_id}"
                )
            }
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

fn parse(slice: &[char]) -> Result<(Operation, Vec<f64>), FoldExprError> {
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
                    return Err(ParseFoldExprError::InvalidCharacter(*next, pos).into());
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

    Ok((operation, operands))
}
