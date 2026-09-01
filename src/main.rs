mod char_buf;
mod terminal;

use crate::terminal::Terminal;

use crate::char_buf::CharBuf;

fn main() {
    let Ok(terminal) = Terminal::new() else {
        eprintln!("Error preparing the terminal!");
        return;
    };
    let cbuf = CharBuf::new(&terminal);
    println!("Terminal height: {}\nTerminal width: {}", terminal.height, terminal.width);
}
