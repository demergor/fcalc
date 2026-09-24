use core::fmt;
use std::{error::Error, fmt::Write};

use crate::operation::Operation;

pub const COEFF_DELIM: char = '\'';

pub struct Function {
    params: Vec<Term>,
    expr: Expression,
}

pub struct Expression {
    operation: Operation,
    operands: Vec<Term>,
    cur_operand: Option<usize>,
}

impl Expression {
    pub fn write_chars(&self, buf: &mut Vec<char>) -> Result<(), Box<dyn Error>> {
        buf.clear();
        let mut writer = CharWriter { buf };

        write!(writer, "{} ", self.operation.as_char())?;
        for term in &self.operands {
            write!(
                writer,
                "{}{COEFF_DELIM}{} ",
                term.coeff,
                if let Some(name) = term.name.clone() {
                    name
                } else {
                    String::from("")
                }
            )?;
        }

        Ok(())
    }
}

impl Default for Expression {
    fn default() -> Self {
        Self {
            operation: Operation::Addition,
            operands: Vec::new(),
            cur_operand: None,
        }
    }
}

struct Term {
    pub name: Option<String>,
    pub coeff: f64,
}

struct CharWriter<'a> {
    buf: &'a mut Vec<char>,
}

impl fmt::Write for CharWriter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buf.extend(s.chars());
        Ok(())
    }
}
