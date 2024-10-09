//! Parse

use chumsky::prelude::*;
use std::sync::OnceLock;

// -------------------- Shared Tools --------------------

pub const RESERVED_KEYWORDS: [&str; 6] = ["import", "is", "derives", "struct", "fn", "enum"];

static IDENT_PARSER: OnceLock<Box<dyn Parser<char, String, Error = Simple<char>> + Sync + Send>> =
    OnceLock::new();

fn get_ident_parser() -> &'static (dyn Parser<char, String, Error = Simple<char>> + Sync + Send) {
    IDENT_PARSER.get_or_init(|| {
        Box::new(
            text::ident()
                .try_map(|s: String, span| {
                    if !RESERVED_KEYWORDS.contains(&&s.as_str()) {
                        Ok(s)
                    } else {
                        Err(Simple::custom(span, "Unexpected keyword"))
                    }
                })
                .padded(),
        )
    })
}

static CAMEL_CASE: OnceLock<Box<dyn Parser<char, String, Error = Simple<char>> + Sync + Send>> =
    OnceLock::new();

fn get_camel_case_parser() -> &'static (dyn Parser<char, String, Error = Simple<char>> + Sync + Send)
{
    CAMEL_CASE.get_or_init(|| {
        Box::new(
            filter(|c: &char| c.is_ascii_uppercase())
                .chain(filter(|c: &char| c.is_ascii_alphanumeric()).repeated())
                .collect::<String>()
                .padded(),
        )
    })
}

// -------------------- AST --------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ASTNode {
    ImportStatement(Import),
    StructDeclaration(Object),
    EnumDeclaration(Object),
}

fn ast_parser() -> impl Parser<char, Vec<ASTNode>, Error = Simple<char>> {
    let import_node = import_parser().map(ASTNode::ImportStatement);
    let struct_node = struct_parser().map(ASTNode::StructDeclaration);
    let enum_node = enum_parser().map(ASTNode::EnumDeclaration);

    let node = import_node.or(struct_node).or(enum_node);

    node.padded().repeated().then_ignore(end())
}

pub fn parse_source(source: &str) -> Result<Vec<ASTNode>, Vec<Simple<char>>> {
    ast_parser().parse(source)
}

// -------------------- Imports --------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Import {
    file: String,
    items: Vec<String>,
}

fn import_parser() -> impl Parser<char, Import, Error = Simple<char>> {
    let ident = get_ident_parser();

    let file_name = ident;

    let items = just("with")
        .ignore_then(ident.repeated().collect())
        .or_not()
        .map(|opt_items| opt_items.unwrap_or_default());

    text::keyword("import")
        .ignore_then(file_name.padded())
        .then(items)
        .map(|(file, items)| Import { file, items })
        .then_ignore(just(';'))
}

// -------------------- Objects --------------------

/// An Object is either a struct or an enum because they have the same AST
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Object {
    name: String,
    fields: Vec<Field>,
    props: Vec<ObjectProperties>,
    derives: Vec<ObjectMethods>,
}

/// Note that `type_` may be the string `<Empty>`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Field {
    name: String,
    type_: String,
}

/// Properties for data types (Structs and Enums)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectProperties {
    Public,
    Export,
}

/// `Derivable` methods for data types (Structs and Enums)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectMethods {
    Eq,
    Log,
    Custom(String),
}

// -------------------- Struct Parsing --------------------

pub fn struct_parser() -> impl Parser<char, Object, Error = Simple<char>> {
    let ident = get_ident_parser();

    let camel_case = get_camel_case_parser();

    let field = ident.then(ident).map(|(name, type_)| Field { name, type_ });

    let fields = field.separated_by(just("::")).at_least(1);

    let property = choice((
        text::keyword("Public")
            .to(ObjectProperties::Public)
            .labelled("Public"),
        text::keyword("Export")
            .to(ObjectProperties::Export)
            .labelled("Export"),
    ));

    let struct_derives = choice((
        text::keyword("Eq").to(ObjectMethods::Eq),
        text::keyword("Log").to(ObjectMethods::Log),
    ))
    .or(ident.map(ObjectMethods::Custom));

    let properties = just("is")
        .ignore_then(property.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    let derives = just("derives")
        .ignore_then(struct_derives.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    text::keyword("struct")
        .ignore_then(camel_case)
        .then_ignore(just("="))
        .then(fields)
        .then(properties)
        .then(derives)
        .then_ignore(just(";"))
        .map(|(((name, fields), properties), derives)| Object {
            name,
            fields,
            props: properties,
            derives,
        })
}

// -------------------- Enum Parsing --------------------

pub fn enum_parser() -> impl Parser<char, Object, Error = Simple<char>> {
    let ident = get_ident_parser();

    let camel_case = get_camel_case_parser();

    // Types are optional in an enum field, so they get mapped to `<Empty>`
    let field = ident
        .then(
            ident
                .or_not()
                .map(|opt_type| opt_type.unwrap_or_else(|| "<Empty>".to_string())),
        )
        .map(|(name, type_)| Field { name, type_ });

    let fields = field.separated_by(just("|")).at_least(1);

    let property = choice((
        text::keyword("Public")
            .to(ObjectProperties::Public)
            .labelled("Public"),
        text::keyword("Export")
            .to(ObjectProperties::Export)
            .labelled("Export"),
    ));

    let struct_derives = choice((
        text::keyword("Eq").to(ObjectMethods::Eq),
        text::keyword("Log").to(ObjectMethods::Log),
    ))
    .or(ident.map(ObjectMethods::Custom));

    let properties = just("is")
        .ignore_then(property.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    let derives = just("derives")
        .ignore_then(struct_derives.padded().repeated())
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    text::keyword("enum")
        .ignore_then(camel_case)
        .then_ignore(just("="))
        .then(fields)
        .then(properties)
        .then(derives)
        .then_ignore(just(";"))
        .map(|(((name, fields), properties), derives)| Object {
            name,
            fields,
            props: properties,
            derives,
        })
}
