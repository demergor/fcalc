use core::fmt;
use std::{
    collections::BTreeMap,
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufReader},
    ops,
};

use crate::operation::{self, Operation, OperationExecutionError};

pub const COEFF_DELIM: char = '\'';
pub const INSERTION_PREFIX: &[char] = &['F', 'U', 'N', ' '];
pub const DELETION_PREFIX: &[char] = &['D', 'E', 'L', ' '];

pub struct FuncMap {
    pub map: BTreeMap<String, Function>,
}

impl FuncMap {
    pub fn new() -> Result<FuncMap, io::Error> {
        let mut func_map = FuncMap {
            map: BTreeMap::new(),
        };

        func_map.populate_from_config()?;

        Ok(func_map)
    }

    pub fn handle(&mut self, line: String) -> HandleResult {
        if let Some((func_name, func)) = parse_ins(&line) {
            return if self.map.insert(func_name, func).is_none() {
                HandleResult::Insertion
            } else {
                HandleResult::Update
            };
        }

        if let Some(func_name) = parse_del(&line) {
            return if self.map.remove(&func_name).is_some() {
                HandleResult::DeletionSuccess
            } else {
                HandleResult::DeletionFail
            };
        }

        HandleResult::GenericFail
    }

    fn populate_from_config(&mut self) -> io::Result<()> {
        let file = match File::open(crate::func_config_path()) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err),
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            self.handle(line?);
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq)]
pub struct Function {
    params: Vec<(String, Option<f64>)>,
    operation: Operation,
    operands: Vec<Term>,
    cur_operands: ops::Range<usize>,
}

impl Function {
    pub fn evaluate(&mut self, args: Vec<f64>) -> Result<f64, FunctionError> {
        self.substitute(args)?;
        Ok(self.operation.execute(
            &self
                .operands
                .iter()
                .map(|term| term.coeff)
                .collect::<Vec<_>>(),
        )?)
    }

    pub fn reduce(&mut self) {
        todo!()
    }

    pub fn reparse(
        &mut self,
        slice: &[char],
        cursor_pos: usize,
    ) -> Result<(), FunctionError> {
        let (params, operation, operands, cur_operand) = parse_func(slice, cursor_pos)?;
        self.params = params;
        self.operation = operation;
        self.operands = operands;
        self.cur_operands = cur_operand;

        Ok(())
    }

    pub fn write_chars(&self, buf: &mut Vec<char>) -> fmt::Result {
        buf.clear();
        let mut writer = CharWriter { buf };
        self.write_fmt(&mut writer)?;

        Ok(())
    }

    pub fn reverse(&mut self) {
        if self.operands.is_empty() {
            return;
        }

        self.operands.reverse();
        self.cur_operands = (self.operands.len() - 1)..self.operands.len();
    }

    pub fn new_from_cur_operands(&self) -> Option<Function> {
        if self.cur_operands.is_empty() {
            return None;
        }

        let mut new_func = Function::default();
        new_func.operation = Operation::Addition;
        new_func
            .operands
            .extend(self.operands[self.cur_operands.clone()].iter().cloned());

        Some(new_func)
    }

    pub fn param_names(&self) -> Vec<String> {
        self.params.iter().map(|(name, _)| name.clone()).collect()
    }

    pub fn cur_operands(&self) -> ops::Range<usize> {
        self.cur_operands.clone()
    }

    pub fn next_operands(&mut self) -> ops::Range<usize> {
        if self.cur_operands.is_empty() {
            return self.cur_operands.clone();
        }

        self.cur_operands.start = (self.cur_operands.start + 1) % self.operands.len();
        self.cur_operands.end = (self.cur_operands.end + 1) & self.operands.len();

        self.cur_operands.clone()
    }

    pub fn previous_operands(&mut self) -> ops::Range<usize> {
        if self.cur_operands.is_empty() {
            return self.cur_operands.clone();
        }

        self.cur_operands.start = if self.cur_operands.start == 0 {
            self.operands.len() - 1
        } else {
            self.cur_operands.start - 1
        };

        self.cur_operands.end = if self.cur_operands.end == 0 {
            self.operands.len() - 1
        } else {
            self.cur_operands.end - 1
        };

        self.cur_operands.clone()
    }

    pub fn cur_operand_to_last(&mut self) {
        if self.cur_operands.is_empty() {
            return;
        }

        self.cur_operands = self.operands.len() - 1..self.operands.len();
    }

    pub fn change_operands(&mut self, other: &Function) {
        self.cur_operands =
            self.cur_operands.start..(self.cur_operands.start + other.operands.len());
        self.operands
            .splice(self.cur_operands.clone(), other.operands.clone());

        self.params.extend(other.params.clone());
        self.params.sort_by(|a, b| a.0.cmp(&b.0));
        self.params.dedup();
    }

    fn substitute(&mut self, args: Vec<f64>) -> Result<(), FunctionError> {
        if args.len() != self.params.len() {
            return Err(FunctionError::ArgumentMismatch);
        }

        for (idx, arg) in args.iter().enumerate() {
            self.params[idx].1 = Some(*arg);
        }

        for term in self.operands.iter_mut() {
            for var_name in &term.vars {
                let Some(pos) = self.params.iter().position(|(key, _)| key == var_name)
                else {
                    panic!("Function's parameters contain unknown variable names!");
                };

                term.coeff *= self.params[pos].1.unwrap();
            }
        }

        Ok(())
    }

    fn write_fmt<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        write!(writer, "{} ", self.operation.as_char())?;
        for term in &self.operands {
            if term.coeff != 1.0 || term.vars.is_empty() {
                write!(writer, "{}", term.coeff)?;
            }

            for var_name in &term.vars {
                write!(writer, "{COEFF_DELIM}{}", var_name,)?;
            }

            write!(writer, " ")?;
        }

        Ok(())
    }
}

impl Default for Function {
    fn default() -> Self {
        Self {
            params: Vec::new(),
            operation: Operation::Addition,
            operands: Vec::new(),
            cur_operands: 0..0,
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub vars: Vec<String>,
    pub coeff: f64,
}

pub enum HandleResult {
    Insertion,
    DeletionFail,
    DeletionSuccess,
    Update,
    GenericFail,
}

#[derive(Debug)]
pub enum FunctionError {
    ArgumentMismatch,
    OperationError(OperationExecutionError),
    ParseError(usize, char),
    ParseOperationError(operation::ParseOperationError),
}

impl From<OperationExecutionError> for FunctionError {
    fn from(err: OperationExecutionError) -> Self {
        Self::OperationError(err)
    }
}

impl From<operation::ParseOperationError> for FunctionError {
    fn from(err: operation::ParseOperationError) -> Self {
        Self::ParseOperationError(err)
    }
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

fn parse_func(
    slice: &[char],
    cursor_pos: usize,
) -> Result<
    (
        Vec<(String, Option<f64>)>,
        Operation,
        Vec<Term>,
        ops::Range<usize>,
    ),
    FunctionError,
> {
    let start = slice.iter().position(|ch| !ch.is_whitespace());
    let end = slice.iter().rposition(|ch| !ch.is_whitespace());

    let slice = match (start, end) {
        (Some(start), Some(end)) => &slice[start..=end],
        _ => &[],
    };

    let mut idx = 0;
    let mut it = slice.iter();

    let operation = match it.next() {
        Some(&ch) => Operation::try_from(ch)?,
        None => return Err(FunctionError::ParseError(idx, ' ')),
    };

    let mut params = Vec::<String>::new();
    let mut operands = Vec::<Term>::new();
    let mut cur_operand = 0..0;

    let mut cur_var = String::from("");
    let mut cur_term_vars = Vec::new();
    let mut cur_num = 0.0;

    let mut in_num = false;
    let mut already_float = false;
    let mut already_delim = false;
    let mut negative = false;
    let mut comp_div = 1.0;
    let mut cursor_on_operand = false;

    for ch in it {
        idx += 1;
        match ch {
            ch if ch.is_whitespace() => {
                if !in_num && !already_delim {
                    continue;
                }

                if already_delim && cur_var.is_empty() && cur_term_vars.is_empty() {
                    return Err(FunctionError::ParseError(idx, *ch));
                }

                if already_delim && !cur_var.is_empty() {
                    params.push(cur_var.clone());
                    cur_term_vars.push(cur_var.clone());
                }

                cur_num /= comp_div;
                operands.push(Term {
                    vars: cur_term_vars.clone(),
                    coeff: if negative { -cur_num } else { cur_num },
                });

                cur_var.clear();
                cur_term_vars.clear();
                cur_num = 0.0;

                in_num = false;
                already_float = false;
                already_delim = false;
                negative = false;
                comp_div = 1.0;
            }
            &COEFF_DELIM => {
                if already_delim {
                    if cur_var.is_empty() {
                        return Err(FunctionError::ParseError(idx, '\''));
                    }

                    params.push(cur_var.clone());
                    cur_term_vars.push(cur_var.clone());
                    cur_var.clear();
                }

                if !in_num {
                    cur_num = 1.0;
                }

                already_delim = true;
            }
            ch if already_delim => cur_var.push(*ch),
            '.' if !already_float => already_float = true,
            '-' if !negative => negative = true,
            ch if let Some(digit) = ch.to_digit(10) => {
                cur_num = cur_num * 10.0 + digit as f64;
                comp_div *= if already_float { 10.0 } else { 1.0 };
                in_num = true;
            }
            &ch => return Err(FunctionError::ParseError(idx, ch)),
        }

        if cursor_pos == idx {
            cur_operand = operands.len()..operands.len() + 1;
            cursor_on_operand = in_num || already_delim;
        }
    }

    if already_delim && cur_var.is_empty() && cur_term_vars.is_empty() {
        return Err(FunctionError::ParseError(idx, '\''));
    }

    cur_operand = match cur_operand {
        _ if operands.is_empty() => 0..0,
        range if range == (0..1) => 0..1,
        range if range.is_empty() => operands.len() - 1..operands.len(),
        range => {
            if cursor_on_operand {
                range
            } else {
                range.start - 1..range.end - 1
            }
        }
    };

    if !in_num && !already_delim {
        params.sort();
        params.dedup();

        return Ok((
            params.iter().cloned().map(|name| (name, None)).collect(),
            operation,
            operands,
            cur_operand,
        ));
    }

    if !cur_var.is_empty() {
        assert!(already_delim);
        params.push(cur_var.clone());
        cur_term_vars.push(cur_var);
    }

    params.sort();
    params.dedup();

    cur_num /= comp_div;
    cur_num = if negative { -cur_num } else { cur_num };

    operands.push(Term {
        vars: cur_term_vars.clone(),
        coeff: cur_num,
    });

    Ok((
        params.iter().cloned().map(|name| (name, None)).collect(),
        operation,
        operands,
        cur_operand,
    ))
}

fn parse_ins(line: &str) -> Option<(String, Function)> {
    if !line.starts_with(INSERTION_PREFIX) {
        return None;
    }

    let func_name: String = line
        .chars()
        .skip(INSERTION_PREFIX.len())
        .take_while(|&ch| ch != ':')
        .collect();

    let func_def: Vec<char> = line
        .chars()
        .skip(INSERTION_PREFIX.len())
        .skip_while(|&ch| ch != ':')
        .skip(1)
        .collect();

    let mut func = Function::default();
    if func.reparse(&func_def, 0).is_err() {
        return None;
    }

    Some((func_name, func))
}

fn parse_del(line: &str) -> Option<String> {
    if !line.starts_with(DELETION_PREFIX) {
        return None;
    }

    let line: String = line.chars().skip(DELETION_PREFIX.len()).collect();
    if line.contains(' ') || line.contains(':') || line.contains(COEFF_DELIM) {
        return None;
    }

    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_parse() {
        assert!(parse_func(&Vec::new(), 0).is_err());
    }

    #[test]
    fn test_only_operation_parse() {
        let test_operation = Operation::Addition;
        let test_vec = vec![test_operation.as_char()];

        let expected_params: Vec<(String, Option<f64>)> = Vec::new();
        let expected_operation = test_operation;
        let expected_operands: Vec<Term> = Vec::new();
        let expected_cur_operand = 0..0;

        let actual = parse_func(&test_vec, 0).expect("Parsing failed unexpectedly!");
        assert_eq!(expected_params, actual.0);
        assert_eq!(expected_operation, actual.1);
        assert_eq!(expected_operands, actual.2);
        assert_eq!(expected_cur_operand, actual.3);
    }

    #[test]
    fn test_default_function_chars() {
        let mut actual = Vec::new();
        Function::default()
            .write_chars(&mut actual)
            .expect("Writing chars failed unexpectedly!");

        assert!(actual.len() == 2);
        assert!(Operation::try_from(actual[0]).is_ok());
        assert!(actual[1].is_whitespace());
    }

    #[test]
    fn test_valid_parse() {
        let expected_params: Vec<(String, Option<f64>)> =
            vec!["!n", "...", "0", "va-l", "var1", "x", "x.i", "z"]
                .iter()
                .map(|&s| (String::from(s), None))
                .collect();
        let expected_operation = Operation::Division;
        let expected_operands: Vec<Term> = vec![
            (Vec::new(), 5.0),
            (vec![String::from("x.i"), String::from("x")], 1.0),
            (vec![String::from("!n")], 2.1),
            (vec![String::from("var1")], 3.0),
            (vec![String::from("z")], 0.0),
            (vec![String::from("0")], 39.0),
            (vec![String::from("...")], -69.67),
            (vec![String::from("va-l")], -1521.0),
            (vec![String::from("x.i")], -1.0),
        ]
        .iter()
        .cloned()
        .map(|(vars, x)| Term {
            vars: vars,
            coeff: x,
        })
        .collect();
        let expected_cur_operand = 3..4;

        let test_vec: Vec<char> =
            "/ 5.' 1'x.i'x 2.1'!n 3'var1 0.00'z 39'0 - 69.67'... 1521-'va-l -'x.i"
                .chars()
                .collect();

        let actual = parse_func(&test_vec, 27).expect("Parsing failed unexpectedly!");

        assert_eq!(expected_params, actual.0, "Parameter mismatch!");
        assert_eq!(expected_operation, actual.1, "Operation mismatch!");
        assert_eq!(expected_operands, actual.2, "Operands mismatch!");
        assert_eq!(expected_cur_operand, actual.3, "Cur operand mismatch!");
    }
}
