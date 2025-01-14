#![allow(dead_code)]

mod codegen_c;
mod diagnostics;
mod expression_parser;
mod lexer;
mod parser;
mod pipeline;

use std::env;
use std::error::Error;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    // Capture command line
    let args: Vec<String> = env::args().collect();
    let file: &str = if args.len() == 1 {
        "main.iona"
    } else {
        &args[1]
    };
    let verbose: bool = if args.len() == 3 && &args[2] == "-v" {
        true
    } else {
        false
    };
    let t_start = Instant::now();
    let maybe_ast = pipeline::file_to_ast(&file, verbose);
    if let Err(e) = maybe_ast {
        eprint!("{}", e);
        std::process::exit(1);
    }
    let ast = maybe_ast.unwrap();
    let generated_code = codegen_c::write_all(file, ast.iter());
    fs::write("gen/test_case.c", generated_code).expect("Unable to write file");
    let t_all = Instant::now();
    // Report on code timings
    println!("finished in {:?}", t_all - t_start,);
    Ok(())
}
