use crate::{io::Key, terminal::Terminal};

pub struct CharBuf {
    buf: Vec<char>,

    cur_pos: usize,
    eval_start_pos: usize,

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

    pub fn update(&mut self, key: Key) -> usize {
        // Might be needed to remember where the last change occurred 
        // to mark syntax errors
        let last_pos = self.cur_pos;
        match key {
            Key::Char(ch) => {
                self.buf[self.cur_pos] = ch;
                self.cur_pos += 1;
            }
            Key::Backspace => {
                self.buf[self.cur_pos] = ' ';
                self.cur_pos -= 1;
            }
            Key::ArrowRight => {
                self.cur_pos -= 1;
            }
            Key::ArrowLeft => {
                self.cur_pos += 1;
            }
            _ => (),
        }

        last_pos
    }

    fn clean(&mut self) {
        self.buf.fill(' ');
        self.cur_pos = 0;
    }
}
