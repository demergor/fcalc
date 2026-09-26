use core::fmt;
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::Write,
    fs::File,
    io::{self, BufRead, BufReader},
};

use crate::operation::{self, Operation, OperationExecutionError};

pub const COEFF_DELIM: char = '\'';
pub const INSERTION_PREFIX: &[char] = &['F', 'U', 'N'];
pub const DELETION_PREFIX: &[char] = &['D', 'E', 'L'];

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

pub struct Function {
    params: Vec<(String, Option<f64>)>,
    operation: Operation,
    operands: Vec<Term>,
    cur_operand: Option<usize>,
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

    pub fn reparse(
        &mut self,
        slice: &[char],
        cursor_pos: usize,
    ) -> Result<(), FunctionError> {
        let (params, operation, operands, cur_operand) = parse_func(slice, cursor_pos)?;
        self.params = params;
        self.operation = operation;
        self.operands = operands;
        self.cur_operand = cur_operand;

        Ok(())
    }

    pub fn write_chars(&self, buf: &mut Vec<char>) -> Result<(), Box<dyn Error>> {
        buf.clear();
        let mut writer = CharWriter { buf };

        write!(writer, "{} ", self.operation.as_char())?;
        for term in &self.operands {
            write!(
                writer,
                "{}{COEFF_DELIM}{} ",
                term.coeff,
                if let Some(name) = term.var_name.clone() {
                    name
                } else {
                    String::from("")
                }
            )?;
        }

        Ok(())
    }

    pub fn cur_operand_to_last(&mut self) {
        if self.cur_operand.is_none() {
            return;
        }

        self.cur_operand = Some(self.operands.len() - 1);
    }

    fn substitute(&mut self, args: Vec<f64>) -> Result<(), FunctionError> {
        if args.len() != self.params.len() {
            return Err(FunctionError::ArgumentMismatch);
        }

        for (idx, arg) in args.iter().enumerate() {
            self.params[idx].1 = Some(*arg);
        }

        for term in self.operands.iter_mut() {
            let Some(var_name) = &term.var_name else {
                continue;
            };

            let Some(pos) = self.params.iter().position(|(key, _)| key == var_name)
            else {
                panic!("Function's parameters contain unknown variable names!");
            };

            term.coeff *= self.params[pos].1.unwrap();
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
            cur_operand: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Term {
    pub var_name: Option<String>,
    pub coeff: f64,
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
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
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
        Option<usize>,
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
    let mut cur_operand = None;

    let mut cur_var = String::from("");
    let mut cur_num = 0.0;

    let mut in_num = false;
    let mut already_float = false;
    let mut already_delim = false;
    let mut negative = false;
    let mut comp_div = 1.0;

    for ch in it {
        idx += 1;
        match ch {
            ch if ch.is_whitespace() => {
                if !in_num && !already_delim {
                    continue;
                }

                if already_delim && !cur_var.is_empty() {
                    params.push(cur_var.clone());
                }

                let var_name = if cur_var.is_empty() {
                    None
                } else {
                    Some(cur_var.clone())
                };

                cur_num /= comp_div;
                operands.push(Term {
                    var_name,
                    coeff: if negative { -cur_num } else { cur_num },
                });

                cur_var.clear();
                cur_num = 0.0;

                in_num = false;
                already_float = false;
                already_delim = false;
                negative = false;
                comp_div = 1.0;
            }
            &COEFF_DELIM => {
                if already_delim {
                    return Err(FunctionError::ParseError(idx, COEFF_DELIM));
                }

                if !in_num {
                    cur_num = 1.0;
                }

                already_delim = true;
            }
            ch if already_delim => cur_var.push(*ch),
            '.' => {
                if already_float {
                    return Err(FunctionError::ParseError(idx, '.'));
                }

                already_float = true;
            }
            '-' if !negative => negative = true,
            ch if let Some(digit) = ch.to_digit(10) => {
                cur_num = cur_num * 10.0 + digit as f64;
                comp_div *= if already_float { 10.0 } else { 1.0 };
                in_num = true;
            }
            &ch => return Err(FunctionError::ParseError(idx, ch)),
        }

        if cursor_pos == idx {
            cur_operand = Some(operands.len());
        }
    }

    cur_operand = match cur_operand {
        _ if operands.is_empty() => None,
        Some(idx) if idx == 0 => Some(0),
        Some(idx) => Some(idx - 1),
        None => Some(operands.len() - 1),
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
        params.push(cur_var.clone());
    }

    params.sort();
    params.dedup();

    let var_name = if cur_var.is_empty() {
        None
    } else {
        Some(cur_var)
    };

    cur_num /= comp_div;
    cur_num = if negative { -cur_num } else { cur_num };

    operands.push(Term {
        var_name,
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

pub enum HandleResult {
    Insertion,
    DeletionFail,
    DeletionSuccess,
    Update,
    GenericFail,
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
        let expected_cur_operand = None;

        let actual = parse_func(&test_vec, 0).expect("Parsing failed unexpectedly!");
        assert_eq!(expected_params, actual.0);
        assert_eq!(expected_operation, actual.1);
        assert_eq!(expected_operands, actual.2);
        assert_eq!(expected_cur_operand, actual.3);
    }

    #[test]
    fn test_valid_parse() {
        let expected_params: Vec<(String, Option<f64>)> =
            vec!["!n", "...", "0", "va-l", "var1", "x.i", "z"]
                .iter()
                .map(|&s| (String::from(s), None))
                .collect();
        let expected_operation = Operation::Division;
        let expected_operands: Vec<Term> = vec![
            (None, 5.0),
            (Some(String::from("x.i")), 1.0),
            (Some(String::from("!n")), 2.1),
            (Some(String::from("var1")), 3.0),
            (Some(String::from("z")), 0.0),
            (Some(String::from("0")), 39.0),
            (Some(String::from("...")), -69.67),
            (Some(String::from("va-l")), -1521.0),
            (Some(String::from("x.i")), -6666.1),
        ]
        .iter()
        .cloned()
        .map(|(var_name, x)| Term {
            var_name: var_name,
            coeff: x,
        })
        .collect();
        let expected_cur_operand = Some(3);

        let test_vec: Vec<char> =
            "/ 5.' 1'x.i 2.1'!n 3'var1 0.00'z 39'0 - 69.67'... 1521-'va-l -6666.1'x.i"
                .chars()
                .collect();

        let actual = parse_func(&test_vec, 25).expect("Parsing failed unexpectedly!");

        assert_eq!(expected_params, actual.0, "Parameter mismatch!");
        assert_eq!(expected_operation, actual.1, "Operation mismatch!");
        assert_eq!(expected_operands, actual.2, "Operands mismatch!");
        assert_eq!(expected_cur_operand, actual.3, "Cur operand mismatch!");
    }
}
