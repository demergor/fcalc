struct FunctionHandler {
    agency: Agency,
    cur_col: usize,
    input_buf: Vec<char>,

    term_width: u16,
    term_height: u16,
}

enum Agency {
    Definition, 
    Selection,
}
