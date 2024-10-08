use chumsky::prelude::*;

#[derive(Debug, PartialEq, Clone)]
struct Field {
    name: String,
    type_: String,
}

/// Properties for objects (Structs and Enums)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectProperties {
    Public,
    Export,
}

/// `Derivable` methods for objects (Structs and Enums)
#[derive(Debug, Clone, PartialEq, Eq)]
enum ObjectMethods {
    Eq,
    Log,
    Custom(String),
}

#[derive(Debug, PartialEq, Clone)]
struct Struct {
    name: String,
    fields: Vec<Field>,
    props: Vec<ObjectProperties>,
    derives: Vec<ObjectMethods>,
}

const RESERVED_KEYWORDS: [&str; 2] = ["is", "derives"];

fn struct_parser() -> impl Parser<char, Struct, Error = Simple<char>> {
    let ident = text::ident()
        .try_map(|s: String, span| {
            if s != "is" && s != "derives" {
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
        just("Public").to(ObjectProperties::Public),
        just("Debug").to(ObjectProperties::Export),
    ));

    let struct_derives = choice((
        just("Eq").to(ObjectMethods::Eq),
        just("Log").to(ObjectMethods::Log),
    ))
    .or(ident.map(ObjectMethods::Custom));

    let properties = just("is")
        .ignore_then(struct_property.repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    let derives = just("derives")
        .ignore_then(struct_derives.repeated())
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
    let input = "struct Employee = id int :: salary int is Public ThreadSafe derives Log;";
    match struct_parser().parse(input) {
        Ok(struct_def) => println!("{:?}", struct_def),
        Err(e) => println!("Error: {:?}", e),
    }
}
