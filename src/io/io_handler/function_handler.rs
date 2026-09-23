use std::{error::Error, io};

use crate::terminal::Terminal;

pub struct FunctionHandler {
    cur_col: usize,
    input_buf: Vec<char>,

    term_width: u16,
    term_height: u16,
}

impl FunctionHandler {
    pub fn new(bounds: &Terminal) -> Result<FunctionHandler, io::Error> {
        todo!();
    }

    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        todo!();
    }
}
