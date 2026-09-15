use std::io::{self, stdin, Read, Write};

use crate::{
    fold_expr::FoldExpr,
    terminal::{self, Terminal},
};

pub struct IoHandler {
    fold_exprs: Vec<FoldExpr>,
    cur_row: u16,

    term_width: u16,
    term_height: u16,
}

impl IoHandler {
    pub fn new(bounds: &Terminal) -> Result<IoHandler, IoError> {
        const CLEAR_LINE: &str = "\r\x1b[2K";
        const TIMEOUT_MS: i32 = 100;

        let mut buf = [0; 1];
        let mut stdout = std::io::stdout();
        let mut stdin = stdin();

        write!(stdout, "{}Determining cursor position...", CLEAR_LINE)?;
        while Terminal::byte_ready(10)? {
            stdin.read_exact(&mut buf)?;
        }

        write!(stdout, "\x1b[6n")?;
        stdout.flush()?;

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if buf[0] != terminal::ESC {
            return Err(IoError::MissingRowInformation);
        }

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if buf[0] != b'[' {
            return Err(IoError::MissingRowInformation);
        }

        if !Terminal::byte_ready(TIMEOUT_MS)? {
            return Err(IoError::MissingRowInformation);
        }

        stdin.read_exact(&mut buf)?;
        if !buf[0].is_ascii_digit() {
            return Err(IoError::MissingRowInformation);
        }

        let mut cur_row = u16::from(buf[0] - b'0');
        while buf[0] != b';' {
            if !Terminal::byte_ready(TIMEOUT_MS)? {
                return Err(IoError::MissingRowInformation);
            }

            stdin.read_exact(&mut buf)?;
            if !buf[0].is_ascii_digit() {
                return Err(IoError::MissingRowInformation);
            }

            cur_row = 10 * cur_row + u16::from(buf[0] - b'0');
        }

        write!(stdout, "{}Clearing stdin...", CLEAR_LINE)?;
        stdout.flush()?;

        while buf[0] != b'R' {
            if !Terminal::byte_ready(TIMEOUT_MS)? {
                return Err(IoError::MissingRowInformation);
            }

            stdin.read_exact(&mut buf)?;
        }

        write!(stdout, "{CLEAR_LINE}")?;

        Ok(IoHandler {
            fold_exprs: Vec::new(),
            cur_row,
            term_width: bounds.width,
            term_height: bounds.height,
        })
    }

    // TODO: Implement everything else
}

enum Mode {
    Insert,
    Normal,
}

enum IoError {
    MissingRowInformation,
    Io(io::Error),
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
