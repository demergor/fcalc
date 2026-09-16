use std::{
    collections::VecDeque, io::{Read, Stdin},
};

use crate::{io::Key, terminal::{self, Terminal}};

const TIMEOUT_MS: i32 = 30;
const BACKSPACE: u8 = b'\x08';

// UTF8
const _ONE_BYTE_TEST: u8 = 0b1000_0000;
const _ONE_BYTE_EXPECTED: u8 = 0b0000_0000;
const TWO_BYTE_TEST: u8 = 0b1100_0000;
const TWO_BYTE_EXPECTED: u8 = 0b1110_0000;
const THREE_BYTE_TEST: u8 = 0b1111_0000;
const THREE_BYTE_EXPECTED: u8 = 0b1110_0000;
const FOUR_BYTE_TEST: u8 = 0b1111_0000;
const FOUR_BYTE_EXPECTED: u8 = 0b1111_0000;
const CONTINUATION_BYTE_TEST: u8 = 0b1100_0000;
const CONTINUATION_BYTE_EXPECTED: u8 = 0b1000_0000;

pub struct InputParser {
    pub pending_keys: VecDeque<Key>,

    stdin: Stdin,
    state: ParseState,
    multi_byte_buf: Vec<u8>,
}

impl InputParser {
    pub fn new() -> InputParser {
        InputParser {
            pending_keys: VecDeque::new(),
            stdin: std::io::stdin(),
            state: ParseState::Normal,
            multi_byte_buf: Vec::new(),
        }
    }

    pub fn poll_key(&mut self) {
        let Ok(ready) = Terminal::byte_ready(TIMEOUT_MS) else {
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
                terminal::ESC => self.state = ParseState::Escape,
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
                BACKSPACE => self.pending_keys.push_back(Key::Backspace),
                ch => self.pending_keys.push_back(Key::Char(char::from(ch))),
            },
            ParseState::Escape => match buf[0] {
                b'[' => self.state = ParseState::Csi,
                terminal::ESC => self.pending_keys.push_back(Key::Escape),
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

                    let mut it = s.chars();
                    let Some(ch) = it.next() else {
                        return;
                    };

                    if it.next().is_some() {
                        panic!("Malformed UTF-8 sequence!");
                    }

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

#[derive(PartialEq, Eq)]
enum ParseState {
    Csi,
    Escape,
    Utf8(u8),
    Normal,
}
