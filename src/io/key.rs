#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
