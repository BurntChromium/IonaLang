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

    pub fn display(&self, source: &str) -> String {
        format!(
            "{:?} in {}:{}:{}\n{}",
            self.level,
            self.position.filename,
            self.position.line,
            self.position.column,
            create_rich_diagnostic_message(&self.position, source, &self.message)
        )
    }
}

/// Create a nice diagnostic message that includes the source code context
fn create_rich_diagnostic_message(position: &SourcePosition, input: &str, message: &str) -> String {
    let mut lines = input.lines();
    let mut buffer = String::new();

    // Get the line before
    if position.line > 0 {
        if let Some(line) = lines.nth(position.line - 1) {
            buffer.push_str(&format!(" {} |", position.line - 1));
            buffer.push_str(line);
            buffer.push('\n'); // Add a newline after the line
        }
    }

    // Get the primary line, and add an error message
    if let Some(line) = lines.next() {
        let align = format!(" {} |", position.line);
        buffer.push_str(&align);
        buffer.push_str(line);
        buffer.push('\n'); // Add a newline after the line
                           // Add spaces until we reach the column, then place a caret (`^`)
        let caret_position = " ".repeat(position.column + align.len()) + "^";
        buffer.push_str(&caret_position);
        buffer.push_str(message);
        buffer.push('\n');
    }

    // Get the line after
    if let Some(line) = lines.next() {
        buffer.push_str(&format!(" {} |", position.line + 1));
        buffer.push_str(line);
        buffer.push('\n'); // Add a newline after the line
    }
    buffer.push('\n');

    buffer
}
