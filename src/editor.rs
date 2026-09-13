use crate::{
    fold_expr::{FoldExpr, FoldExprArena},
    io::Key,
    operation::{Operation, OperationExecutionError},
    terminal::{self, Terminal},
};

pub struct Editor {
    buf: Vec<char>,
    mode: EditorMode,

    cur_pos: usize,
    cur_operand: Option<usize>,

    fold_exprs: FoldExprArena,
    cur_fold_expr_id: Option<usize>,
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
            cur_operand: None,
            fold_exprs: FoldExprArena::new(),
            cur_fold_expr_id: None,
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
            },
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                if let Some(id) = self.cur_fold_expr_id {
                    self.fold_exprs.buf[id].operation = op;
                } else {
                    self.cur_fold_expr_id = Some(self.fold_exprs.init(op));
                }
            },
            Key::Enter => {
                let Some(id) = self.cur_fold_expr_id else {
                    return Ok(());
                };

                let result = self.fold_exprs.evaluate(id)?;
                self.display_local_result(result);
            },
            Key::Backspace => {
                let Some(id) = self.cur_fold_expr_id else {
                    return Ok(());
                };

                if !self.buf[self.cur_pos].is_whitespace() {
                    self.buf[self.cur_pos] = ' ';

                }

                // TODO: Call `FoldExprArena::update` to re-parse the expression and
                // display/render the new result
                self.cur_pos -= 1;
            },
            Key::ArrowRight => {
                self.cur_pos -= 1;
            },
            Key::ArrowLeft => {
                self.cur_pos += 1;
            },
            _ => (),
        }

        Ok(())
    }

    fn handle_normal(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Char('i') => {
                self.mode = EditorMode::Insert;
            },
            Key::Char('e') => {
                // edit current operand
                todo!()
            },
            Key::Char('q') => {
                // exit current state
                todo!()
            },
            Key::Char('=') => {
                // evaluate and display result (global)
                let Some(root_id) = self.fold_exprs.root_id() else {
                    self.display_info("No expression to evaluate");
                    return Ok(());
                };

                let result = self.fold_exprs.evaluate(root_id)?;
                self.display_global_result(result);
            },
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
