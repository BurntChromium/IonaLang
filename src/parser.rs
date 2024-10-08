//! Parser Combinator 

use std::string::Drain;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceLocation {
    file: String,
    line: usize,
    offset: usize
}

impl SourceLocation {
    fn new(filepath: &str) -> SourceLocation {
        SourceLocation {
            file: filepath.to_string(),
            line: 0,
            offset: 0
        }
    }
}

#[derive(Debug)]
enum ParserErrorClass {
    NotFound,
}

#[derive(Debug)]
pub struct ParserError {
    loc: SourceLocation,
    class: ParserErrorClass
}

impl ParserError {
    fn from_parser(parser: &Parser, class: ParserErrorClass) -> ParserError {
        ParserError {
            loc: parser.location.clone(),
            class
        }
    }
}

/// Stateful parser for text
/// 
/// ### Fields
/// 
/// - `location`: where in the file are we?
/// - `stream`: a stateful tracker for the input data
/// - `errors`: any errors we encounter along the way
/// - `commit`: state tracker - should the operation modify the state or do we roll back the operation? 
#[derive(Debug)]
pub struct Parser {
    location: SourceLocation,
    stream: String,
    errors: Vec<ParserError>,
    commit: bool
}

impl Parser {
    pub fn new(input: &str, filepath: &str) -> Parser {
        Parser {
            location: SourceLocation::new(filepath),
            stream: input.to_string(),
            errors: Vec::new(),
            commit: true
        }
    }

    /// Try to consume a pattern without returning
    pub fn then(&mut self, pattern: &str) -> &mut Self {
        let chunk_len = pattern.len();
        if self.stream.starts_with(pattern) {
            let drained = self.stream.drain(0..chunk_len).collect::<String>(); 
            self.advance_loc(drained);
            self
        } else {
            self.errors.push(ParserError::from_parser(self, ParserErrorClass::NotFound));
            self
        }
    }

    pub fn capture(&mut self, pattern: fn(&str) -> (bool, usize)) -> Option<String> {
        let (success, size) = pattern(&self.stream);
        if success {
            let drained = self.stream.drain(0..size).collect::<String>();
            self.advance_loc(drained.clone());
            return Some(drained);
        } else {
            return None;
        }
    }

    pub fn capture_as<T>(&mut self, pattern: fn(&str) -> (bool, usize, T)) -> Option<T> {
        let (success, size, out) = pattern(&self.stream);
        if success {
            let drained = self.stream.drain(0..size).collect::<String>();
            self.advance_loc(drained);
            return Some(out);
        } else {
            return None;
        }
    }

    pub fn capture_many(&mut self, pattern: fn(&str) -> (bool, usize)) -> Vec<String> {
        let mut output: Vec<String> = Vec::new();
        let mut exhausted = false;
        while !exhausted {
            let result = self.capture(pattern);
            match result {
                Some(item) => output.push(item),
                None => {exhausted = true}
            }
        }
        output
    }

    /// Consume without returning until a pattern is found
    pub fn take_until(&mut self, pattern: &str) -> &mut Self {
        loop {
            if self.stream.starts_with(pattern) {
                let drained = self.stream.drain(0..pattern.len()).collect::<String>(); 
                self.advance_loc(drained);
                return self;
            } else {
                if self.stream.len() > 1 {
                    // Advance
                    self.stream = self.stream[1..].to_string();    
                } else {
                    return self;
                }
            }
        }
    }

    /// Update the internal state tracking the location in the code
    fn advance_loc(&mut self, chunk: String) {
        if self.commit {
            for char in chunk.chars() {
                // 
                if char == 0xA as char {
                    self.location.line += 1;
                    self.location.offset = 0;
                } else {
                    self.location.offset += 1;
                }
            }
        } 
    }
}