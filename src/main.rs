mod diagnostics;
mod lexer;
mod parser;

use std::env;
use std::error::Error;
use std::fs;
use std::time::Instant;

use lexer::Lexer;
use parser::Parser;

fn main() -> Result<(), Box<dyn Error>> {
    // Capture command line
    let args: Vec<String> = env::args().collect();
    let file: &str = if args.len() == 1 {
        "main.iona"
    } else {
        &args[1]
    };
    // Try to open linked file
    let maybe_text = fs::read_to_string(file);
    let program_root: String;
    if maybe_text.is_err() {
        return Err(format!("unable to find file {}, aborting compilation", file).into());
    } else {
        program_root = maybe_text.unwrap();
    }
    println!("input file is: \n{}", program_root);
    // Start timer
    let now = Instant::now();
    // Lex
    let mut lexer = Lexer::new(file);
    lexer.lex(&program_root);
    println!("time elapsed: {:?}", Instant::now() - now);
    // Parse the file
    let mut parser = Parser::new(lexer.token_stream);
    let out = parser.parse_struct_declaration();
    println!("{:#?}", out);
    Ok(())
}
