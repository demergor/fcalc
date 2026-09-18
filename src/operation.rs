use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Addition,
    Division,
    Factorial,
    Multiplication,
    Power,
    Root,
    Subtraction,
}

impl Operation {
    pub fn is_unary(&self) -> bool {
        match self {
            Self::Factorial => true,
            _ => false,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            Self::Addition => '+',
            Self::Division => '÷',
            Self::Factorial => '!',
            Self::Multiplication => '×',
            Self::Power => '^',
            Self::Root => '√',
            Self::Subtraction => '-',
        }
    }
}

impl TryFrom<char> for Operation {
    type Error = ParseOperationError;

    fn try_from(ch: char) -> Result<Self, Self::Error> {
        match ch {
            '+' => Ok(Self::Addition),
            '/' | '÷' => Ok(Self::Division),
            '!' => Ok(Self::Factorial),
            '*' | '×' | '⋅' | '∗' => Ok(Self::Multiplication),
            '^' => Ok(Self::Power),
            '\\' | '√' => Ok(Self::Root),
            '-' => Ok(Self::Subtraction),
            _ => Err(ParseOperationError::InvalidCharacter(ch)),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Debug)]
pub enum OperationExecutionError {
    DivisionByZero,
    InvalidInput,
    WrongOperandCount,
}

impl Display for OperationExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "Division by zero!"),
            Self::InvalidInput => write!(f, "Operation not defined for given input!"),
            Self::WrongOperandCount => {
                write!(f, "Wrong number of operands provided for given operator!")
            }
        }
    }
}

impl Error for OperationExecutionError {}

fn divide(x: f64, y: f64) -> Result<f64, OperationExecutionError> {
    if y == 0.0 {
        return Err(OperationExecutionError::DivisionByZero);
    }

    Ok(x / y)
}

fn factorial(x: f64) -> Result<f64, OperationExecutionError> {
    if x < 0.0 || x.fract() != 0.0 || x > i64::MAX as f64 {
        return Err(OperationExecutionError::InvalidInput);
    }

    let n = x as u64;
    Ok((1..=n).fold(1.0f64, |acc, x| acc * x as f64))
}

impl Operation {
    pub fn execute(self, ops: &[f64]) -> Result<f64, OperationExecutionError> {
        let mut it = ops.iter().copied();
        let Some(first) = it.next() else {
            return Err(OperationExecutionError::WrongOperandCount);
        };

        match self {
            Self::Addition => Ok(it.fold(first, |acc, x| acc + x)),
            Self::Division => it.try_fold(first, divide),
            Self::Factorial => {
                if it.next().is_some() {
                    return Err(OperationExecutionError::WrongOperandCount);
                }

                factorial(first)
            }
            Self::Multiplication => Ok(it.fold(first, |acc, x| acc * x)),
            Self::Power => Ok(it.fold(first, |acc, x| acc.powf(x))),
            Self::Root => Ok(it.fold(first, |acc, x| acc.powf(1.0 / x))),
            Self::Subtraction => {
                if let Some(next) = it.next() {
                    Ok(it.fold(first - next, |acc, x| acc - x))
                } else {
                    Ok(-first)
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseOperationError {
    InvalidCharacter(char),
    EmptyInput,
}

impl Display for ParseOperationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter(ch) => write!(
                f,
                "Parsing of operation failed due to following invalid character: {ch}"
            ),
            Self::EmptyInput => write!(f, "No operands provided!"),
        }
    }
}

impl Error for ParseOperationError {}
