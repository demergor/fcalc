use std::{error::Error, io};

use crate::{function::Expression, terminal::Terminal};

pub struct FunctionHandler {
    exprs: Vec<Expression>,
    lines: Vec<Vec<char>>,
    cur_col: usize,

    term_width: u16,
    term_height: u16,
}

impl FunctionHandler {
    pub fn new(bounds: &Terminal) -> Result<FunctionHandler, Box<dyn Error>> {
        let root_expr = Expression::default();
        let mut buf = Vec::new();
        root_expr.write_chars(&mut buf)?;

        Ok(FunctionHandler {
            exprs: vec![root_expr],
            lines: vec![buf],
            cur_col: 0,

            term_width: bounds.width,
            term_height: bounds.height,
        })
    }



    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        todo!();
    }
}
