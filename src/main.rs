use chumsky::prelude::*;
use ariadne::{Report, ReportKind, Source, Label, Color};

#[derive(Debug, PartialEq, Clone)]
struct Field {
    name: String,
    type_: String,
}

/// Properties for data types (Structs and Enums)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataProperties {
    Public,
    Export,
}

/// `Derivable` methods for data types (Structs and Enums)
#[derive(Debug, Clone, PartialEq, Eq)]
enum DataMethods {
    Eq,
    Log,
    Custom(String),
}

#[derive(Debug, PartialEq, Clone)]
struct Struct {
    name: String,
    fields: Vec<Field>,
    props: Vec<DataProperties>,
    derives: Vec<DataMethods>,
}

const RESERVED_KEYWORDS: [&str; 2] = ["is", "derives"];

fn struct_parser() -> impl Parser<char, Struct, Error = Simple<char>> {
    let ident = text::ident()
        .try_map(|s: String, span| {
            if !RESERVED_KEYWORDS.contains(&&s.as_str()) {
                Ok(s)
            } else {
                Err(Simple::custom(span, "Unexpected keyword"))
            }
        })
        .padded();

    let camel_case = filter(|c: &char| c.is_ascii_uppercase())
        .chain(filter(|c: &char| c.is_ascii_alphanumeric()).repeated())
        .collect::<String>()
        .padded();

    let field = ident.then(ident).map(|(name, type_)| Field { name, type_ });

    let fields = field.separated_by(just("::")).at_least(1);

    let struct_property = choice((
        text::keyword("Public").to(DataProperties::Public).labelled("Public"),
        text::keyword("Export").to(DataProperties::Export).labelled("Export"),
    ));

    let struct_derives = choice((
        text::keyword("Eq").to(DataMethods::Eq),
        text::keyword("Log").to(DataMethods::Log),
    ))
    .or(ident.map(DataMethods::Custom));

    let properties = just("is")
        .ignore_then(struct_property.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    let derives = just("derives")
        .ignore_then(struct_derives.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    just("struct")
        .ignore_then(camel_case)
        .then_ignore(just("="))
        .then(fields)
        .then(properties)
        .then(derives)
        .then_ignore(just(";"))
        .map(|(((name, fields), properties), derives)| Struct {
            name,
            fields,
            props: properties,
            derives,
        })
        .labelled("struct definition")
}

fn main() {
    let input = "struct Employee = id int :: salary int is Public derives Log;";
    let file_id = "example.txt";

    match struct_parser().parse(input) {
        Ok(struct_def) => println!("{:?}", struct_def),
        Err(e) => {
            let report = Report::build(ReportKind::Error, file_id, 0)
                .with_message("Failed to parse struct definition")
                .with_label(Label::new((file_id, 0..input.len()))
                    .with_message("in this struct definition")
                    .with_color(Color::Red))
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
                        "a valid identifier".to_string()  // Provide a fallback
                    };
                    
                    let found = e.found().map(|c| format!("'{}'", c)).unwrap_or_else(|| "<something else>".to_string());
                    
                    Label::new((file_id, e.span().start..e.span().end))
                        .with_message(format!("Expected {}, found {}", expected, found))
                        .with_color(Color::Yellow)
                }))
                .finish();

            report.eprint((file_id, Source::from(input))).unwrap();
        }
    }
}
