#[derive(Clone, Copy)]
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
