//! Combine the stages of compilation for repeated calls

use std::error::Error;
use std::fs;

use crate::lexer::Lexer;
use crate::parser::{ASTNode, Parser};

pub fn file_to_ast(filepath: &str, verbose: bool) -> Result<Vec<ASTNode>, Box<dyn Error>> {
    // Try to open linked file
    let maybe_text = fs::read_to_string(filepath);
    let program_text: String = if maybe_text.is_err() {
        return Err(format!("unable to find file {}, aborting compilation\n", filepath).into());
    } else {
        maybe_text.unwrap()
    };
    // Lex
    let mut lexer = Lexer::new(filepath);
    lexer.lex(&program_text);
    // Parse the file
    let mut parser = Parser::new(lexer.token_stream);
    let out = parser.parse_all();
    if !out.diagnostics.is_empty() {
        // out.output.is_none()
        let message_buffer = out
            .diagnostics
            .iter()
            .map(|d| d.display(&program_text))
            .collect::<String>();
        if verbose {
            eprintln!(
                "Parser stack trace (in code order, top-to-bottom)\n{:#?}",
                parser.unwind_stack()
            );
        }
        if out.output.is_none() {
            return Err(format!(
                "could not compile due to parsing error(s)\n\n{}",
                message_buffer
            )
            .into());
        } else {
            eprintln!("non-fatal errors\n{}", message_buffer);
            return Ok(out.output.unwrap());
        }
    } else {
        return Ok(out.output.unwrap());
    }
}
