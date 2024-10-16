//! Recursive Descent Parser
use crate::diagnostics::{self, Diagnostic};
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

struct StructField {
    name: String,
    type_: String,
}

struct Struct {
    fields: Vec<StructField>,
    properties: Vec<String>,
    traits: Vec<String>,
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

    pub fn parse_struct_declaration(&mut self) -> ParserOutput<String> {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        // Capture struct
        let mut o = self.then_ignore(Symbol::Struct);
        if o.output.is_none() {
            diagnostics.append(&mut o.diagnostics);
        }
        self.skip_whitespace();
        // Capture name
        let mut name = self.then_identifier();
        if name.output.is_none() {
            diagnostics.append(&mut name.diagnostics);
        }
        self.skip_whitespace();
        // Capture {
        o = self.then_ignore(Symbol::BraceOpen);
        if o.output.is_none() {
            diagnostics.append(&mut o.diagnostics);
        }
        self.skip_whitespace();
        if diagnostics.len() > 0 {
            ParserOutput::err(diagnostics)
        } else {
            ParserOutput::okay(name.output.unwrap())
        }
    }
}
