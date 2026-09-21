struct VariableHandler {
    agency: Agency,
    cur_col: usize,
    input_buf: Vec<char>,
    term_width: u16,
    term_height: u16,
}

impl VariableHandler {
    fn handle(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match self.agency {
            Agency::Declaration => handle_decl(key),
            Agency::Selection => handle_select(key),
        }
    }

    fn handle_decl(&mut self, key: Key) -> Result<State, Box<dyn Error>> {
        match key {
            Key::Char('q') => {
                self.lines.pop();
                self.mode = Mode::Normal;
                self.render()?;

                self.cur_col = self.cur_pair()?.1.len();
                print!("\x1b[{}G", self.cur_col + 1);
                stdout().flush()?;

                return Ok(State::Continue);
            }
            Key::Char('h') => {
                // TODO: Add hints where appropriate
                print!(concat!(
                        "\x1b[H\x1b[2K\x1b[3mThe \"VAR \" prefix declares a new variable, ",
                        "\"DEL \" deletes an existing one\x1b[0m\n"
                ));
                stdout().flush()?;
            }
            Key::Enter => {
                let var_decl: String = self.cur_pair()?.1.iter().collect();
                // TODO: Change the `insert` signature to handle deletions as well
                let success = self.var_map.insert(var_decl);

                self.lines.pop();
                self.mode = Mode::Normal;
                self.render()?;

                // TODO: Make this display the correct operation that succeeded/failed
                // (Both a declaration or a deletion could have been attempted)
                if success {
                    print!("\x1b[H\x1b[42mVARIABLE DECLARATION SUCCESSFUL!\x1b[0m");
                } else {
                    print!("\x1b[H\x1b[41mVARIABLE DECLARATION FAILED!\x1b[0m");
                }

                stdout().flush()?;
                self.cur_col = self.cur_pair()?.1.len();
                self.confirm_enter = true;

                return Ok(State::Continue);
            }
            Key::Char(ch) => {
                let cur_col = self.cur_col;
                let cur_line = self.cur_pair()?.1;

                cur_line.insert(cur_col, ch);
                self.cur_col = if cur_col == self.cur_pair()?.1.len() {
                    self.cur_pair()?.1.len()
                } else {
                    self.cur_col + 1
                };
            }
            Key::Backspace => {
                if self.cur_col == 0 {
                    return Ok(State::Continue);
                }

                let cur_col = self.cur_col - 1;
                let cur_line = self.cur_pair()?.1;

                if cur_line.is_empty() {
                    return Ok(State::Continue);
                }

                cur_line.remove(cur_col);
                self.cur_col = cur_col;
            }
            Key::ArrowRight => {
                self.cur_col = min(self.cur_col + 1, self.cur_pair()?.1.len());
            }
            Key::ArrowLeft => {
                self.cur_col = if self.cur_col == 0 {
                    0
                } else {
                    self.cur_col - 1
                };
            }
            _ => (),
        }

        let line: String = self.cur_pair()?.1.iter().collect();
        print!("\r\x1b[2K{}\x1b[{}G", line, self.cur_col + 1);
        stdout().flush()?;

        Ok(State::Continue)
    }
}

enum Agency {
    Declaration,
    Selection,
}
