//! Split text stream into tokens

use crate::diagnostics::Diagnostic;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    pub filename: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Identifier(String),
    Integer(i64),
    Float(f64),
    Import,
    Struct,
    Enum,
    Function,
    Generic,
    With,
    Colon,
    Comma,
    Tag, // @
    Metadata,
    Properties,
    Traits,
    Permissions,
    Semicolon,
    BraceOpen,    // {
    BraceClose,   // }
    BracketOpen,  // [
    BracketClose, // ]
    LeftAngle,    // <
    RightAngle,   // >
    ParenOpen,    // (
    ParenClose,   // )
    Dash,         // -
    Dot,          // .
    Or,
    And,
    LessThan,
    GreaterThan,
    Plus,
    Times,
    Divide,
    Modulo,
    Space,
    NewLine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub symbol: Symbol,
    pub pos: SourcePosition,
}

impl Token {
    fn new(symbol: Symbol, pos: &SourcePosition) -> Token {
        Token {
            symbol,
            pos: pos.clone(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?} ({}, {})",
            self.symbol, self.pos.line, self.pos.column
        )
    }
}

pub struct Lexer {
    pub token_stream: Vec<Token>,
    position: SourcePosition,
    pub diagnostics: Vec<Diagnostic>,
}

impl Lexer {
    pub fn new(filename: &str) -> Lexer {
        Lexer {
            token_stream: Vec::new(),
            position: SourcePosition {
                filename: filename.to_string(),
                line: 0,
                column: 0,
            },
            diagnostics: Vec::new(),
        }
    }

    /// Handle the standard case for inserting a new token
    fn simple_add(&mut self, symbol: Symbol, input_len: usize) {
        self.token_stream.push(Token::new(symbol, &self.position));
        self.position.column += input_len;
    }

    pub fn lex(&mut self, code: &str) {
        let mut chars = code.chars().peekable();
        while let Some(&c) = chars.peek() {
            match c {
                // Consume comments until a line break
                '#' => {
                    while let Some(&ch) = chars.peek() {
                        if ch != '\n' {
                            chars.next(); // consume the character
                        } else {
                            break; // Stop at the end of the line
                        }
                    }
                }
                '\n' => {
                    self.simple_add(Symbol::NewLine, 1);
                    chars.next();
                    // Manually set position -- this overwrites/undoes the change in simple_add
                    self.position.line += 1;
                    self.position.column = 0;
                }
                '\t' => {
                    self.simple_add(Symbol::Space, 4);
                    chars.next();
                }
                c if c.is_whitespace() => {
                    self.simple_add(Symbol::Space, c.len_utf8());
                    chars.next();
                }
                ';' => {
                    self.simple_add(Symbol::Semicolon, 1);
                    chars.next();
                }
                '{' => {
                    self.simple_add(Symbol::BraceOpen, 1);
                    chars.next();
                }
                '}' => {
                    self.simple_add(Symbol::BraceClose, 1);
                    chars.next();
                }
                '[' => {
                    self.simple_add(Symbol::BracketOpen, 1);
                    chars.next();
                }
                ']' => {
                    self.simple_add(Symbol::BracketClose, 1);
                    chars.next();
                }
                '<' => {
                    self.simple_add(Symbol::LeftAngle, 1);
                    chars.next();
                }
                '>' => {
                    self.simple_add(Symbol::RightAngle, 1);
                    chars.next();
                }
                '(' => {
                    self.simple_add(Symbol::ParenOpen, 1);
                    chars.next();
                }
                ')' => {
                    self.simple_add(Symbol::ParenClose, 1);
                    chars.next();
                }
                '-' => {
                    self.simple_add(Symbol::Dash, 1);
                    chars.next();
                }
                '.' => {
                    self.simple_add(Symbol::Dot, 1);
                    chars.next();
                }
                ':' => {
                    self.simple_add(Symbol::Colon, 1);
                    chars.next();
                }
                '@' => {
                    self.simple_add(Symbol::Tag, 1);
                    chars.next();
                }
                ',' => {
                    self.simple_add(Symbol::Comma, 1);
                    chars.next();
                }
                '+' => {
                    self.simple_add(Symbol::Plus, 1);
                }
                '/' => {
                    self.simple_add(Symbol::Divide, 1);
                }
                '*' => {
                    self.simple_add(Symbol::Times, 1);
                }
                '%' => {
                    self.simple_add(Symbol::Modulo, 1);
                }
                c if c.is_whitespace() => {
                    println!("some other space? {}", c);
                    self.simple_add(Symbol::Space, c.len_utf8());
                    chars.next();
                }
                c if c.is_alphabetic() => {
                    // We can't use take_while because it's too aggressive with whitespace
                    let mut word = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            word.push(ch);
                            chars.next(); // consume the character
                        } else {
                            break; // Stop when the next character isn't alphanumeric
                        }
                    }
                    let word_len = word.len();
                    match word.as_str() {
                        "import" => self.simple_add(Symbol::Import, word_len),
                        "struct" => self.simple_add(Symbol::Struct, word_len),
                        "enum" => self.simple_add(Symbol::Enum, word_len),
                        "fn" => self.simple_add(Symbol::Function, word_len),
                        "with" => self.simple_add(Symbol::With, word_len),
                        "metadata" => self.simple_add(Symbol::Metadata, word_len),
                        "Is" => self.simple_add(Symbol::Properties, word_len),
                        "Derives" => self.simple_add(Symbol::Traits, word_len),
                        "Uses" => self.simple_add(Symbol::Permissions, word_len),
                        "Generic" => self.simple_add(Symbol::Generic, word_len),
                        _ => self.simple_add(Symbol::Identifier(word), word_len),
                    }
                }
                c if c.is_numeric() => {
                    let mut number: String = chars
                        .by_ref()
                        .take_while(|&ch| ch.is_numeric() || ch == '.') // Allow decimal point in take_while
                        .collect();
                    if number.contains('.') {
                        let tail: String =
                            chars.by_ref().take_while(|&ch| ch.is_numeric()).collect();
                        number.push_str(&tail);
                        if let Ok(f) = number.parse() {
                            self.simple_add(Symbol::Float(f), number.len());
                        } else {
                            // Handle error
                        }
                    } else {
                        if let Ok(n) = number.parse() {
                            self.simple_add(Symbol::Integer(n), number.len());
                        } else {
                            // Handle error
                        }
                    }
                }
                other => {
                    // Handle unexpected characters
                    self.diagnostics.push(Diagnostic::new_error_simple(
                        &format!("Unexpected symbol in program {}", other),
                        &self.position,
                    ));
                    chars.next();
                }
            }
        }
        // Add trailing whitespace to avoid over-running the token boundary during parsing
        self.position.line += 1;
        self.position.column = 0;
        self.simple_add(Symbol::NewLine, 1);
    }
}

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn lex_int() {
        let input_int = "64";
        let mut lexer = Lexer::new("test");
        lexer.lex(&input_int);
        assert_eq!(lexer.token_stream[0].symbol, Symbol::Integer(64));
    }

    #[test]
    fn lex_float() {
        let input_int = "3947.2884";
        let mut lexer = Lexer::new("test");
        lexer.lex(&input_int);
        assert_eq!(lexer.token_stream[0].symbol, Symbol::Float(3947.2884));
    }

    #[test]
    fn lex_function_call_variables() {
        let input_int = "foo(a, b)";
        let mut lexer = Lexer::new("test");
        lexer.lex(&input_int);
        let symbols = lexer
            .token_stream
            .iter()
            .map(|t| t.symbol.clone())
            .collect::<Vec<Symbol>>();
        assert_eq!(
            symbols,
            vec![
                Symbol::Identifier("foo".to_string()),
                Symbol::ParenOpen,
                Symbol::Identifier("a".to_string()),
                Symbol::Comma,
                Symbol::Space,
                Symbol::Identifier("b".to_string()),
                Symbol::ParenClose,
                Symbol::NewLine
            ]
        );
    }

    #[test]
    fn lex_function_call_ints() {
        let input_int = "foo(1, 2)";
        let mut lexer = Lexer::new("test");
        lexer.lex(&input_int);
        let symbols = lexer
            .token_stream
            .iter()
            .map(|t| t.symbol.clone())
            .collect::<Vec<Symbol>>();
        assert_eq!(
            symbols,
            vec![
                Symbol::Identifier("foo".to_string()),
                Symbol::ParenOpen,
                Symbol::Integer(1),
                Symbol::Comma,
                Symbol::Space,
                Symbol::Integer(2),
                Symbol::ParenClose,
                Symbol::NewLine
            ]
        );
    }

    #[test]
    fn lex_function_call_floats() {
        let input_int = "sub(1.2, 3.4)";
        let mut lexer = Lexer::new("test");
        lexer.lex(&input_int);
        let symbols = lexer
            .token_stream
            .iter()
            .map(|t| t.symbol.clone())
            .collect::<Vec<Symbol>>();
        assert_eq!(
            symbols,
            vec![
                Symbol::Identifier("foo".to_string()),
                Symbol::ParenOpen,
                Symbol::Float(1.2),
                Symbol::Comma,
                Symbol::Space,
                Symbol::Float(3.4),
                Symbol::ParenClose,
                Symbol::NewLine
            ]
        );
    }
}
