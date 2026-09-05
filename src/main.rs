mod char_buf;
mod operation;
mod terminal;

use crate::terminal::Terminal;

use crate::char_buf::CharBuf;

fn main() {
    let Ok(terminal) = Terminal::new() else {
        eprintln!("Error preparing the terminal!");
        return;
    };
    let cbuf = CharBuf::new(&terminal);

    let height = terminal.height;
    let width = terminal.width;

    drop(terminal);

    println!("Terminal height: {}\nTerminal width: {}", height, width);
}
