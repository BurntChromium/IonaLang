//! Recursive Descent Parser
use crate::diagnostics::Diagnostic;
use crate::expression_parser::Expr;
use crate::lexer::{Symbol, Token};

// -------------------- Parser Object --------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    tokens: Vec<Token>,
    offset: usize,
    pub recursion_counter: usize,
}

/// Golang-esque error handling to allow multiple returns
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserOutput<T> {
    pub output: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> ParserOutput<T> {
    pub fn okay(output: T) -> Self {
        ParserOutput {
            output: Some(output),
            diagnostics: vec![],
        }
    }

    pub fn err(diagnostics: Vec<Diagnostic>) -> Self {
        ParserOutput {
            output: None,
            diagnostics,
        }
    }

    /// Errors can be safely cast from one type to another because only the output varies
    ///
    /// This lets return a parser error from an "inner" parser error that returns type T from an outer parser that returns type O
    pub fn transmute_error<O>(self) -> ParserOutput<O> {
        ParserOutput {
            output: None,
            diagnostics: self.diagnostics,
        }
    }
}

pub trait ParserOutputExt<T> {
    fn and_then<U, F>(self, f: F) -> ParserOutput<U>
    where
        F: FnOnce(T) -> ParserOutput<U>;

    fn map<U, F>(self, f: F) -> ParserOutput<U>
    where
        F: FnOnce(T) -> U;

    fn ignore(self) -> ParserOutput<()>;
}

impl<T> ParserOutputExt<T> for ParserOutput<T> {
    fn and_then<U, F>(self, f: F) -> ParserOutput<U>
    where
        F: FnOnce(T) -> ParserOutput<U>,
    {
        match self.output {
            Some(value) => {
                let next = f(value);
                ParserOutput {
                    output: next.output,
                    diagnostics: self
                        .diagnostics
                        .into_iter()
                        .chain(next.diagnostics)
                        .collect(),
                }
            }
            None => ParserOutput {
                output: None,
                diagnostics: self.diagnostics,
            },
        }
    }

    fn map<U, F>(self, f: F) -> ParserOutput<U>
    where
        F: FnOnce(T) -> U,
    {
        ParserOutput {
            output: self.output.map(f),
            diagnostics: self.diagnostics,
        }
    }

    fn ignore(self) -> ParserOutput<()> {
        self.map(|_| ())
    }
}

// -------------------- AST --------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Void,
    Integer,
    String,
    Boolean,
    Array(Box<Type>),
    Map(Box<Type>),
    Shared(Box<Type>),
    Generic(String),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataProperties {
    Public,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTraits {
    Eq,
    Show,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
    pub properties: Vec<DataProperties>,
    pub traits: Vec<DataTraits>,
}

/// An enum has the same shape as a struct but different rules
///
/// For clarity I separate the types, even though they're functionally identical
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub name: String,
    pub fields: Vec<Field>,
    pub properties: Vec<DataProperties>,
    pub traits: Vec<DataTraits>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub file: String,
    pub items: Vec<String>,
}

/// Functions can have different properties than Data Types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionProperties {
    Public,
    Export,
}

/// Functions have a permissions/effects system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionPermissions {
    ReadFile,
    WriteFile,
    ReadIO,
    WriteIO,
    HTTPAny,
    HTTPGet,
    HTTPPost,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub args: Vec<Field>,
    pub returns: Type,
    pub properties: Vec<FunctionProperties>,
    pub permissions: Vec<FunctionPermissions>,
    pub contracts: Vec<FunctionContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTNode {
    StructDeclaration(Struct),
    EnumDeclaration(Enum),
    ImportStatement(Import),
}

// -------------------- Parsers --------------------

// -------------------| Parse Top Level Nodes |-------------------

impl Parser {
    pub fn parse_all(&mut self) -> ParserOutput<Vec<ASTNode>> {
        self.parse_list_newline_separated(|p| p.parse_top_level_declaration())
    }

    fn parse_top_level_declaration(&mut self) -> ParserOutput<ASTNode> {
        self.skip_whitespace();
        match self.peek().symbol {
            Symbol::Struct => self.parse_struct().map(ASTNode::StructDeclaration),
            Symbol::Enum => self.parse_enum().map(ASTNode::EnumDeclaration),
            Symbol::Import => self.parse_import().map(ASTNode::ImportStatement),
            _ => {
                let message = format!(
                    "expected a keyword such as 'fn', 'struct', or 'import', but found {:?}",
                    self.peek().symbol
                );
                self.single_error(&message)
            }
        }
    }
}

// -------------------| Parse Types |--------------------

impl Parser {
    fn parse_type(&mut self) -> ParserOutput<Type> {
        // Handle generics
        if self.peek().symbol == Symbol::Generic {
            self.then_ignore(Symbol::Generic);
            self.then_ignore(Symbol::LeftAngle);
            let generic = self
                .then_identifier()
                .and_then(|name| ParserOutput::okay(Type::Generic(name)));
            self.then_ignore(Symbol::RightAngle);
            return generic;
        }
        // Handle everything else
        self.then_identifier().and_then(|name| match name.as_str() {
            "Int" => ParserOutput::okay(Type::Integer),
            "Str" => ParserOutput::okay(Type::String),
            "Bool" => ParserOutput::okay(Type::Boolean),
            _ => ParserOutput::okay(Type::Custom(name)),
        })
    }
}

// -------------------| Parser Imports |--------------------

impl Parser {
    fn parse_import(&mut self) -> ParserOutput<Import> {
        self.then_ignore(Symbol::Import)
            .and_then(|_| self.with_whitespace(|p| p.then_identifier()))
            .and_then(|file| {
                self.with_whitespace(|p| p.then_ignore(Symbol::With))
                    .and_then(|_| {
                        self.parse_list_comma_separated(|p| {
                            p.with_whitespace(|p| p.then_identifier())
                        })
                    })
                    .and_then(|items| {
                        self.then_ignore(Symbol::Semicolon)
                            .map(|_| Import { file, items })
                    })
            })
    }
}

// -------------------| Shared Parsers: Structs and Enums |--------------------

impl Parser {
    fn parse_data_properties(&mut self) -> ParserOutput<DataProperties> {
        self.then_identifier().and_then(|name| match name.as_str() {
            "Public" => ParserOutput::okay(DataProperties::Public),
            "Export" => ParserOutput::okay(DataProperties::Export),
            other => self.single_error::<DataProperties>(&format!(
                "expected 'Public' or 'Export', but received {}",
                other
            )),
        })
    }

    fn parse_data_traits(&mut self) -> ParserOutput<DataTraits> {
        self.then_identifier().and_then(|name| match name.as_str() {
            "Eq" => ParserOutput::okay(DataTraits::Eq),
            "Show" => ParserOutput::okay(DataTraits::Show),
            other => self.single_error::<DataTraits>(&format!(
                "expected 'Eq' or 'Show', but received {}",
                other
            )),
        })
    }

    fn parse_metadata_list<T, F>(
        &mut self,
        expected_symbol: Symbol,
        parse_item: F,
    ) -> ParserOutput<Vec<T>>
    where
        F: Fn(&mut Self) -> ParserOutput<T>,
    {
        self.then_ignore(expected_symbol)
            .and_then(|_| self.with_whitespace(|p| p.then_ignore(Symbol::Colon)))
            .and_then(|_| self.parse_list_comma_separated(|p| parse_item(p)))
            .and_then(|values| self.then_ignore(Symbol::Semicolon).map(|_| values))
    }

    fn parse_metadata_data_types(
        &mut self,
    ) -> ParserOutput<(Vec<DataProperties>, Vec<DataTraits>)> {
        self.then_ignore(Symbol::Tag)
            .and_then(|_| self.then_ignore(Symbol::Metadata))
            .and_then(|_| self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen)))
            .and_then(|_| {
                let mut properties = Vec::new();
                let mut traits = Vec::new();
                let mut diagnostics = Vec::new();

                loop {
                    self.skip_whitespace();
                    match self.peek().symbol {
                        Symbol::Properties => {
                            let result = self.parse_metadata_list(Symbol::Properties, |p| {
                                p.parse_data_properties()
                            });
                            properties.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::Traits => {
                            let result =
                                self.parse_metadata_list(Symbol::Traits, |p| p.parse_data_traits());
                            traits.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::BraceClose => break,
                        _ => {
                            diagnostics.push(Diagnostic::new_error_simple(
                                "Unexpected token in metadata",
                                &self.peek().pos,
                            ));
                            self.consume(); // Skip the unexpected token
                        }
                    }
                }

                ParserOutput {
                    output: Some((properties, traits)),
                    diagnostics,
                }
            })
            .and_then(|metadata| self.then_ignore(Symbol::BraceClose).map(|_| metadata))
    }
}

// -------------------| Struct Parsers |--------------------

impl Parser {
    fn parse_struct_declaration(&mut self) -> ParserOutput<String> {
        self.then_ignore(Symbol::Struct)
            .and_then(|_| self.with_whitespace(|p| p.then_identifier()))
            .and_then(|name| {
                self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen).map(|_| name))
            })
    }

    fn parse_field_mandatory_type(&mut self) -> ParserOutput<Field> {
        self.then_identifier().and_then(|name| {
            self.with_whitespace(|p| p.then_ignore(Symbol::Colon))
                .and_then(|_| self.with_whitespace(|p| p.parse_type()))
                .map(|type_| Field {
                    name,
                    field_type: type_,
                })
        })
    }

    pub fn parse_struct(&mut self) -> ParserOutput<Struct> {
        let name = self.parse_struct_declaration();
        if name.output.is_none() {
            return name.transmute_error::<Struct>();
        }
        let struct_name = name.output.clone().unwrap();
        name.and_then(|_| {
            self.parse_list_comma_separated(|p| {
                p.with_whitespace(|p| p.parse_field_mandatory_type())
            })
        })
        .and_then(|fields| {
            let metadata = self.parse_metadata_data_types();
            metadata.map(|(properties, traits)| Struct {
                name: struct_name,
                fields,
                properties,
                traits,
            })
        })
        .and_then(|struct_| {
            self.with_whitespace(|p| p.then_ignore(Symbol::BraceClose))
                .map(|_| struct_)
        })
    }
}

// -------------------| Enum Parsers |--------------------

impl Parser {
    fn parse_enum_declaration(&mut self) -> ParserOutput<String> {
        self.then_ignore(Symbol::Enum)
            .and_then(|_| self.with_whitespace(|p| p.then_identifier()))
            .and_then(|name| {
                self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen).map(|_| name))
            })
    }

    fn parse_field_optional_type(&mut self) -> ParserOutput<Field> {
        self.then_identifier().and_then(|name| {
            self.with_whitespace(|p| {
                match p.peek().symbol {
                    Symbol::Colon => {
                        // This is a typed field
                        p.then_ignore(Symbol::Colon)
                            .and_then(|_| p.with_whitespace(|p| p.parse_type()))
                            .map(|field_type| Field { name, field_type })
                    }
                    Symbol::Comma => {
                        // This is a typeless field
                        ParserOutput::okay(Field {
                            name,
                            field_type: Type::Void,
                        })
                    }
                    _ => {
                        let message = format!(
                            "expected ':' or ',' after enum field name, but found {:?}",
                            p.peek().symbol
                        );
                        p.single_error(&message)
                    }
                }
            })
        })
    }

    pub fn parse_enum(&mut self) -> ParserOutput<Enum> {
        let name = self.parse_enum_declaration();
        if name.output.is_none() {
            return name.transmute_error::<Enum>();
        }
        let enum_name = name.output.clone().unwrap();
        name.and_then(|_| {
            self.parse_list_comma_separated(|p| {
                p.with_whitespace(|p| p.parse_field_optional_type())
            })
        })
        .and_then(|fields| {
            let metadata = self.parse_metadata_data_types();
            metadata.map(|(properties, traits)| Enum {
                name: enum_name,
                fields,
                properties,
                traits,
            })
        })
        .and_then(|enum_| {
            self.with_whitespace(|p| p.then_ignore(Symbol::BraceClose))
                .map(|_| enum_)
        })
    }
}

// -------------------| Parse Functions |--------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<Field>,
    pub return_type: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractType {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionContract {
    type_: ContractType,
    condition: Expr,
    message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    condition: Option<Expr>, // None is the catch all case (`_` in a match or `else` in a ternary)
    computation: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionCall(Expr),
    VariableDeclaration {
        name: String,
        type_: Type,
        value: Expr,
    },
    VariableMutation {
        name: String,
        value: Expr,
    },
    Conditional(Vec<Branch>),
    Return(Expr),
}

impl Parser {
    /// Returns (Name, Args, ReturnType)
    fn parse_function_declaration(&mut self) -> ParserOutput<FunctionDeclaration> {
        // Parse "fn" keyword and function name
        let fn_and_name = self
            .then_ignore(Symbol::Function)
            .and_then(|_| self.with_whitespace(|p| p.then_identifier()));

        // Parse parameters and return type
        let declaration = fn_and_name.and_then(|name| {
            self.then_ignore(Symbol::ParenOpen)
                .and_then(|_| self.parse_list_comma_separated(|p| p.parse_field_mandatory_type()))
                .and_then(|parameters| {
                    self.then_ignore(Symbol::ParenClose).and_then(|_| {
                        // Parse return type arrow and type
                        self.with_whitespace(|p| p.then_ignore(Symbol::Dash))
                            .and_then(|_| self.then_ignore(Symbol::RightAngle))
                            .and_then(|_| self.with_whitespace(|p| p.parse_type()))
                            .map(|return_type| (name, parameters, return_type))
                    })
                })
        });

        // Parse opening brace and construct final result
        declaration.and_then(|(name, parameters, return_type)| {
            self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen))
                .map(|_| FunctionDeclaration {
                    name,
                    parameters,
                    return_type,
                })
        })
    }

    fn parse_fn_properties(&mut self) -> ParserOutput<FunctionProperties> {
        self.then_identifier().and_then(|name| match name.as_str() {
            "Public" => ParserOutput::okay(FunctionProperties::Public),
            "Export" => ParserOutput::okay(FunctionProperties::Export),
            other => self.single_error::<FunctionProperties>(&format!(
                "expected 'Public' or 'Export', but received {}",
                other
            )),
        })
    }

    fn parse_fn_permissions(&mut self) -> ParserOutput<FunctionPermissions> {
        self.then_identifier().and_then(|name| match name.as_str() {
            "ReadFile" => ParserOutput::okay(FunctionPermissions::ReadFile),
            "WriteFile" => ParserOutput::okay(FunctionPermissions::WriteFile),
            "ReadIO" => ParserOutput::okay(FunctionPermissions::ReadIO),
            "WriteIO" => ParserOutput::okay(FunctionPermissions::WriteIO),
            "HTTPAny" => ParserOutput::okay(FunctionPermissions::HTTPAny),
            "HTTPGet" => ParserOutput::okay(FunctionPermissions::HTTPGet),
            "HTTPPost" => ParserOutput::okay(FunctionPermissions::HTTPPost),
            other => ParserOutput::okay(FunctionPermissions::Custom(other.to_string())),
        })
    }

    fn parse_function_metadata(
        &mut self,
    ) -> ParserOutput<(Vec<FunctionProperties>, Vec<FunctionPermissions>)> {
        self.then_ignore(Symbol::Tag)
            .and_then(|_| self.then_ignore(Symbol::Metadata))
            .and_then(|_| self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen)))
            .and_then(|_| {
                let mut properties = Vec::new();
                let mut traits = Vec::new();
                let mut diagnostics = Vec::new();

                loop {
                    self.skip_whitespace();
                    match self.peek().symbol.clone() {
                        Symbol::Properties => {
                            let result = self.parse_metadata_list(Symbol::Properties, |p| {
                                p.parse_fn_properties()
                            });
                            properties.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::Permissions => {
                            let result = self.parse_metadata_list(Symbol::Permissions, |p| {
                                p.parse_fn_permissions()
                            });
                            traits.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::BraceClose => break,
                        other => {
                            diagnostics.push(Diagnostic::new_error_simple(
                                &format!("encountered an unexpected symbol parsing function metadata: found {:?}, expected `Is` (Properties), `Uses` (Permissions), or `}}`", other),
                                &self.peek().pos,
                            ));
                            self.consume(); // Skip the unexpected token
                        }
                    }
                }

                ParserOutput {
                    output: Some((properties, traits)),
                    diagnostics,
                }
            })
            .and_then(|metadata| self.then_ignore(Symbol::BraceClose).map(|_| metadata))
    }

    fn parse_function_contracts(&mut self) -> ParserOutput<Vec<FunctionContract>> {
        self.then_ignore(Symbol::Tag)
            .and_then(|_| self.then_ignore(Symbol::Contracts))
            .and_then(|_| self.with_whitespace(|p| p.then_ignore(Symbol::BraceOpen)))
            .and_then(|_| {
                let mut contracts = Vec::new();
                let mut diagnostics = Vec::new();

                loop {
                    self.skip_whitespace();
                    match self.peek().symbol.clone() {
                        Symbol::In | Symbol::Out => {
                            let contract_type = match self.peek().symbol {
                                Symbol::In => ContractType::Input,
                                Symbol::Out => ContractType::Output,
                                _ => unreachable!(),
                            };
                            self.consume(); // Consume In/Out

                            // Parse ": ("
                            let result = self.then_ignore(Symbol::Colon).and_then(|_| {
                                self.with_whitespace(|p| p.then_ignore(Symbol::ParenOpen))
                            });

                            if result.output.is_none() {
                                diagnostics.extend(result.diagnostics);
                                self.skip_to_next_newline();
                                continue;
                            }

                            // Parse the condition expression
                            self.skip_whitespace();
                            let condition = self.parse_expr(0);
                            if condition.output.is_none() {
                                diagnostics.extend(condition.diagnostics);
                                self.skip_to_next_newline();
                                continue;
                            }

                            // Parse ", "
                            let comma_result =
                                self.with_whitespace(|p| p.then_ignore(Symbol::Comma));
                            if comma_result.output.is_none() {
                                diagnostics.extend(comma_result.diagnostics);
                                self.skip_to_next_newline();
                                continue;
                            }

                            // Parse the error message string
                            self.skip_whitespace();
                            let message = match &self.peek().symbol.clone() {
                                Symbol::StringLiteral(s) => {
                                    self.consume();
                                    s.clone()
                                }
                                _ => {
                                    diagnostics.push(Diagnostic::new_error_simple(
                                        "Expected string for contract message",
                                        &self.peek().pos,
                                    ));
                                    self.skip_to_next_newline();
                                    continue;
                                }
                            };

                            // Parse closing paren
                            let close_result =
                                self.with_whitespace(|p| p.then_ignore(Symbol::ParenClose));

                            if close_result.output.is_none() {
                                diagnostics.extend(close_result.diagnostics);
                                self.skip_to_next_newline();
                                continue;
                            }

                            contracts.push(FunctionContract {
                                type_: contract_type,
                                condition: condition.output.unwrap(),
                                message,
                            });
                        }
                        Symbol::BraceClose => break,
                        other => {
                            diagnostics.push(Diagnostic::new_error_simple(
                                &format!("Unexpected symbol in contract declaration: {:?}", other),
                                &self.peek().pos,
                            ));
                            self.consume(); // Skip the unexpected token
                        }
                    }
                }
                if contracts.len() > 0 {
                    ParserOutput {
                        output: Some(contracts),
                        diagnostics,
                    }
                } else {
                    ParserOutput {
                        output: None,
                        diagnostics,
                    }
                }
            })
            .and_then(|contracts| self.then_ignore(Symbol::BraceClose).map(|_| contracts))
    }

    fn parse_statement(&mut self) -> ParserOutput<Statement> {
        self.skip_whitespace();
        match &self.peek().symbol {
            Symbol::Let => self.parse_variable_declaration(),
            Symbol::If => self.parse_conditional(),
            Symbol::Match => self.parse_match(),
            Symbol::Return => self.parse_return(),
            Symbol::Identifier(_) => {
                // Could be function call or assignment
                let expr = self.parse_expr(0);
                if expr.output.is_none() {
                    return expr.transmute_error();
                }

                self.skip_whitespace();
                match &self.peek().symbol {
                    Symbol::Equals => {
                        // It's an assignment
                        self.consume(); // consume =
                        self.skip_whitespace();
                        let value = self.parse_expr(0);
                        if value.output.is_none() {
                            return value.transmute_error();
                        }
                        self.then_ignore(Symbol::Semicolon)
                            .map(|_| Statement::VariableMutation {
                                name: match &expr.output.unwrap() {
                                    Expr::Variable(name) => name.clone(),
                                    _ => panic!("Invalid assignment target"),
                                },
                                value: value.output.unwrap(),
                            })
                    }
                    Symbol::Semicolon => {
                        // It's a function call
                        self.consume(); // consume ;
                        ParserOutput::okay(Statement::FunctionCall(expr.output.unwrap()))
                    }
                    _ => self.single_error("Expected = or ; after expression"),
                }
            }
            _ => self.single_error("Expected statement"),
        }
    }

    fn parse_variable_declaration(&mut self) -> ParserOutput<Statement> {
        self.consume(); // consume let
        self.skip_whitespace();

        // Parse name
        let name = match &self.peek().symbol {
            Symbol::Identifier(id) => id.clone(),
            _ => return self.single_error("Expected identifier after let"),
        };
        self.consume();

        // Parse type annotation
        self.skip_whitespace();
        self.then_ignore(Symbol::Colon)
            .and_then(|_| {
                self.skip_whitespace();
                self.parse_type()
            })
            .and_then(|type_| {
                // Parse initializer
                self.skip_whitespace();
                self.then_ignore(Symbol::Equals)
                    .and_then(|_| {
                        self.skip_whitespace();
                        self.parse_expr(0)
                    })
                    .and_then(|value| {
                        self.then_ignore(Symbol::Semicolon)
                            .map(|_| Statement::VariableDeclaration { name, type_, value })
                    })
            })
    }

    fn parse_conditional(&mut self) -> ParserOutput<Statement> {
        let mut branches = Vec::new();
        let mut diagnostics = Vec::new();

        // Parse if branch
        self.consume(); // consume if
        self.skip_whitespace();

        let condition = self.parse_expr(0);
        if condition.output.is_none() {
            return condition.transmute_error();
        }

        self.skip_whitespace();
        let block_result = self.parse_block();
        if block_result.output.is_none() {
            return block_result.transmute_error();
        }

        branches.push(Branch {
            condition: Some(condition.output.unwrap()),
            computation: block_result.output.unwrap(),
        });

        // Parse elif branches
        loop {
            self.skip_whitespace();
            if self.peek().symbol != Symbol::Elif {
                break;
            }

            self.consume(); // consume elif
            self.skip_whitespace();

            let elif_condition = self.parse_expr(0);
            if elif_condition.output.is_none() {
                diagnostics.extend(elif_condition.diagnostics);
                break;
            }

            self.skip_whitespace();
            let elif_block = self.parse_block();
            if elif_block.output.is_none() {
                diagnostics.extend(elif_block.diagnostics);
                break;
            }

            branches.push(Branch {
                condition: Some(elif_condition.output.unwrap()),
                computation: elif_block.output.unwrap(),
            });
        }

        // Parse optional else branch
        self.skip_whitespace();
        if self.peek().symbol == Symbol::Else {
            self.consume();
            self.skip_whitespace();

            let else_block = self.parse_block();
            if else_block.output.is_none() {
                diagnostics.extend(else_block.diagnostics);
                return ParserOutput::err(diagnostics);
            }

            branches.push(Branch {
                condition: None,
                computation: else_block.output.unwrap(),
            });
        }

        if !diagnostics.is_empty() {
            ParserOutput::err(diagnostics)
        } else {
            ParserOutput::okay(Statement::Conditional(branches))
        }
    }

    fn parse_match(&mut self) -> ParserOutput<Statement> {
        self.consume(); // consume match
        self.skip_whitespace();

        let match_expr = self.parse_expr(0);
        if match_expr.output.is_none() {
            return match_expr.transmute_error();
        }

        self.skip_whitespace();
        let brace_result = self.then_ignore(Symbol::BraceOpen);
        if brace_result.output.is_none() {
            return brace_result.transmute_error();
        }

        let mut branches = Vec::new();
        let mut diagnostics = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek().symbol == Symbol::BraceClose {
                self.consume();
                break;
            }

            // Parse match pattern
            let condition = if self.peek().symbol == Symbol::Underscore {
                self.consume();
                None
            } else {
                let expr = self.parse_expr(0);
                if expr.output.is_none() {
                    diagnostics.extend(expr.diagnostics);
                    break;
                }
                Some(expr.output.unwrap())
            };

            self.skip_whitespace();
            let arrow_result = self.then_ignore(Symbol::FatArrow);
            if arrow_result.output.is_none() {
                diagnostics.extend(arrow_result.diagnostics);
                break;
            }

            self.skip_whitespace();
            let computation = if self.peek().symbol == Symbol::BraceOpen {
                let block_result = self.parse_block();
                if block_result.output.is_none() {
                    diagnostics.extend(block_result.diagnostics);
                    break;
                }
                block_result.output.unwrap()
            } else {
                let expr = self.parse_expr(0);
                if expr.output.is_none() {
                    diagnostics.extend(expr.diagnostics);
                    break;
                }

                let semi_result = self.then_ignore(Symbol::Semicolon);
                if semi_result.output.is_none() {
                    diagnostics.extend(semi_result.diagnostics);
                    break;
                }

                vec![Statement::Return(expr.output.unwrap())]
            };

            branches.push(Branch {
                condition,
                computation,
            });
        }

        if !diagnostics.is_empty() {
            ParserOutput::err(diagnostics)
        } else {
            ParserOutput::okay(Statement::Conditional(branches))
        }
    }

    fn parse_return(&mut self) -> ParserOutput<Statement> {
        self.consume(); // consume return
        self.skip_whitespace();

        let expr = self.parse_expr(0);
        if expr.output.is_none() {
            return expr.transmute_error();
        }

        self.then_ignore(Symbol::Semicolon)
            .map(|_| Statement::Return(expr.output.unwrap()))
    }

    fn parse_block(&mut self) -> ParserOutput<Vec<Statement>> {
        self.skip_whitespace();
        self.then_ignore(Symbol::BraceOpen).and_then(|_| {
            let mut statements = Vec::new();
            let mut diagnostics = Vec::new();

            loop {
                self.skip_whitespace();
                if self.peek().symbol == Symbol::BraceClose {
                    self.consume();
                    break;
                }

                let stmt = self.parse_statement();
                if let Some(s) = stmt.output {
                    statements.push(s);
                }
                diagnostics.extend(stmt.diagnostics);
            }

            ParserOutput {
                output: Some(statements),
                diagnostics,
            }
        })
    }
}

// -------------------- Parsing Utilities --------------------

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            offset: 0,
            tokens,
            recursion_counter: 0,
        }
    }

    /// Check the next token
    ///
    /// To avoid running out of bounds, the lexer inserts a dummy newline at the end of the input
    pub fn peek(&self) -> &Token {
        &self.tokens[self.offset]
    }

    /// Return the next token and advance the cursor
    ///
    /// To avoid running out of bounds, the lexer inserts a dummy newline at the end of the input
    pub fn consume(&mut self) -> &Token {
        let token = &self.tokens[self.offset];
        self.offset += 1;
        token
    }

    /// Helper method to create a single error from a given message
    pub fn single_error<T>(&self, message: &str) -> ParserOutput<T> {
        ParserOutput::err(vec![Diagnostic::new_error_simple(
            message,
            &self.peek().pos,
        )])
    }

    pub fn skip_whitespace(&mut self) {
        while matches!(self.peek().symbol, Symbol::Space | Symbol::NewLine)
            && self.offset < self.tokens.len()
            && self.offset < self.tokens.len() - 1
        {
            self.consume();
        }
    }

    pub fn then_ignore(&mut self, expected: Symbol) -> ParserOutput<()> {
        if self.peek().symbol == expected {
            self.consume();
            ParserOutput::okay(())
        } else {
            let message = format!(
                "expected {:?}, but found {:?}",
                expected,
                self.peek().symbol
            );
            ParserOutput::err(vec![Diagnostic::new_error_simple(
                &message,
                &self.peek().pos,
            )])
        }
    }

    fn then_identifier(&mut self) -> ParserOutput<String> {
        let next = self.consume();
        match &next.symbol {
            Symbol::Identifier(name) => ParserOutput::okay(name.to_string()),
            _ => {
                let message = format!("expected an identifier, but found {:?}", next.symbol);
                ParserOutput::err(vec![Diagnostic::new_error_simple(&message, &next.pos)])
            }
        }
    }

    fn chain<T, F>(&mut self, f: F) -> ParserOutput<T>
    where
        F: FnOnce(&mut Self) -> ParserOutput<T>,
    {
        f(self)
    }

    fn with_whitespace<T, F>(&mut self, f: F) -> ParserOutput<T>
    where
        F: FnOnce(&mut Self) -> ParserOutput<T>,
    {
        self.skip_whitespace();
        let result = f(self);
        self.skip_whitespace();
        result
    }

    fn skip_to_next_newline(&mut self) {
        loop {
            match &self.peek().symbol {
                Symbol::NewLine => {
                    self.consume();
                    break;
                }
                _ => {
                    self.consume();
                }
            }
        }
    }

    /// This parses a list of comma separated items. It doesn't handle EOF.
    pub fn parse_list_comma_separated<T, F>(&mut self, parse_item: F) -> ParserOutput<Vec<T>>
    where
        F: Fn(&mut Self) -> ParserOutput<T>,
    {
        let mut items = Vec::new();
        let mut diagnostics = Vec::new();

        loop {
            match parse_item(self) {
                ParserOutput {
                    output: Some(item),
                    diagnostics: item_diags,
                } => {
                    items.push(item);
                    diagnostics.extend(item_diags);
                    self.with_whitespace(|p| p.then_ignore(Symbol::Comma));
                }
                ParserOutput {
                    output: None,
                    diagnostics: item_diags,
                } => {
                    diagnostics.extend(item_diags);
                    break;
                }
            }
            // Symbols that denote the end of the list
            if self.peek().symbol == Symbol::BraceClose
                || self.peek().symbol == Symbol::BracketClose
                || self.peek().symbol == Symbol::Tag
                || self.peek().symbol == Symbol::Semicolon
                || self.peek().symbol == Symbol::ParenClose
            {
                break;
            }
        }

        ParserOutput {
            output: Some(items),
            diagnostics,
        }
    }

    /// This parses higher level lists, like between AST nodes, that are newline separated. It does handle EOF.
    fn parse_list_newline_separated<T, F>(&mut self, parse_item: F) -> ParserOutput<Vec<T>>
    where
        F: Fn(&mut Self) -> ParserOutput<T>,
    {
        let mut items = Vec::new();
        let mut diagnostics = Vec::new();

        while self.offset < self.tokens.len() {
            self.skip_whitespace();
            if self.offset >= self.tokens.len() {
                break;
            }

            let initial_offset = self.offset;
            match parse_item(self) {
                ParserOutput {
                    output: Some(item),
                    diagnostics: item_diags,
                } => {
                    items.push(item);
                    diagnostics.extend(item_diags);
                }
                ParserOutput {
                    output: None,
                    diagnostics: item_diags,
                } => {
                    diagnostics.extend(item_diags);
                    // If we couldn't parse an item and didn't advance, break to avoid an infinite loop
                    if self.offset == initial_offset {
                        break;
                    }
                }
            }

            // Skip any trailing whitespace or newlines
            self.skip_whitespace();
        }

        ParserOutput {
            output: Some(items),
            diagnostics,
        }
    }
}

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression_parser::BinaryOperator;
    use crate::lexer::Lexer;

    #[test]
    fn parse_fn_declaration() {
        let program_text = "fn foo(a: Int, b: Int) -> Int {";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_function_declaration();
        let expected = FunctionDeclaration {
            name: "foo".to_string(),
            parameters: vec![
                Field {
                    name: "a".to_string(),
                    field_type: Type::Integer,
                },
                Field {
                    name: "b".to_string(),
                    field_type: Type::Integer,
                },
            ],
            return_type: Type::Integer,
        };
        assert!(out.output.is_some());
        assert_eq!(out.output.unwrap(), expected);
    }

    #[test]
    fn parse_fn_metadata() {
        let program_text = r#"@metadata {
		    Is: Public;
		    Uses: ReadFile, WriteFile;
	    }"#;
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_function_metadata();
        println!("{:#?}", out);
        // Check
        let expected_properties: Vec<FunctionProperties> = vec![FunctionProperties::Public];
        let expected_permissions: Vec<FunctionPermissions> = vec![
            FunctionPermissions::ReadFile,
            FunctionPermissions::WriteFile,
        ];
        assert!(out.output.is_some());
        let (perms, props) = out.output.unwrap();
        assert_eq!(expected_permissions, props);
        assert_eq!(expected_properties, perms);
    }

    #[test]
    fn parse_fn_contracts() {
        let program_text = r#"@contracts {
		    In: (a > 0, "a must be greater than 0")
		    Out: (result > 0, "output must be greater than 0")
	    }"#;
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        let symbols = lexer
            .token_stream
            .iter()
            .map(|t| t.symbol.clone())
            .collect::<Vec<Symbol>>();
        println!("{:?}", symbols);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_function_contracts();
        println!("{:#?}", out);
        assert!(out.output.is_some());
        // Check
        let expected_in: FunctionContract = FunctionContract {
            type_: ContractType::Input,
            condition: Expr::BinaryOp {
                left: Box::new(Expr::Variable("a".to_string())),
                operator: BinaryOperator::GreaterThan,
                right: Box::new(Expr::IntegerLiteral(0)),
            },
            message: "a must be greater than 0".to_string(),
        };
        let expected_out: FunctionContract = FunctionContract {
            type_: ContractType::Output,
            condition: Expr::BinaryOp {
                left: Box::new(Expr::Variable("result".to_string())),
                operator: BinaryOperator::GreaterThan,
                right: Box::new(Expr::IntegerLiteral(0)),
            },
            message: "output must be greater than 0".to_string(),
        };
        let expected: Vec<FunctionContract> = vec![expected_in, expected_out];
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn parse_variable_declaration() {
        let program = "let x: Int = 42;";
        let mut lexer = Lexer::new("test");
        lexer.lex(program);
        let mut parser = Parser::new(lexer.token_stream);

        let result = parser.parse_statement();
        assert!(result.output.is_some());

        match result.output.unwrap() {
            Statement::VariableDeclaration { name, type_, value } => {
                assert_eq!(name, "x");
                assert_eq!(type_, Type::Integer);
                assert_eq!(value, Expr::IntegerLiteral(42));
            }
            _ => panic!("Expected VariableDeclaration"),
        }
    }

    #[test]
    fn parse_conditional() {
        let program = r#"if x > 5 {
            return 10;
        } elif x < 0 {
            return 0;
        } else {
            return 5;
        }"#;

        let mut lexer = Lexer::new("test");
        lexer.lex(program);
        let mut parser = Parser::new(lexer.token_stream);

        let result = parser.parse_statement();
        assert!(result.output.is_some());

        match result.output.unwrap() {
            Statement::Conditional(branches) => {
                assert_eq!(branches.len(), 3);

                // Check if branch
                assert!(branches[0].condition.is_some());
                assert_eq!(branches[0].computation.len(), 1);

                // Check elif branch
                assert!(branches[1].condition.is_some());
                assert_eq!(branches[1].computation.len(), 1);

                // Check else branch
                assert!(branches[2].condition.is_none());
                assert_eq!(branches[2].computation.len(), 1);
            }
            _ => panic!("Expected Conditional"),
        }
    }

    #[test]
    fn parse_match() {
        let program = r#"match x {
            0 => 42;
            1 => { return 43; }
            _ => 44;
        }"#;

        let mut lexer = Lexer::new("test");
        lexer.lex(program);
        let mut parser = Parser::new(lexer.token_stream);

        let result = parser.parse_statement();
        println!("{:#?}", result.diagnostics);
        assert!(result.output.is_some());

        match result.output.unwrap() {
            Statement::Conditional(branches) => {
                assert_eq!(branches.len(), 3);

                // Check literal match
                assert_eq!(branches[0].condition, Some(Expr::IntegerLiteral(0)));
                assert_eq!(branches[0].computation.len(), 1);

                // Check block match
                assert_eq!(branches[1].condition, Some(Expr::IntegerLiteral(1)));
                assert_eq!(branches[1].computation.len(), 1);

                // Check catch-all
                assert!(branches[2].condition.is_none());
                assert_eq!(branches[2].computation.len(), 1);
            }
            _ => panic!("Expected Conditional"),
        }
    }
}
