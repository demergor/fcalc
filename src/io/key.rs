#[derive(Clone, Copy, Debug)]
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
