mod parser;

use std::env;
use std::error::Error;
use std::fs;
use std::time::Instant;

use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::parser::*;

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
    // Start timer
    let now = Instant::now();
    println!("input file is: \n{}", program_root);
    // Parse the file
    match parse_source(&program_root) {
        Ok(struct_def) => {
            let elapsed = now.elapsed();
            println!("parsed file in {:.2?}", elapsed);
            println!("{:#?}", struct_def);
        }
        Err(e) => {
            println!("{:#?}", e);
            let report = Report::build(ReportKind::Error, file, 0)
                .with_message("Failed to parse")
                .with_label(
                    Label::new((file, 0..program_root.len()))
                        .with_message("in this line")
                        .with_color(Color::Red),
                )
                .with_labels(e.into_iter().map(|e| {
                    let expected = if e.expected().next().is_some() {
                        e.expected()
                            .map(|expected| match expected {
                                Some(c) => format!("'{}'", c),
                                None => "end of program_root".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    } else {
                        "a valid identifier".to_string() // Provide a fallback
                    };

                    let found = e
                        .found()
                        .map(|c| format!("'{}'", c))
                        .unwrap_or_else(|| "<something else>".to_string());

                    Label::new((file, e.span().start..e.span().end))
                        .with_message(format!("Expected {}, found {}", expected, found))
                        .with_color(Color::Yellow)
                }))
                .finish();

            report.eprint((file, Source::from(program_root))).unwrap();
        }
    }
    Ok(())
}
