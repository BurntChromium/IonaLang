#![allow(dead_code)]

mod aggregation;
mod cli;
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

use aggregation::ParsingTables;
use cli::{Flags, Target};

/// Which standard library files should we NOT emit?
const NO_EMIT_LIST: [&'static str; 1] = ["arrays.iona"];

fn main() -> Result<(), Box<dyn Error>> {
    // Capture command line
    let args: Vec<String> = env::args().collect();
    let command = cli::parse_args(&args)?;
    let t_start = Instant::now();
    // Compile a normal target
    if let Target::Entrypoint(file) = command.target {
        let maybe_ast = pipeline::file_to_ast(&file, command.flags.contains(&Flags::Verbose));
        if let Err(e) = maybe_ast {
            eprint!("{}", e);
            std::process::exit(1);
        }
        let ast = maybe_ast.unwrap();
        // TEMP: handle std lib gen (will use pipeline later)
        let module_name = file
            .file_stem()
            .expect(&format!("unable to get file stem from filename {:?}", file))
            .to_string_lossy();
        let mut tables = ParsingTables::new();
        tables.update(&ast, &module_name);
        let filled_templates = codegen_c::generate_templated_libs(&tables.types);
        codegen_c::emit_templated_stdlib_files(&filled_templates);
        // Write file
        let generated_code =
            codegen_c::write_all(ast.iter(), &tables.types, file.to_str().unwrap(), true);
        fs::write("gen/test_case.c", generated_code).expect("Unable to write file");
        let t_all = Instant::now();
        // Report on code timings
        println!(
            "finished compiling {} in {:?}",
            &file.to_string_lossy(),
            t_all - t_start
        );
        return Ok(());
    }
    // Compile the standard library
    if let Target::StdLib = command.target {
        let paths = fs::read_dir("stdlib").expect("unable to find /stdlib/ directory in root");
        for path in paths {
            let file = path.unwrap();
            let entrypoint_filepath = &file.path();
            let maybe_ast = pipeline::file_to_ast(
                &entrypoint_filepath,
                command.flags.contains(&Flags::Verbose),
            );
            if let Err(e) = maybe_ast {
                eprint!("{}", e);
                std::process::exit(1);
            }
            let ast = maybe_ast.unwrap();
            // Check if we emit code for this
            if NO_EMIT_LIST.contains(&file.file_name().to_str().unwrap()) {
                // Report on code timings
                let t_all = Instant::now();
                println!(
                    "finished compiling {} in {:?}",
                    &file.file_name().to_str().unwrap(),
                    t_all - t_start
                );
                continue;
            }
            // TEMP: handle std lib gen (will use pipeline later)
            let module_name = entrypoint_filepath
                .file_stem()
                .expect(&format!(
                    "unable to get file stem from filename {:?}",
                    entrypoint_filepath
                ))
                .to_string_lossy();
            let mut tables = ParsingTables::new();
            tables.update(&ast, &module_name);
            let filled_templates = codegen_c::generate_templated_libs(&tables.types);
            codegen_c::emit_templated_stdlib_files(&filled_templates);
            // Write file
            let generated_code = codegen_c::write_all(
                ast.iter(),
                &tables.types,
                &file.path().file_stem().unwrap().to_string_lossy(),
                true,
            );
            let new_path = format!(
                "c_libs/gen_{}",
                file.file_name().to_str().unwrap().replace(".iona", ".h")
            );
            fs::write(new_path, generated_code).expect("Unable to write file");
            let t_all = Instant::now();
            // Report on code timings
            println!(
                "finished compiling {} in {:?}",
                &file.file_name().to_str().unwrap(),
                t_all - t_start
            );
        }
        Ok(())
    } else {
        return Err("impossible!".into());
    }
}
