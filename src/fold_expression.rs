use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use crate::operation::{Operation, OperationExecutionError, ParseOperationError};

pub struct FoldExpression {
    operands: Vec<f64>,
    operation: Operation,
}

impl FoldExpression {
    pub fn evaluate(&self) -> Result<f64, OperationExecutionError> {
        self.operation.execute(&self.operands)
    }
}

#[derive(Debug)]
pub enum ParseFoldExpressionError {
    InvalidCharacter(char),
    OperationError(ParseOperationError),
}

impl From<ParseOperationError> for ParseFoldExpressionError {
    fn from(e: ParseOperationError) -> Self {
        Self::OperationError(e)
    }
}

impl Display for ParseFoldExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter(ch) => write!(
                f,
                "Parsing of fold expression failed due to following character: {ch}"
            ),
            Self::OperationError(e) => write!(
                f,
                "Parsing of fold expression failed at operation parsing step: {e}"
            ),
        }
    }
}

impl Error for ParseFoldExpressionError {}

impl TryFrom<&[char]> for FoldExpression {
    type Error = ParseFoldExpressionError;

    fn try_from(chars: &[char]) -> Result<Self, Self::Error> {
        let start = chars.iter().position(|ch| !ch.is_whitespace());
        let end = chars.iter().rposition(|ch| !ch.is_whitespace());

        let chars = match (start, end) {
            (Some(start), Some(end)) => &chars[start..=end],
            _ => &[],
        };

        let mut it = chars.iter();
        let operation: Operation = match it.next() {
            Some(ch) => Operation::try_from(*ch)?,
            None => return Err(ParseOperationError::EmptyInput.into()),
        };

        let mut operands = Vec::new();
        let mut cur: f64 = 0.0;
        let mut already_float = false;
        let mut comp_div = 1.0;
        let mut first_digit = true;

        while let Some(ch) = it.next() {
            if ch.is_whitespace() {
                if !first_digit {
                    operands.push(cur / comp_div);
                    cur = 0.0;
                    already_float = false;
                    comp_div = 1.0;
                    first_digit = true;
                }

                continue;
            }

            if ch == &'.' && !already_float {
                already_float = true;
                continue;
            }

            if let Some(digit) = ch.to_digit(10) {
                cur = cur * 10.0 + digit as f64;
                first_digit = false;
            } else {
                return Err(ParseFoldExpressionError::InvalidCharacter(*ch));
            }

            comp_div *= if already_float { 10.0 } else { 1.0 };
        }

        Ok(FoldExpression {
            operands: operands.into_iter().skip(1).collect(),
            operation,
        })
    }
}
