use std::{
    collections::VecDeque,
    io::{self, Read, Stdin},
};

const TIMEOUT_MS: i32 = 30;
const ESC: u8 = b'\x1b';
const BACKSPACE: u8 = b'\x08';

// UTF8
const ONE_BYTE_TEST: u8 = 0b1000_0000;
const ONE_BYTE_EXPECTED: u8 = 0b0000_0000;
const TWO_BYTE_TEST: u8 = 0b1100_0000;
const TWO_BYTE_EXPECTED: u8 = 0b1110_0000;
const THREE_BYTE_TEST: u8 = 0b1111_0000;
const THREE_BYTE_EXPECTED: u8 = 0b1110_0000;
const FOUR_BYTE_TEST: u8 = 0b1111_0000;
const FOUR_BYTE_EXPECTED: u8 = 0b1111_0000;
const CONTINUATION_BYTE_TEST: u8 = 0b1100_0000;
const CONTINUATION_BYTE_EXPECTED: u8 = 0b1000_0000;

pub enum Key {
    Char(char),

    Enter,
    Backspace,
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
                byte if byte & TWO_BYTE_TEST == TWO_BYTE_EXPECTED => {
                    self.multi_byte_buf.push(byte);
                    self.state = ParseState::Utf8(1)
                },
                byte if byte & THREE_BYTE_TEST == THREE_BYTE_EXPECTED => {
                    self.multi_byte_buf.push(byte);
                    self.state = ParseState::Utf8(2)
                },
                byte if byte & FOUR_BYTE_TEST == FOUR_BYTE_EXPECTED => {
                    self.multi_byte_buf.push(byte);
                    self.state = ParseState::Utf8(3)
                },
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
                assert!(remaining_bytes <= 3 && remaining_bytes > 0);

                // Discard malformed utf8 code points
                if buf[0] & CONTINUATION_BYTE_TEST != CONTINUATION_BYTE_EXPECTED {
                    self.multi_byte_buf.clear();
                    self.state = ParseState::Normal;
                    return;
                }

                self.multi_byte_buf.push(buf[0]);
                if remaining_bytes > 1 {
                    self.state = ParseState::Utf8(remaining_bytes - 1);
                } else {
                    let Ok(s) = std::str::from_utf8(&self.multi_byte_buf) else {
                        self.multi_byte_buf.clear();
                        self.state = ParseState::Normal;
                        return;
                    };

                    let Some(ch) = s.chars().next() else {
                        return;
                    };

                    self.multi_byte_buf.clear();
                    self.state = ParseState::Normal;
                    self.pending_keys.push_back(Key::Char(ch));
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
