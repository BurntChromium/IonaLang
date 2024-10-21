//! Split text stream into tokens

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    pub filename: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    Identifier(String),
    Integer(i64),
    Import,
    Struct,
    Enum,
    Generic,
    With,
    Colon,
    Comma,
    Tag,
    Metadata,
    Properties,
    Traits,
    Semicolon,
    BraceOpen,    // {
    BraceClose,   // }
    BracketOpen,  // [
    BracketClose, // ]
    AngleOpen,    // <
    AngleClose,   // >
    Space,
    NewLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
                    self.simple_add(Symbol::AngleOpen, 1);
                    chars.next();
                }
                '>' => {
                    self.simple_add(Symbol::AngleClose, 1);
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
                        "with" => self.simple_add(Symbol::With, word_len),
                        "metadata" => self.simple_add(Symbol::Metadata, word_len),
                        "Is" => self.simple_add(Symbol::Properties, word_len),
                        "Derives" => self.simple_add(Symbol::Traits, word_len),
                        "Generic" => self.simple_add(Symbol::Generic, word_len),
                        _ => self.simple_add(Symbol::Identifier(word), word_len),
                    }
                }
                c if c.is_numeric() => {
                    let number: String = chars.by_ref().take_while(|&ch| ch.is_numeric()).collect();
                    if let Ok(n) = number.parse() {
                        self.simple_add(Symbol::Integer(n), number.len());
                    } else {
                        // Handle error
                    }
                }
                _ => {
                    // Handle unexpected characters
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
