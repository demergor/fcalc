use std::{error::Error, io};

use crate::{function::{FuncMap, Function}, terminal::Terminal};

pub struct FunctionHandler {
    func_map: FuncMap,
    funcs: Vec<Function>,
    lines: Vec<Vec<char>>,
    cur_col: usize,

    term_width: u16,
    term_height: u16,
}

impl FunctionHandler {
    pub fn new(bounds: &Terminal) -> Result<FunctionHandler, Box<dyn Error>> {
        todo!()
            /*
        let root_expr = Function::default();
        let mut buf = Vec::new();
        root_expr.write_chars(&mut buf)?;

        Ok(FunctionHandler {

            exprs: vec![root_expr],
            lines: vec![buf],
            cur_col: 0,

            term_width: bounds.width,
            term_height: bounds.height,
        })
            */
    }



    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
