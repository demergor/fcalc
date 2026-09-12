mod char_buf;
mod fold_expr;
mod io;
mod operation;
mod terminal;

use std::error::Error;
use std::io::stdin;

use crate::fold_expr::FoldExpr;
use crate::terminal::Terminal;

use crate::char_buf::CharBuf;

fn main() -> Result<(), Box<dyn Error>> {
    todo!()
    /*
    loop {
        println!("Please enter your fold expression below:");
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .expect("Failed to read input!");
        let fold_expression =
            FoldExpr::try_from(input.chars().collect::<Vec<char>>().as_slice())?;
        let result = fold_expression.evaluate()?;
        println!("{fold_expression:?}");
        println!("{result}");
    }
    */

    /*
    let Ok(terminal) = Terminal::new() else {
        eprintln!("Error preparing the terminal!");
        return;
    };
    let cbuf = CharBuf::new(&terminal);

    let height = terminal.height;
    let width = terminal.width;

    drop(terminal);

    println!("Terminal height: {}\nTerminal width: {}", height, width);
    */
}
