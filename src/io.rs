use std::io::{Read, Stdin};

pub enum Key {
    Char(u8),

    Enter,
    Escape,

    ArrowUp,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
}

enum ParseState {
    Csi,
    Escape,
    Normal,
}

struct IoParser {
    stdin: Stdin,
    state: ParseState,
}

impl IoParser {
    // TODO: Add some kind of timer for `Escape`-case
    // TODO: Maybe add some kind of queue to "remember failed csi-sequences/escape" and not 
    // just discard them at failure
    fn next_key(&mut self) -> Option<Key> {
        let mut buf = [0u8; 1];
        self.stdin.read_exact(&mut buf).ok()?;

        match self.state {
            ParseState::Normal => {
                match buf[0] {
                    0x1b => {
                        self.state = ParseState::Escape; 
                        None
                    },
                    ch => Some(Key::Char(ch)),
                }
            },
            ParseState::Escape => {
                match buf[0] {
                    b'[' => {
                        self.state = ParseState::Csi;
                        None
                    },
                    0x1b => {
                        Some(Key::Escape)
                    },
                    ch => {
                        self.state = ParseState::Normal; 
                        Some(Key::Char(ch))
                    },
                }
            },
            ParseState::Csi => {
                self.state = ParseState::Normal;
                match buf[0] {
                    b'A' => Some(Key::ArrowUp),
                    b'B' => Some(Key::ArrowDown),
                    b'C' => Some(Key::ArrowRight),
                    b'D' => Some(Key::ArrowLeft),
                    _ => None,
                }
            },
        }


    }
}
