use core::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Addition,
    Division,
    Factorial,
    Multiplication,
    Power,
    Root,
    Subtraction,
}

#[derive(Debug)]
enum OperationExecutionError {
    DivisionByZero,
    InvalidInput,
    Overflow,
    WrongOperandCount,
}

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
    fn execute(
        self,
        ops: impl IntoIterator<Item = f64>,
    ) -> Result<f64, OperationExecutionError> {
        let mut it = ops.into_iter();
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
            Self::Subtraction => Ok(it.fold(first, |acc, x| acc - x)),
        }
    }
}

#[derive(Debug)]
pub enum ParseOperationError {
    InvalidCharacter(char),
    EmptyInput,
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
        let c = match self {
            Self::Addition => '+',
            Self::Division => '/',
            Self::Factorial => '!',
            Self::Multiplication => '*',
            Self::Power => '^',
            Self::Root => '√',
            Self::Subtraction => '-',
        };

        write!(f, "{c}")
    }
}
