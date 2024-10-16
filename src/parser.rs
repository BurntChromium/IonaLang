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
    fn parse_struct_declaration(&mut self) -> ParserOutput<String> {
        self.expect_token(Symbol::Struct)
            .and_then(|_| self.with_whitespace(|p| p.expect_identifier()))
            .and_then(|name| {
                self.with_whitespace(|p| p.expect_token(Symbol::BraceOpen).map(|_| name))
            })
    }

    fn parse_field(&mut self) -> ParserOutput<Field> {
        self.expect_identifier().and_then(|name| {
            self.with_whitespace(|p| p.expect_token(Symbol::Colon))
                .and_then(|_| self.with_whitespace(|p| p.expect_identifier()))
                .map(|type_| Field { name, type_ })
        })
    }

    fn parse_metadata_entry(&mut self) -> ParserOutput<(String, Vec<String>)> {
        self.expect_identifier().and_then(|key| {
            self.with_whitespace(|p| p.expect_token(Symbol::Colon))
                .and_then(|_| self.with_whitespace(|p| p.expect_token(Symbol::BracketOpen)))
                .and_then(|_| self.parse_list(|p| p.expect_identifier()))
                .and_then(|values| {
                    self.expect_token(Symbol::BracketClose)
                        .map(|_| (key, values))
                })
        })
    }

    fn parse_metadata(&mut self) -> ParserOutput<(Vec<String>, Vec<String>)> {
        self.expect_token(Symbol::Tag)
            .and_then(|_| self.expect_token(Symbol::Metadata))
            .and_then(|_| self.with_whitespace(|p| p.expect_token(Symbol::BraceOpen)))
            .and_then(|_| self.parse_list(|p| p.with_whitespace(|p| p.parse_metadata_entry())))
            .and_then(|entries| {
                self.expect_token(Symbol::BraceClose).map(|_| {
                    let mut properties = Vec::new();
                    let mut traits = Vec::new();
                    for (key, values) in entries {
                        match key.as_str() {
                            "Is" => properties.extend(values),
                            "Derives" => traits.extend(values),
                            _ => {} // Ignore unknown metadata keys
                        }
                    }
                    (properties, traits)
                })
            })
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
            .and_then(|struct_| self.expect_token(Symbol::BraceClose).map(|_| struct_))
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { offset: 0, tokens }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.offset]
    }

    fn consume(&mut self) -> &Token {
        let token = &self.tokens[self.offset];
        self.offset += 1;
        token
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek().symbol, Symbol::Space | Symbol::NewLine) {
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
