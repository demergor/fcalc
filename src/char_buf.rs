use crate::terminal::Terminal;

pub struct CharBuf {
    buf: Vec<char>,

    cur_pos: u16,
    eval_start_pos: u16,

    width: usize,
    height: usize,
}

impl CharBuf {
    pub fn new(bounds: &Terminal) -> CharBuf {
        let height = usize::from(bounds.height);
        let width = usize::from(bounds.width);

        CharBuf {
            buf: vec![' '; width * height],
            cur_pos: 0,
            eval_start_pos: 0,
            width,
            height,
        }
    }

    fn clean(&mut self) {
        self.buf.fill(' ');
        self.cur_pos = 0;
    }
}
