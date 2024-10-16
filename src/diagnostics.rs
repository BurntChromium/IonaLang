//! Compiler Errors, Warnings, and Lints

use crate::lexer::SourcePosition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueLevel {
    Lint,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    level: IssueLevel,
    message: String,
    position: SourcePosition,
    references: Option<Vec<SourcePosition>>,
}

impl Diagnostic {
    pub fn new_error_simple(message: &str, position: &SourcePosition) -> Self {
        Diagnostic {
            level: IssueLevel::Error,
            message: message.to_string(),
            position: position.clone(),
            references: None,
        }
    }
}

/// Given a SourcePosition and an input file (string), get the context from the input file
fn get_context(position: &SourcePosition, input: &str) -> String {
    let mut lines = input.lines();
    let mut buffer = "".to_string();
    if position.line > 0 {
        buffer += lines.nth(position.line - 1).unwrap();
    }
    buffer += lines.next().unwrap();
    match lines.next() {
        Some(line) => {
            buffer += line;
        }
        None => {}
    }
    buffer
}
