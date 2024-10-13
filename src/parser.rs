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
    Void,
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

fn snake_case_parser() -> impl Parser<char, String, Error = Simple<char>> {
    filter(|c: &char| c.is_ascii_lowercase() || *c == '_')
        .chain(
            filter(|c: &char| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_').repeated(),
        )
        .collect::<String>()
        .padded()
        .labelled("snake_case identifier")
}

fn type_parser() -> impl Parser<char, Type, Error = Simple<char>> {
    recursive(|type_parser| {
        let basic_type = choice((
            text::keyword("int").to(Type::Integer),
            text::keyword("float").to(Type::Float),
            text::keyword("str").to(Type::String),
        ));

        let list_type = text::keyword("List")
            .ignore_then(type_parser.clone().delimited_by(just('['), just(']')))
            .map(|inner| Type::List(Box::new(inner)));

        let tuple_type = text::keyword("Tuple")
            .ignore_then(
                type_parser
                    .clone()
                    .separated_by(just(',').padded())
                    .at_least(1)
                    .delimited_by(just('['), just(']')),
            )
            .map(|inner| Type::Tuple(inner));

        let custom_type = camel_case_parser().map(Type::Custom);

        choice((basic_type, list_type, tuple_type, custom_type))
    })
}

// -------------------- AST --------------------

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    ImportStatement(Import),
    TypeAliasDeclaration(TypeAlias),
    StructDeclaration(Object),
    EnumDeclaration(Object),
    FunctionDeclaration(FunctionDeclaration),
}

fn ast_parser() -> impl Parser<char, Vec<ASTNode>, Error = Simple<char>> {
    let import_node = import_parser().map(ASTNode::ImportStatement);
    let alias_node = alias_parser().map(ASTNode::TypeAliasDeclaration);
    let struct_node = struct_parser().map(ASTNode::StructDeclaration);
    let enum_node = enum_parser().map(ASTNode::EnumDeclaration);
    let function_node = function_parser().map(ASTNode::FunctionDeclaration);

    let node = import_node
        .or(alias_node)
        .or(struct_node)
        .or(enum_node)
        .or(function_node);

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
    type_: Type,
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
        .then(type_parser())
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
            type_parser()
                .or_not()
                .map(|opt_type| opt_type.unwrap_or_else(|| Type::Void)),
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

// -------------------- Functions --------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionProperties {
    Public,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FunctionPermissions {
    ReadIO,
    WriteIO,
    ReadFS,  // todo: allow scoping to specific paths?
    WriteFS, // todo: allow scoping to specific paths?
    HTTPAny,
    HTTPGet,
    HTTPPost,
    Any,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    name: String,
    args: Vec<Field>,
    return_type: Type,
    is: Vec<FunctionProperties>,
    derives: Vec<FunctionPermissions>,
    contract_in: Option<Expression>,
    contract_out: Option<Expression>,
    body: Vec<Statement>,
    returns: Expression,
}

pub fn function_parser() -> impl Parser<char, FunctionDeclaration, Error = Simple<char>> {
    let field = ident_parser()
        .then(type_parser())
        .map(|(name, type_)| Field { name, type_ });

    let fields = field.separated_by(just("::")).at_least(1);

    let function_header = text::keyword("fn")
        .ignore_then(snake_case_parser())
        .then_ignore(just('=').padded())
        .then(fields)
        .then_ignore(just("->").padded())
        .then(type_parser());

    let prop_parser = choice((
        text::keyword("Public").to(FunctionProperties::Public),
        text::keyword("Export").to(FunctionProperties::Export),
    ));

    let requirements_parser = choice((
        text::keyword("ReadIO").to(FunctionPermissions::ReadIO),
        text::keyword("WriteIO").to(FunctionPermissions::WriteIO),
        text::keyword("ReadFS").to(FunctionPermissions::ReadFS),
        text::keyword("WriteFS").to(FunctionPermissions::WriteFS),
        text::keyword("HTTPAny").to(FunctionPermissions::HTTPAny),
        text::keyword("HTTPGet").to(FunctionPermissions::HTTPGet),
        text::keyword("HTTPPost").to(FunctionPermissions::HTTPPost),
        text::keyword("Any").to(FunctionPermissions::Any),
    ))
    .or(camel_case_parser().map(FunctionPermissions::Custom));

    let function_props = text::keyword("Is:")
        .padded()
        .ignore_then(prop_parser.padded().repeated())
        .or_not()
        .map(|props| props.unwrap_or_default());

    let function_requirements = text::keyword("Uses:")
        .padded()
        .ignore_then(requirements_parser.padded().repeated())
        .or_not()
        .map(|reqs| reqs.unwrap_or_default());

    let contract_in = text::keyword("In:")
        .padded()
        .ignore_then(expression_parser())
        .then_ignore(just(';'))
        .padded();

    let contract_out = text::keyword("Out:")
        .padded()
        .ignore_then(expression_parser())
        .then_ignore(just(';'))
        .padded();

    let function_body = just('{')
        .padded()
        .ignore_then(
            function_props
                .then(function_requirements)
                .then(contract_in.or_not())
                .then(contract_out.or_not())
                .then(statement_parser().repeated())
                .then(
                    just("return")
                        .padded()
                        .ignore_then(expression_parser())
                        .then_ignore(just(';')),
                ),
        )
        .then_ignore(just('}').padded());

    function_header.then(function_body).map(
        |(
            ((name, args), return_type),
            (((((is, derives), contract_in), contract_out), body), returns),
        )| {
            FunctionDeclaration {
                name,
                args,
                return_type,
                is,
                derives,
                contract_in,
                contract_out,
                body,
                returns,
            }
        },
    )
}

// -------------------- Statements --------------------

#[derive(Debug, Clone, PartialEq)]
struct VariableDeclaration {
    name: String,
    v_type: Type,
    attributes: Vec<VariableAttribute>,
    value: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VariableAttribute {
    Mutable,
    ThreadSafe,
}

#[derive(Debug, Clone, PartialEq)]
struct Conditional {
    condition: Expression,
    effect: Statement,
}

#[derive(Debug, Clone, PartialEq)]
enum Statement {
    VariableDecl(VariableDeclaration),
    BareFuncCall(Expression),
}

fn variable_declaration_parser() -> impl Parser<char, VariableDeclaration, Error = Simple<char>> {
    let attribute = choice((
        just("Mutable").to(VariableAttribute::Mutable),
        just("ThreadSafe").to(VariableAttribute::ThreadSafe),
    ));

    let attributes = just("::")
        .ignore_then(attribute.repeated())
        .collect::<Vec<_>>()
        .or_not()
        .map(Option::unwrap_or_default);

    let declaration = just("let")
        .ignore_then(snake_case_parser())
        .then_ignore(just("::"))
        .then(type_parser().padded())
        .then(attributes)
        .then_ignore(just("="))
        .then(expression_parser())
        .map(
            |(((name, v_type), attributes), value)| VariableDeclaration {
                name,
                v_type,
                attributes,
                value,
            },
        );

    declaration
}

fn statement_parser() -> impl Parser<char, Statement, Error = Simple<char>> {
    let var_decl = variable_declaration_parser().map(Statement::VariableDecl);
    let bare_fun = expression_parser().map(Statement::BareFuncCall);
    var_decl.or(bare_fun)
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
    Identifier(String),
    TupleLiteral(Vec<Expression>),
    ListLiteral(Vec<Expression>),
    FieldAccess(Box<Expression>, String),
    Group(Vec<Expression>),
}

fn expression_parser() -> impl Parser<char, Vec<Expression>, Error = Simple<char>> {
    recursive(|expr| {
        let int_literal = text::int(10)
            .map(|s: String| Expression::IntLiteral(s.parse().unwrap()))
            .padded();

        // Floats that look like 42.1 or 42.
        let float_with_decimal = text::int(10)
            .then(just('.').then(text::digits(10)))
            .map(|(whole, (_, frac))| {
                Expression::FloatLiteral(format!("{}.{}", whole, frac).parse().unwrap())
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

        let parenthesized_expr = expr.clone().delimited_by(just('('), just(')'));

        let function_call = ident_parser() // Parse the function name
            .then(
                // Parse zero or more space-separated expressions as arguments
                parenthesized_expr
                    .or(expr.clone()) // An argument is either an expression or a parenthesized expression
                    .repeated(), // Collect multiple arguments
            )
            .map(|(name, args)| {
                // Convert parsed name and arguments into a FunctionCall expression
                Expression::FunctionCall(FunctionCall { name, expr: args })
            });

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
    fn test_type_parser_int() {
        assert_eq!(Type::Integer, type_parser().parse("int").unwrap())
    }

    #[test]
    fn test_type_parser_str() {
        assert_eq!(Type::String, type_parser().parse("str").unwrap())
    }

    #[test]
    fn test_type_parser_float() {
        assert_eq!(Type::Float, type_parser().parse("float").unwrap())
    }

    #[test]
    fn test_type_parser_list1() {
        assert_eq!(
            Type::List(Box::new(Type::Integer)),
            type_parser().parse("List[int]").unwrap()
        )
    }

    #[test]
    fn test_type_parser_tuple1() {
        assert_eq!(
            Type::Tuple(vec![Type::Integer, Type::String]),
            type_parser().parse("Tuple[int, str]").unwrap()
        )
    }

    #[test]
    fn test_type_parser_custom1() {
        assert_eq!(
            Type::Custom("Employee".to_string()),
            type_parser().parse("Employee").unwrap()
        )
    }

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

    // map add 2 (concat [1.0, 2f] [3f, 4.0])
    #[test]
    fn test_parse_expr_fn_call_add_simple() {
        let input = "add 1 2";
        let result = expression_parser().parse(input);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(
            Expression::FunctionCall(FunctionCall {
                name: "add".to_string(),
                expr: vec![Expression::IntLiteral(1), Expression::IntLiteral(2)]
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

    #[test]
    fn test_parse_expr_list_floats() {
        let input = "[1f, 2f, 3f]";
        let result = expression_parser().parse(input);
        match result {
            Ok(value) => {
                assert_eq!(
                    Expression::ListLiteral(vec![
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

    #[test]
    fn test_parse_expr_list_str() {
        let input = "[\"a\", \"b\", \"c\"]";
        let result = expression_parser().parse(input);
        match result {
            Ok(value) => {
                assert_eq!(
                    Expression::ListLiteral(vec![
                        Expression::StrLiteral("a".to_string()),
                        Expression::StrLiteral("b".to_string()),
                        Expression::StrLiteral("c".to_string())
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

    #[test]
    fn test_parse_var_decl_1() {
        let input = "let x :: int = 5;";
        let result = variable_declaration_parser().parse(input);
        match result {
            Ok(value) => assert_eq!(
                VariableDeclaration {
                    name: "x".to_string(),
                    v_type: Type::Integer,
                    attributes: vec![],
                    value: Expression::IntLiteral(5)
                },
                value
            ),
            Err(e) => {
                println!("{:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    fn test_parse_fn_1() {
        let input = r#"fn foo = a int :: b int -> int {
            Is: Public;

            return div a b;
        }"#;
        let result = function_parser().parse(input);
        match result {
            Ok(value) => assert_eq!(
                FunctionDeclaration {
                    name: "foo".to_string(),
                    args: vec![Field{ name: "a".to_string(), type_: Type::Integer}, Field{ name: "b".to_string(), type_: Type::Integer}],
                    return_type: Type::Integer,
                    is: vec![FunctionProperties::Public],
                    derives: vec![],
                    contract_in: None,
                    contract_out: None,
                    body: vec![],
                    returns: Expression::FunctionCall(
                        FunctionCall {
                            name: "div".to_string(),
                            expr: vec![
                                Expression::
                            ]
                        }
                    )
                },
                value
            ),
            Err(e) => {
                println!("{:?}", e);
                assert_eq!(true, false);
            }
        }
    }
}
