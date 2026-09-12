use crate::{
    fold_expr::FoldExpr,
    io::Key,
    operation::{Operation, OperationExecutionError},
    terminal::Terminal,
};

pub struct Editor {
    buf: Vec<char>,
    mode: EditorMode,

    cur_pos: usize,
    cur_operand: Option<usize>,

    fold_expr: FoldExpr,
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
            fold_expr: FoldExpr {
                operation: Operation::Addition,
                operands: Vec::new(),
            },
            last_result: None,
            width,
            height,
        }
    }

    pub fn update(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match self.mode {
            EditorMode::Insert => self.handle_insert(key),
            EditorMode::Normal => self.handle_normal(key),
        }
    }

    fn handle_insert(&mut self, key: Key) -> Result<(), OperationExecutionError> {
        match key {
            Key::Escape => {
                self.mode = EditorMode::Normal;
            },
            Key::Char(ch) if let Ok(op) = Operation::try_from(ch) => {
                self.fold_expr.operation = op;
            },
            Key::Enter => {
                // evaluate and display result (local)
                todo!()
            },
            Key::Backspace => {
                self.buf[self.cur_pos] = ' ';
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
                todo!()
            },
            _ => ()
        }

        Ok(())
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
