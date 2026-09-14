use std::cmp::{max, min};

use crate::{
    fold_expr::{FoldExpr, FoldExprArena},
    io::Key,
    operation::{Operation, OperationExecutionError},
    terminal::{self, Terminal},
};

pub struct IoHandler {
    buf: Vec<char>,
    mode: IoHandlerMode,
    cur_pos: usize,

    fold_exprs: FoldExprArena,
    open_fold_exprs: Vec<FoldExprPosData>,
    last_result: Option<FoldExpr>,

    width: usize,
    height: usize,
}

impl IoHandler {
    pub fn new(bounds: &Terminal) -> IoHandler {
        let height = usize::from(bounds.height);
        let width = usize::from(bounds.width);

        IoHandler {
            buf: vec![' '; width * height],
            mode: IoHandlerMode::Normal,
            cur_pos: 0,
            fold_exprs: FoldExprArena::new(),
            open_fold_exprs: Vec::new(),
            last_result: None,
            width,
            height,
        }
    }

    pub fn update(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match self.mode {
            IoHandlerMode::Insert => self.handle_insert(key)?,
            IoHandlerMode::Normal => self.handle_normal(key)?,
        }

        self.render();
        Ok(())
    }

    fn handle_insert(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Escape => {
                self.mode = IoHandlerMode::Normal;
            }
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                let Some(fe_data) = self.open_fold_exprs.last() else {
                    let id = self.fold_exprs.init(op);
                    self.open_fold_exprs.push(FoldExprPosData {
                        id: id,
                        start_pos: self.cur_pos,
                        end_pos: self.cur_pos + 1,
                        cur_operand: None,
                    });

                    return Ok(());
                };

                self.fold_exprs.buf[fe_data.id].operation = op;
            }
            Key::Enter => {
                if self.open_fold_exprs.is_empty() {
                    return Ok(());
                }

                let id = self.open_fold_exprs.last().unwrap().id;
                let result = self.fold_exprs.evaluate(id)?;
                self.display_local_result(result);
            }
            Key::Backspace => {
                if self.open_fold_exprs.is_empty() {
                    return Ok(());
                };

                if !self.buf[self.cur_pos].is_whitespace() {
                    self.buf[self.cur_pos] = ' ';
                }

                // TODO: Call `FoldExprArena::update` to re-parse the expression and
                // display/render the new result
                // TODO: Check bounds
                // TODO: Shift everything in the bufline to the left, not just the cursor
                self.cur_pos -= 1;
            }
            Key::ArrowRight => {
                self.cur_pos = min(self.buf.len() - 1, self.cur_pos + 1);
                // self.buf.len() 
            }
            Key::ArrowLeft => {
                self.cur_pos = max(0, self.cur_pos - 1);
            }
            _ => (),
        }

        Ok(())
    }

    fn handle_normal(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Char('i') => {
                self.mode = IoHandlerMode::Insert;
            }
            Key::Char('c') => {
                // clear
                todo!()
            }
            Key::Char('e') => {
                // edit current operand
                let Some(&FoldExprPosData {
                    id,
                    cur_operand: Some(operand),
                    ..
                }) = self.open_fold_exprs.last()
                else {
                    return Ok(());
                };

                let child_id = self.fold_exprs.child_id(id, operand);
                self.under_cursor().len();

                self.scroll_down(2);

                // TODO: Handle placement of new inline `FoldExpr` (think about what
                // should happen when there is no more vertical space for a new expr)

                // TODO: How to decide what start_pos and what end_pos should be?
                /*
                self.open_fold_exprs.push(FoldExprPosData {
                    id: expr_data.id,
                    start_pos: expr_data.start_pos + 2 * self.width, // Or whatever
                    end_pos: expr_data.end_pos,
                });
                */
                todo!()
            }
            Key::Char('q') => {
                // exit current state
                todo!()
            }
            Key::Char('=') => {
                // evaluate and display result (global)
                let Some(root_id) = self.fold_exprs.root_id() else {
                    self.display_info("No expression to evaluate");
                    return Ok(());
                };

                let result = self.fold_exprs.evaluate(root_id)?;
                self.display_global_result(result);
            }
            _ => (),
        }

        Ok(())
    }

    fn render(&self) {
        print!("{}", terminal::SAVE_CURSOR_POS);
        todo!();
        print!("{}", terminal::RESTORE_CURSOR_POS);
    }

    fn display_info(&self, _s: &str) {
        todo!();
    }

    fn display_global_result(&self, _value: f64) {
        todo!();
    }

    fn display_local_result(&self, _value: f64) {
        todo!();
    }

    fn under_cursor(&self) -> std::ops::Range<usize> {
        if self.buf[self.cur_pos].is_whitespace() {
            return self.cur_pos..self.cur_pos;
        }

        let mut start = self.cur_pos;
        while start >= 0 && self.buf[start].is_whitespace() {
            start -= 1;
        }

        let mut end = self.cur_pos;
        for pos in self.cur_pos..self.buf.len() {
            if self.buf[pos].is_whitespace() {
                end = pos;
                break;
            }
        }

        start..end
    }

    fn clear(&mut self) {
        self.buf.fill(' ');
        self.cur_pos = 0;
    }
}

struct FoldExprPosData {
    id: usize,
    start_pos: usize,
    end_pos: usize,
    cur_operand: Option<usize>,
}

enum IoHandlerMode {
    Insert,
    Normal,
}
