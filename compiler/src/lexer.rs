use crate::{
    diagnostic::Diagnostic,
    types::{self, NumWord},
};

const AVERAGE_WORD_LEN: usize = 4;

pub enum Token<'a> {
    FnDecl(NumWord<'a>),
    Other,
}

pub fn lex <'a> (source_code: &'a str, diag: &'a mut Diagnostic) -> Result<Vec<Token<'a>>, ()> {
    let mut lexer_splitter = LexerSplitter::new(source_code, diag);
    let spltted = lexer_splitter.run()?;

    let mut result = Vec::with_capacity(spltted.len());
    for num_word in spltted {
        let token = match num_word.word {
            "fn" => Token::FnDecl(num_word),
            _ => Token::Other,
        };
        result.push(token);
    }
    Ok(result)
}

struct WordStart {
    idx: usize,
    line: usize,
    col: usize,
}

impl WordStart {
    fn new(idx: usize, line: usize, col: usize) -> Self {
        Self { idx, line, col }
    }
}

#[derive(PartialEq, Eq)]
enum LexerSplitterState {
    Normal,
    InString,
    InSingleComment,
}

struct LexerSplitter<'a> {
    diag: &'a mut Diagnostic,
    src: &'a str,
    iter: std::iter::Peekable<std::str::CharIndices<'a>>,
    result: Vec<NumWord<'a>>,
    state: LexerSplitterState,
    word_start_line: usize,
    word_start_column: usize,
    word_start_idx: usize,
    cur_line: usize,
    cur_column: usize,
}

impl<'a> LexerSplitter<'a> {
    fn new(source_code: &'a str, diag: &'a mut Diagnostic) -> Self {
        Self {
            diag: diag,
            src: source_code,
            iter: source_code.char_indices().peekable(),
            result: Vec::with_capacity(source_code.len() / AVERAGE_WORD_LEN),
            state: LexerSplitterState::Normal,
            word_start_line: 1,
            word_start_column: 1,
            word_start_idx: 0,
            cur_line: 1,
            cur_column: 1,
        }
    }

    fn run(mut self) -> Result<Vec<NumWord<'a>>, ()> {
        while let Some((cur_idx, cur_ch)) = self.iter.next() {
            let cur_ch_is_alphanum = cur_ch.is_alphanumeric();
            let (peek_idx, peek_ch, peek_ch_is_alphanum) = match self.iter.peek() {
                Some(&p) => (p.0, p.1, p.1.is_alphanumeric()),
                None => (self.src.len(), '\0', !cur_ch_is_alphanum),
            };

            self.process(
                cur_idx,
                cur_ch,
                cur_ch_is_alphanum,
                peek_idx,
                peek_ch,
                peek_ch_is_alphanum,
            );

            self.cur_column += 1;
        }

        if self.state == LexerSplitterState::InString {
            self.diag.fatal(
                "unclosed string",
                self.word_start_line,
                self.word_start_column,
            );
            return Err(());
        }

        dbg!(&self.result);
        Ok(self.result)
    }

    fn process(
        &mut self,
        cur_idx: usize,
        cur_ch: char,
        cur_ch_is_alphanum: bool,
        peek_idx: usize,
        peek_ch: char,
        peek_ch_is_alphanum: bool,
    ) {
        let word_start = match self.state {
            LexerSplitterState::InSingleComment => {
                if let Some(r) = self.try_feed_line(cur_ch, peek_ch) {
                    self.state = LexerSplitterState::Normal;
                    Some(r)
                } else {
                    None
                }
            }

            LexerSplitterState::InString => {
                if cur_ch == '"' {
                    self.state = LexerSplitterState::Normal;
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                } else {
                    None
                }
            }

            LexerSplitterState::Normal => {
                // New line
                if let Some(r) = self.try_feed_line(cur_ch, peek_ch) {
                    self.push_word(cur_idx);
                    Some(r)
                // Single line comment
                } else if cur_ch == '/' && peek_ch == '/' {
                    self.push_word(cur_idx);
                    self.state = LexerSplitterState::InSingleComment;
                    None
                // String start
                } else if cur_ch == '"' {
                    self.push_word(cur_idx);
                    self.state = LexerSplitterState::InString;
                    Some(WordStart::new(cur_idx, self.cur_line, self.cur_column))
                // Some of separators (special chars)
                } else if matches!(cur_ch, ';' | '.' | ',' | '{' | '}' | '[' | ']' | '(' | ')') {
                    self.push_word(cur_idx);
                    self.word_start_idx = cur_idx;
                    self.word_start_line = self.cur_line;
                    self.word_start_column = self.cur_column;
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                //
                } else if cur_ch_is_alphanum != peek_ch_is_alphanum {
                    self.push_word(peek_idx);
                    Some(WordStart::new(peek_idx, self.cur_line, self.cur_column + 1))
                } else {
                    None
                }
            }
        };

        if let Some(ws) = word_start {
            self.word_start_idx = ws.idx;
            self.word_start_line = ws.line;
            self.word_start_column = ws.col;
        }
    }

    fn push_word(&mut self, word_end_idx: usize) {
        let mess_word = &self.src[self.word_start_idx..word_end_idx];
        let end_mess_word = mess_word.trim_start();

        if !end_mess_word.is_empty() {
            let num_word = NumWord::new(
                end_mess_word.trim_end(),
                self.word_start_line,
                self.word_start_column + mess_word.len() - end_mess_word.len(),
            );
            self.result.push(num_word);
        }
    }

    fn try_feed_line(&mut self, cur_ch: char, peek_ch: char) -> Option<WordStart> {
        if cur_ch == '\n' || cur_ch == '\r' {
            if cur_ch == '\r' && peek_ch == '\n' {
                self.iter.next();
            }

            self.cur_line += 1;
            self.cur_column = 0;
            Some(WordStart::new(
                self.iter.peek().unwrap_or(&(self.src.len(), 'a')).0,
                self.cur_line,
                self.cur_column,
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::Diagnostic;

    #[test]
    fn lex() {
        let source_code = r#"
  fn fib(n: u32) -> u32 {
      "Всем привет {} меня зовут


      Тофик"
      if n<2{return 1;}//Aboba
      return fib(n - 2) + fib(n - 1);
  }"#;
        let mut diag = Diagnostic::new();
        super::lex(source_code, &mut diag);

        if diag.has_fatal() {
            diag.flush();
        }
    }

    #[test]
    fn split_source_code() {
        //       let source_code = r#"
        // fn fib(n: u32) -> u32 {
        //     if n < 2 { return 1; }
        //     return fib(n - 2) + fib(n - 1);
        // }

        // fn strlen(string: &u32) -> u32 {
        //     let len: u32 = 0;
        //     while *string {
        //       string += 1;
        //     }
        //     return len;
        // }

        // // Entry point
        // fn main() {
        //     // Branches
        //     let number: u32 = 10;
        //     if number == 50 {
        //       print("If");
        //     } else if number == 10 {
        //       print("Elseif");
        //     } else {
        //       print("Else");
        //     }

        //     // Loops
        //     let counter: u32 = 10;
        //     while counter {
        //         print("");
        //         counter -= 1;
        //     }

        //     let n: u32 = 10;
        //     print(fib(n));
        // }"#;
        let source_code = r#"
  fn fib(n: u32) -> u32 {
      if n < 2 { return 1; } // Aboba
      return fib(n - 2) + fib(n - 1);
  }"#;
        // super::split_source_code(source_code);
    }
}
