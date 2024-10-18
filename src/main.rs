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
    // Start timer
    let t_start = Instant::now();
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
    // println!("input file is: \n{}", program_root);
    let t_compile_start = Instant::now();
    // Lex
    let mut lexer = Lexer::new(file);
    lexer.lex(&program_root);
    let t_lexing_done = Instant::now();
    // Parse the file
    let mut parser = Parser::new(lexer.token_stream);
    let out = parser.parse_all();
    let t_parsing_done = Instant::now();
    // Display errors
    if out.output.is_none() {
        for diagnostic in out.diagnostics {
            println!("{}", diagnostic.display(&program_root));
        }
        return Err("could not compile due to parsing errors".into());
    }
    let ast = out.output.unwrap();
    let generated_code = codegen_c::write_all(file, ast.iter());
    let t_codegen_done = Instant::now();
    fs::write("gen/test_case.c", generated_code).expect("Unable to write file");
    let t_all = Instant::now();
    // Report on code timings
    println!(
        "finished in {:?}\nsub-timings\n > lexing: {:?}\n > parsing: {:?}\n > codegen: {:?}",
        t_all - t_start,
        t_lexing_done - t_compile_start,
        t_parsing_done - t_lexing_done,
        t_codegen_done - t_parsing_done
    );
    Ok(())
}
