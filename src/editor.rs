use crate::{
    fold_expr::{FoldExpr, FoldExprArena},
    io::Key,
    operation::{Operation, OperationExecutionError},
    terminal::{self, Terminal},
};

struct FoldExprPosData {
    id: &usize,
    start_pos: usize,
    end_pos: usize,
    cur_operand: Option<usize>,
}

pub struct Editor {
    buf: Vec<char>,
    mode: EditorMode,
    cur_pos: usize,

    fold_exprs: FoldExprArena,
    open_fold_exprs: Vec<FoldExprPosData>,
    last_result: Option<FoldExpr>,

    width: usize,
    height: usize,
}

impl Editor {
    pub fn new(bounds: &Terminal) -> Editor {
        let height = usize::from(bounds.height);
        let width = usize::from(bounds.width);

        Editor {
            buf: vec![' '; width * height],
            mode: EditorMode::Normal,
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
            EditorMode::Insert => self.handle_insert(key)?,
            EditorMode::Normal => self.handle_normal(key)?,
        }

        self.render();
        Ok(())
    }

    fn handle_insert(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Escape => {
                self.mode = EditorMode::Normal;
            }
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                let Some(fe_data) = self.open_fold_exprs.last() else {
                    let id = self.fold_exprs.init(op);
                    self.open_fold_exprs.push(FoldExprPosData {
                        id: &id,
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
                let result = self.fold_exprs.evaluate(*id)?;
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

                // TODO: Shift everything in the bufline to the left, not just the cursor
                self.cur_pos -= 1;
            }
            Key::ArrowRight => {
                self.cur_pos -= 1;
            }
            Key::ArrowLeft => {
                self.cur_pos += 1;
            }
            _ => (),
        }

        Ok(())
    }

    fn handle_normal(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Char('i') => {
                self.mode = EditorMode::Insert;
            }
            Key::Char('c') => {
                // clear
                todo!()
            }
            Key::Char('e') => {
                // edit current operand
                let Some(expr_data) = self.open_fold_exprs.last() else {
                    return Ok(());
                };

                let child_id =
                    self.fold_exprs.buf[expr_data.id].children[expr_data.cur_operand];

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

    fn clear(&mut self) {
        self.buf.fill(' ');
        self.cur_pos = 0;
    }
}

enum EditorMode {
    Insert,
    Normal,
}
