pub struct FunctionHandler {
    agency: Agency,
    cur_col: usize,
    input_buf: Vec<char>,

    term_width: u16,
    term_height: u16,
}

impl FunctionHandler {
    pub fn new() -> FunctionHandler {
        todo!();
    }
}

enum Agency {
    Definition, 
    Selection,
}
