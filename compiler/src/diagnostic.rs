#[derive(Debug, PartialEq)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
}

#[derive(Debug, PartialEq)]
pub struct DiagnosticInfo {
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub col: usize,
}
pub struct Diagnostic {
    pub items: Vec<DiagnosticInfo>,
    has_fatal_flag: bool,
    has_error_flag: bool,
    has_warning_flag: bool,
}

impl Diagnostic {
    pub fn new() -> Self {
        Self {
            items: vec![],
            has_fatal_flag: false,
            has_error_flag: false,
            has_warning_flag: false,
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
        self.has_fatal_flag
    }

    pub fn has_error(&self) -> bool {
        self.has_error_flag
    }

    pub fn has_warning(&self) -> bool {
        self.has_warning_flag
    }

    pub fn reset_flags(&mut self) {
        self.has_fatal_flag = false;
        self.has_error_flag = false;
        self.has_warning_flag = false;
    }

    pub fn is_clear(&self) -> bool {
        self.items.is_empty()
    }

    pub fn warning(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.push_diag(Severity::Warning, message, line, col);
        self.has_warning_flag = true;
    }

    pub fn error(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.push_diag(Severity::Error, message, line, col);
        self.has_error_flag = true;
    }

    pub fn fatal(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.has_fatal_flag = true;
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
