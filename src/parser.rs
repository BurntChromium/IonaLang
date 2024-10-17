//! Recursive Descent Parser
use crate::diagnostics::Diagnostic;
use crate::lexer::{Symbol, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parser {
    tokens: Vec<Token>,
    offset: usize,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    name: String,
    type_: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    fields: Vec<Field>,
    properties: Vec<String>,
    traits: Vec<String>,
}

impl Parser {
    pub fn parse_struct_declaration(&mut self) -> ParserOutput<String> {
        self.expect_token(Symbol::Struct)
            .and_then(|_| self.with_whitespace(|p| p.expect_identifier()))
            .and_then(|name| {
                self.with_whitespace(|p| p.expect_token(Symbol::BraceOpen).map(|_| name))
            })
    }

    pub fn parse_field(&mut self) -> ParserOutput<Field> {
        self.expect_identifier().and_then(|name| {
            self.with_whitespace(|p| p.expect_token(Symbol::Colon))
                .and_then(|_| self.with_whitespace(|p| p.expect_identifier()))
                .map(|type_| Field { name, type_ })
        })
    }

    fn parse_metadata_entry(&mut self, expected_symbol: Symbol) -> ParserOutput<Vec<String>> {
        self.expect_token(expected_symbol)
            .and_then(|_| self.with_whitespace(|p| p.expect_token(Symbol::Colon)))
            .and_then(|_| self.with_whitespace(|p| p.expect_token(Symbol::BracketOpen)))
            .and_then(|_| self.parse_list(|p| p.expect_identifier()))
            .and_then(|values| self.expect_token(Symbol::BracketClose).map(|_| values))
    }

    fn parse_metadata(&mut self) -> ParserOutput<(Vec<String>, Vec<String>)> {
        self.expect_token(Symbol::Tag)
            .and_then(|_| self.expect_token(Symbol::Metadata))
            .and_then(|_| self.with_whitespace(|p| p.expect_token(Symbol::BraceOpen)))
            .and_then(|_| {
                let mut properties = Vec::new();
                let mut traits = Vec::new();
                let mut diagnostics = Vec::new();

                loop {
                    self.skip_whitespace();
                    match self.peek().symbol {
                        Symbol::Properties => {
                            let result = self.parse_metadata_entry(Symbol::Properties);
                            properties.extend(result.output.unwrap_or_default());
                            diagnostics.extend(result.diagnostics);
                        }
                        Symbol::Traits => {
                            let result = self.parse_metadata_entry(Symbol::Traits);
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
            .and_then(|metadata| self.expect_token(Symbol::BraceClose).map(|_| metadata))
    }

    pub fn parse_struct(&mut self) -> ParserOutput<Struct> {
        self.parse_struct_declaration()
            .and_then(|_| self.parse_list(|p| p.with_whitespace(|p| p.parse_field())))
            .and_then(|fields| {
                let metadata = self.parse_metadata();
                metadata.map(|(properties, traits)| Struct {
                    fields,
                    properties,
                    traits,
                })
            })
            .and_then(|struct_| {
                self.with_whitespace(|p| p.expect_token(Symbol::BraceClose))
                    .map(|_| struct_)
            })
    }
}

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

    fn skip_whitespace(&mut self) {
        while matches!(self.peek().symbol, Symbol::Space | Symbol::NewLine)
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

    fn expect_token(&mut self, symbol: Symbol) -> ParserOutput<()> {
        self.then_ignore(symbol)
    }

    fn expect_identifier(&mut self) -> ParserOutput<String> {
        self.then_identifier()
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

    fn parse_list<T, F>(&mut self, parse_item: F) -> ParserOutput<Vec<T>>
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
                    self.with_whitespace(|p| p.expect_token(Symbol::Comma));
                }
                ParserOutput {
                    output: None,
                    diagnostics: item_diags,
                } => {
                    diagnostics.extend(item_diags);
                    break;
                }
            }
            if self.peek().symbol == Symbol::BraceClose
                || self.peek().symbol == Symbol::BracketClose
                || self.peek().symbol == Symbol::Tag
            {
                break;
            }
        }

        ParserOutput {
            output: Some(items),
            diagnostics,
        }
    }
}
