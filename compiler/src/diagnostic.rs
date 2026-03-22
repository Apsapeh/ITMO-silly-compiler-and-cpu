use std::fmt::Display;

#[derive(Debug)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
}

#[derive(Debug)]
pub struct DiagnosticInfo {
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub col: usize,
}
pub struct Diagnostic {
    pub items: Vec<DiagnosticInfo>,
    has_fatal_err: bool,
}

impl Diagnostic {
    pub fn new() -> Self {
        Self {
            items: vec![],
            has_fatal_err: false,
        }
    }

    fn push_diag(
        &mut self,
        severity: Severity,
        message: impl Into<String>,
        line: usize,
        col: usize,
    ) {
        self.items.push(DiagnosticInfo {
            severity,
            message: message.into(),
            line,
            col,
        });
    }

    pub fn has_fatal(&self) -> bool {
        self.has_fatal_err
    }

    pub fn warning(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.push_diag(Severity::Warning, message, line, col);
    }

    pub fn error(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.push_diag(Severity::Error, message, line, col);
    }

    pub fn fatal(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.has_fatal_err = true;
        self.push_diag(Severity::Fatal, message, line, col);
    }

    pub fn flush(mut self) {
        self.items.sort_by_key(|d| (d.line, d.col));
        for diag in self.items {
            match diag.severity {
                Severity::Warning => {
                    eprintln!("warning [{}:{}]: {}", diag.line, diag.col, diag.message);
                }
                Severity::Error | Severity::Fatal => {
                    eprintln!("error [{}:{}]: {}", diag.line, diag.col, diag.message);
                }
            }
        }
    }
}
