//! Combine the stages of compilation for repeated calls

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use crate::aggregation::ParsingTables;
use crate::lexer::Lexer;
use crate::parser::{ASTNode, Parser};

pub fn file_to_ast(filepath: &Path, verbose: bool) -> Result<Vec<ASTNode>, Box<dyn Error>> {
    // Try to open linked file
    let maybe_text = fs::read_to_string(filepath);
    let program_text: String = if maybe_text.is_err() {
        return Err(format!("unable to find file {:?}, aborting compilation\n", filepath).into());
    } else {
        maybe_text.unwrap()
    };
    // Lex
    let mut lexer = Lexer::new(&filepath.to_string_lossy());
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

/// Recursively parse a file, check all of the modules it needs (imports), and then parse those modules too
fn parse_recursively(
    ast_map_handle: &mut HashMap<String, Vec<ASTNode>>,
    tables_handle: &mut ParsingTables,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    for (module, is_parsed) in tables_handle.modules.parsing_status.clone().iter() {
        if !*is_parsed {
            let new_path = Path::new(module);
            let module_name = new_path
                .file_stem()
                .expect(&format!(
                    "unable to get file stem from filename {:?}",
                    new_path
                ))
                .to_string_lossy();
            let new_nodes = file_to_ast(new_path, verbose)?;
            tables_handle.update(&new_nodes, &module_name);
            ast_map_handle.insert(module.to_string(), new_nodes);
            parse_recursively(ast_map_handle, tables_handle, verbose)?;
        }
    }
    Ok(())
}

pub fn parse_all_reachable(
    entrypoint_filepath: &Path,
    verbose: bool,
) -> Result<HashMap<String, Vec<ASTNode>>, Box<dyn Error>> {
    let mut output: HashMap<String, Vec<ASTNode>> = HashMap::new();
    let module_name = entrypoint_filepath
        .file_stem()
        .expect(&format!(
            "unable to get file stem from filename {:?}",
            entrypoint_filepath
        ))
        .to_string_lossy();
    let entrypoint_nodes = file_to_ast(entrypoint_filepath, verbose)?;
    let mut tables = ParsingTables::new();
    tables.update(&entrypoint_nodes, &module_name);
    // We don't need these nodes anymore so put them in the table
    output.insert(
        entrypoint_filepath.to_string_lossy().to_string(),
        entrypoint_nodes,
    );
    parse_recursively(&mut output, &mut tables, verbose)?;
    Ok(output)
}

// -------------------- Unit Tests --------------------

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::lexer::Lexer;
//     use crate::parser::Parser;

//     #[test]
//     fn parse_reachable() {

//     }
// }
