use std::{
    collections::VecDeque,
    io::{self, Read, Stdin},
};

use crate::io::Key::Char;

const TIMEOUT_MS: i32 = 30;
const ESC: u8 = 0x1b;

pub enum Key {
    Char(char),

    Enter,
    Escape,

    ArrowUp,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
}

#[derive(PartialEq, Eq)]
enum ParseState {
    Csi,
    Escape,
    Utf8(u8),
    Normal,
}

struct IoParser {
    pub pending_keys: VecDeque<Key>,

    stdin: Stdin,
    state: ParseState,
    multi_byte_buf: Vec<u8>,
}

impl IoParser {
    // TODO: Add utf8 starting byte detection 
    fn poll_key(&mut self) {
        let Ok(ready) = byte_ready(TIMEOUT_MS) else {
            return;
        };

        if !ready {
            return;
        }

        let mut buf = [0u8; 1];
        if self.stdin.read_exact(&mut buf).is_err() {
            return;
        }

        match self.state {
            ParseState::Normal => match buf[0] {
                ESC => self.state = ParseState::Escape,
                // TODO: if unicode: transition to ParseState::Utf8 and return
                ch => self.pending_keys.push_back(Key::Char(char::from(ch))),
            },
            ParseState::Escape => match buf[0] {
                b'[' => self.state = ParseState::Csi,
                ESC => self.pending_keys.push_back(Key::Escape),
                ch => {
                    self.state = ParseState::Normal;
                    self.pending_keys.push_back(Key::Char(char::from(ch)));
                }
            },
            ParseState::Utf8(remaining_bytes) => {
                assert!(remaining_bytes <= 4 && remaining_bytes > 0);

                self.multi_byte_buf.push(buf[0]);
                if remaining_bytes > 1 {
                    self.state = ParseState::Utf8(remaining_bytes - 1);
                } else {
                    self.state = ParseState::Normal;
                    let Ok(str) = std::str::from_utf8(&self.multi_byte_buf) else {
                        self.multi_byte_buf.clear();
                        return;
                    };

                    let Some(multi_byte_ch) = str.chars().next() else {
                        return;
                    };

                    self.pending_keys
                        .push_back(Key::Char(multi_byte_ch));
                    self.multi_byte_buf.clear();
                };
            }
            ParseState::Csi => {
                match buf[0] {
                    b'A' => self.pending_keys.push_back(Key::ArrowUp),
                    b'B' => self.pending_keys.push_back(Key::ArrowDown),
                    b'C' => self.pending_keys.push_back(Key::ArrowRight),
                    b'D' => self.pending_keys.push_back(Key::ArrowLeft),
                    _ => (),
                }

                self.state = ParseState::Normal;
            }
        }
    }
}

fn byte_ready(timeout_ms: i32) -> io::Result<bool> {
    let mut pollfd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };

    let result = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(result > 0 && (pollfd.events & libc::POLLIN) != 0)
}
