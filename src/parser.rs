//! Parse

use chumsky::prelude::*;

// -------------------- Shared Tools --------------------

pub const RESERVED_KEYWORDS: [&str; 6] = ["import", "is", "derives", "struct", "fn", "enum"];

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Types {
    Integer,
    Float,
    String,
    List(Box<Types>),
    Tuple(Vec<Types>),
    Custom(String),
}

/// Parsing identifiers (stringy names that aren't reserved)
fn ident_parser() -> impl Parser<char, String, Error = Simple<char>> {
    text::ident()
        .try_map(|s: String, span| {
            if !RESERVED_KEYWORDS.contains(&&s.as_str()) {
                Ok(s)
            } else {
                Err(Simple::custom(span, "Unexpected keyword"))
            }
        })
        .padded()
}

fn camel_case_parser() -> impl Parser<char, String, Error = Simple<char>> {
    filter(|c: &char| c.is_ascii_uppercase())
        .chain(filter(|c: &char| c.is_ascii_alphanumeric()).repeated())
        .collect::<String>()
        .padded()
}

fn type_parser() -> impl Parser<char, Types, Error = Simple<char>> {
    recursive(|type_parser| {
        let ident = text::ident();

        let basic_type = choice((
            text::keyword("int").to(Types::Integer),
            text::keyword("float").to(Types::Float),
            text::keyword("str").to(Types::String),
            ident.map(Types::Custom),
        ));

        let list_type = just("List")
            .ignore_then(type_parser.clone().delimited_by(just('['), just(']')))
            .map(|inner| Types::List(Box::new(inner)));

        let tuple_type = just("Tuple")
            .ignore_then(
                type_parser
                    .clone()
                    .separated_by(just(','))
                    .at_least(1)
                    .delimited_by(just('['), just(']')),
            )
            .map(|inner| Types::Tuple(inner));

        choice((list_type, tuple_type, basic_type))
    })
}

// -------------------- AST --------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ASTNode {
    ImportStatement(Import),
    TypeAliasDeclaration(TypeAlias),
    StructDeclaration(Object),
    EnumDeclaration(Object),
}

fn ast_parser() -> impl Parser<char, Vec<ASTNode>, Error = Simple<char>> {
    let import_node = import_parser().map(ASTNode::ImportStatement);
    let alias_node = alias_parser().map(ASTNode::TypeAliasDeclaration);
    let struct_node = struct_parser().map(ASTNode::StructDeclaration);
    let enum_node = enum_parser().map(ASTNode::EnumDeclaration);

    let node = import_node.or(alias_node).or(struct_node).or(enum_node);

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
    let ident = ident_parser();

    let file_name = ident_parser();

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

// -------------------- Type Aliases --------------------

/// Type aliases let you do stuff like `alias Salary = int`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeAlias {
    name: String,
    alias_of: Types,
}

fn alias_parser() -> impl Parser<char, TypeAlias, Error = Simple<char>> {
    let camel_case = camel_case_parser();
    let name = camel_case;
    let alias_of = type_parser();

    text::keyword("alias")
        .ignore_then(name.padded())
        .then(just('='))
        .then(alias_of.padded())
        .map(|(name, alias_of)| TypeAlias {
            name: name.0,
            alias_of,
        })
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
    let camel_case = camel_case_parser();

    let field = ident_parser()
        .then(ident_parser())
        .map(|(name, type_)| Field { name, type_ });

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
    .or(ident_parser().map(ObjectMethods::Custom));

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
    let camel_case = camel_case_parser();

    // Types are optional in an enum field, so they get mapped to `<Empty>`
    let field = ident_parser()
        .then(
            ident_parser()
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
    .or(ident_parser().map(ObjectMethods::Custom));

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
