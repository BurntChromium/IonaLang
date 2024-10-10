//! Parse

use chumsky::prelude::*;

// -------------------- Shared Tools --------------------

pub const RESERVED_KEYWORDS: [&str; 6] = ["import", "is", "derives", "struct", "fn", "enum"];

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    Integer,
    Float,
    String,
    List(Box<Type>),
    Tuple(Vec<Type>),
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

fn type_parser() -> impl Parser<char, Type, Error = Simple<char>> {
    recursive(|type_parser| {
        let ident = text::ident();

        let basic_type = choice((
            text::keyword("int").to(Type::Integer),
            text::keyword("float").to(Type::Float),
            text::keyword("str").to(Type::String),
            ident.map(Type::Custom),
        ));

        let list_type = just("List")
            .ignore_then(type_parser.clone().delimited_by(just('['), just(']')))
            .map(|inner| Type::List(Box::new(inner)));

        let tuple_type = just("Tuple")
            .ignore_then(
                type_parser
                    .clone()
                    .separated_by(just(','))
                    .at_least(1)
                    .delimited_by(just('['), just(']')),
            )
            .map(|inner| Type::Tuple(inner));

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
    alias_of: Type,
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

// -------------------- Statements --------------------

struct VariableDeclaration {
    name: String,
    v_type: Type,
    attributes: Vec<VariableAttribute>,
    value: Vec<Expression>,
}

enum VariableAttribute {
    Mutable,
    ThreadSafe,
}

struct Conditional {
    condition: Expression,
    effect: Statement,
}

enum Statement {
    VariableDecl(VariableDeclaration),
    BareFuncCall(Expression),
}

// -------------------- Expressions --------------------

#[derive(Debug, Clone, PartialEq)]
struct ObjectField {
    name: String,
    field: String,
}

#[derive(Debug, Clone, PartialEq)]
struct FunctionCall {
    name: String,
    expr: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
enum Expression {
    IntLiteral(i64),
    FloatLiteral(f64),
    StrLiteral(String),
    TupleLiteral(Vec<Expression>),
    ListLiteral(Vec<Expression>),
    FieldAccess(ObjectField),
    FunctionCall(FunctionCall),
}

fn expression_parser() -> impl Parser<char, Expression, Error = Simple<char>> {
    recursive(|expr| {
        let int_literal = text::int(10)
            .map(|s: String| Expression::IntLiteral(s.parse().unwrap()))
            .padded();

        // Floats that look like 42.1 or 42.
        let float_with_decimal = text::int(10)
            .then(just('.').then(text::digits(10)).or_not())
            .map(|(whole, frac)| {
                let s = match frac {
                    Some((_, frac)) => format!("{}.{}", whole, frac),
                    None => whole,
                };
                Expression::FloatLiteral(s.parse().unwrap())
            })
            .padded();

        // Floats that look like 42f
        let float_with_f = text::int(10)
            .then(just('f'))
            .map(|(base, _)| Expression::FloatLiteral(base.parse().unwrap()))
            .padded();

        let float_literal = float_with_decimal.or(float_with_f);

        let str_literal = just('"')
            .ignore_then(none_of('"').repeated())
            .then_ignore(just('"'))
            .collect::<String>()
            .map(Expression::StrLiteral)
            .padded();

        let tuple_literal = expr
            .clone()
            .separated_by(just(','))
            .delimited_by(just('('), just(')'))
            .map(Expression::TupleLiteral)
            .padded();

        let list_literal = expr
            .clone()
            .separated_by(just(','))
            .delimited_by(just('['), just(']'))
            .map(Expression::ListLiteral)
            .padded();

        let field_access = ident_parser()
            .then(just('.').ignore_then(ident_parser()))
            .map(|(name, field)| Expression::FieldAccess(ObjectField { name, field }))
            .padded();

        let function_call = ident_parser()
            .then(
                expr.clone()
                    .separated_by(just(' '))
                    .at_least(1)
                    .delimited_by(just('('), just(')'))
                    .or(expr.clone().separated_by(just(' ')).at_least(1))
                    .or_not(),
            )
            .map(|(name, args)| {
                Expression::FunctionCall(FunctionCall {
                    name,
                    expr: args.unwrap_or_default(),
                })
            })
            .padded();

        choice((
            list_literal,
            tuple_literal,
            float_literal,
            field_access,
            int_literal,
            str_literal,
            function_call,
            expr.delimited_by(just('('), just(')')),
        ))
    })
}

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_expr_int() {
        let input = "42";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(Expression::IntLiteral(42), value);
    }

    #[test]
    fn test_parse_expr_float_1() {
        let input = "42.27";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(Expression::FloatLiteral(42.27), value);
    }

    #[test]
    fn test_parse_expr_float_2() {
        let input = "42.";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(Expression::FloatLiteral(42f64), value);
    }

    #[test]
    fn test_parse_expr_float_3() {
        let input = "42f";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(Expression::FloatLiteral(42f64), value);
    }

    #[test]
    fn test_parse_expr_str() {
        let input = "\"forty two\"";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(Expression::StrLiteral("forty two".to_string()), value);
    }

    #[test]
    fn test_parse_expr_field() {
        let input = "obj.field";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(
            Expression::FieldAccess(ObjectField {
                name: "obj".to_string(),
                field: "field".to_string()
            }),
            value
        );
    }

    #[test]
    fn test_parse_expr_tuple() {
        let input = "(1f, 2f, 3f)";
        let result = expression_parser().parse(input);
        match result {
            Ok(value) => {
                assert_eq!(
                    Expression::TupleLiteral(vec![
                        Expression::FloatLiteral(1f64),
                        Expression::FloatLiteral(2f64),
                        Expression::FloatLiteral(3f64)
                    ]),
                    value
                );
            }
            Err(e) => {
                println!("{:?}", e);
                assert_eq!(true, false);
            }
        }
    }
}
