//! Recursive Descent Parser
use crate::diagnostics::Diagnostic;
use crate::lexer::{Symbol, Token};

// -------------------- Parser Object --------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parser {
    tokens: Vec<Token>,
    offset: usize,
}

/// Golang-esque error handling to allow multiple returns
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserOutput<T> {
    pub output: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> ParserOutput<T> {
    fn okay(output: T) -> Self {
        ParserOutput {
            output: Some(output),
            diagnostics: vec![],
        }
    }

    fn err(diagnostics: Vec<Diagnostic>) -> Self {
        ParserOutput {
            output: None,
            diagnostics,
        }
    }

    /// Errors can be safely cast from one type to another because only the output varies
    ///
    /// This lets return a parser error from an "inner" parser error that returns type T from an outer parser that returns type O
    fn transmute_error<O>(self) -> ParserOutput<O> {
        ParserOutput {
            output: None,
            diagnostics: self.diagnostics,
        }
    }
}

trait ParserOutputExt<T> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub args: Vec<Field>,
    pub returns: Type,
    pub properties: Vec<FunctionProperties>,
    pub permissions: Vec<FunctionPermissions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTNode {
    StructDeclaration(Struct),
    EnumDeclaration(Enum),
    ImportStatement(Import),
}

// -------------------- Parsers --------------------

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
            .and_then(|_| self.with_whitespace(|p| p.then_ignore(Symbol::BracketOpen)))
            .and_then(|_| self.parse_list_comma_separated(|p| parse_item(p)))
            .and_then(|values| self.then_ignore(Symbol::BracketClose).map(|_| values))
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

// -------------------| Parse Functions |--------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionDeclaration {
    pub name: String,
    pub parameters: Vec<Field>,
    pub return_type: Type,
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

    fn parse_metadata_functions(
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
                    match self.peek().symbol {
                        Symbol::Properties => {
                            let result = self.parse_metadata_list(Symbol::Properties, |p| {
                                p.parse_fn_properties()
                            });
                            properties.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::Traits => {
                            let result = self.parse_metadata_list(Symbol::Permissions, |p| {
                                p.parse_fn_permissions()
                            });
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

// -------------------- Parsing Utilities --------------------

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { offset: 0, tokens }
    }

    /// Check the next token
    ///
    /// To avoid running out of bounds, the lexer inserts a dummy newline at the end of the input
    fn peek(&self) -> &Token {
        &self.tokens[self.offset]
    }

    /// Return the next token and advance the cursor
    ///
    /// To avoid running out of bounds, the lexer inserts a dummy newline at the end of the input
    fn consume(&mut self) -> &Token {
        let token = &self.tokens[self.offset];
        self.offset += 1;
        token
    }

    /// Helper method to create a single error from a given message
    fn single_error<T>(&self, message: &str) -> ParserOutput<T> {
        ParserOutput::err(vec![Diagnostic::new_error_simple(
            message,
            &self.peek().pos,
        )])
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek().symbol, Symbol::Space | Symbol::NewLine)
            && self.offset < self.tokens.len()
            && self.offset < self.tokens.len() - 1
        {
            self.consume();
        }
    }

    fn then_ignore(&mut self, expected: Symbol) -> ParserOutput<()> {
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

    /// This parses a list of comma separated items. It doesn't handle EOF.
    fn parse_list_comma_separated<T, F>(&mut self, parse_item: F) -> ParserOutput<Vec<T>>
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
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_fn_declaration() {
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
}
