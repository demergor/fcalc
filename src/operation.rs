#[derive(Clone, Copy, Debug)]
enum Operation {
    Addition,
    Division,
    Multiplication,
    Subtraction,
    Root,
    Power,
    Factorial,
}

enum OperationExecutionError {
    DivisionByZero,
    InvalidInput,
    Overflow,
    WrongOperandCount,
}

fn factorial(n: i64) -> Result<i64, OperationExecutionError> {
    if n < 0 {
        return Err(OperationExecutionError::InvalidInput);
    }

    (1..=n)
        .try_fold(1i64, |acc, x| acc.checked_mul(x))
        .ok_or(OperationExecutionError::Overflow)
}

fn sum_int(x: i64, y: i64) -> Result<i64, OperationExecutionError> {
    x.checked_add(y).ok_or(OperationExecutionError::Overflow)
}

fn sum_float(x: f64, y: f64) -> Result<f64, OperationExecutionError> {
    Ok(x + y)
}

impl Operation {
    fn execute_int(
        &self,
        ops: impl IntoIterator<Item = i64>,
    ) -> Result<i64, OperationExecutionError> {
        let mut it = ops.into_iter();
        let Some(first) = it.next() else {
            return Err(OperationExecutionError::WrongOperandCount);
        };

        // TODO: This doesn't check whether the number of given arguments is correct yet
        match self {
            Self::Factorial => factorial(first),
            Self::Addition => it.try_fold(first, |acc, x| {
                acc.checked_add(x).ok_or(OperationExecutionError::Overflow)
            }),
            Self::Division => todo!(),
            Self::Multiplication => todo!(),
            Self::Subtraction => todo!(),
            Self::Power => todo!(),
            Self::Root => todo!(),
        }
    }

    fn execute_float(
        &self,
        ops: impl IntoIterator<Item = f64>,
    ) -> Result<f64, OperationExecutionError> {
        todo!()
    }
}
