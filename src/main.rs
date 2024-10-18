#![allow(dead_code)]

mod codegen_c;
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
    let program_root: String = if maybe_text.is_err() {
        return Err(format!("unable to find file {}, aborting compilation", file).into());
    } else {
        maybe_text.unwrap()
    };
    println!("input file is: \n{}", program_root);
    // Start timer
    let now = Instant::now();
    // Lex
    let mut lexer = Lexer::new(file);
    lexer.lex(&program_root);
    println!("time elapsed: {:?}", Instant::now() - now);
    // Parse the file
    let mut parser = Parser::new(lexer.token_stream);
    let out = parser.parse_all();
    // Display errors
    if out.output.is_none() {
        for diagnostic in out.diagnostics {
            println!("{}", diagnostic.display(&program_root));
        }
        return Err("could not compile due to parsing errors".into());
    }
    println!("{:#?}", out.output);
    // let ast = [out.output.unwrap()];
    // fs::write("gen/test_case.c", codegen_c::write_all(file, ast.iter()))
    //     .expect("Unable to write file");
    Ok(())
}
