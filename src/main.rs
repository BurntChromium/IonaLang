mod parser;

use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::parser::*;

fn main() {
    let input = r#"
        import math with sqrt pow;
        struct Employee = id int :: salary int is Public derives Log;
        enum EmployeeType = Salaried Int | Contract
	        is Public
	        derives Log;
    "#;

    let file_id = "example.txt";

    match parse_source(input) {
        Ok(struct_def) => {
            println!("successfully parsed file");
            println!("{:#?}", struct_def);
        }
        Err(e) => {
            println!("{:#?}", e);
            let report = Report::build(ReportKind::Error, file_id, 0)
                .with_message("Failed to parse")
                .with_label(
                    Label::new((file_id, 0..input.len()))
                        .with_message("in this line")
                        .with_color(Color::Red),
                )
                .with_labels(e.into_iter().map(|e| {
                    let expected = if e.expected().next().is_some() {
                        e.expected()
                            .map(|expected| match expected {
                                Some(c) => format!("'{}'", c),
                                None => "end of input".to_string(),
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

                    Label::new((file_id, e.span().start..e.span().end))
                        .with_message(format!("Expected {}, found {}", expected, found))
                        .with_color(Color::Yellow)
                }))
                .finish();

            report.eprint((file_id, Source::from(input))).unwrap();
        }
    }
}
