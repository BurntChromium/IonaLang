//! Pull expression parsing out into a module for readability
//!
//! This uses a Pratt parser
/*
- parse_expr(0)
  ├─ prefix_parse()
  │  ├─ parse_literal() → IntegerLiteral, FloatLiteral, StringLiteral
  │  ├─ parse_identifier() → Variable or FunctionCall (if followed by parentheses)
  │  ├─ parse_unary() → UnaryOp
  │  └─ parse_grouped() → handles parentheses for grouping
  │
  └─ infix_parse(left)
     ├─ parse_binary() → BinaryOp
     ├─ parse_method_call() → MethodCall (when dot is followed by identifier and parentheses)
     ├─ parse_property() → PropertyAccess (when dot is followed by identifier)
     └─ parse_index() → IndexAccess (when left is followed by square brackets)
*/

use crate::lexer::Symbol;
use crate::parser::*;

// Core Expression enum
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Literals
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),

    // Variables and properties
    Variable(String),
    PropertyAccess {
        object: Box<Expr>,
        property: String,
    },

    // Function and method calls
    FunctionCall {
        name: String,
        arguments: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
    },

    // Operators
    BinaryOp {
        left: Box<Expr>,
        operator: BinaryOperator,
        right: Box<Expr>,
    },
    UnaryOp {
        operator: UnaryOperator,
        operand: Box<Expr>,
    },

    // List and tuple indexing
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,         // +
    Subtract,    // -
    Multiply,    // *
    Divide,      // /
    Modulo,      // %
    LessThan,    // <
    GreaterThan, // >
    And,         // and
    Or,          // or
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate, // -
}

// Precedence levels for operators
const fn precedence(op: &Symbol) -> u8 {
    match op {
        Symbol::Or => 1,
        Symbol::And => 2,
        Symbol::LeftAngle | Symbol::RightAngle => 3,
        Symbol::Plus | Symbol::Dash => 4,
        Symbol::Times | Symbol::Divide | Symbol::Modulo => 5,
        Symbol::Dot => 6, // Property access and method calls
        _ => 0,
    }
}

impl Parser {
    pub fn parse_expr(&mut self, min_precedence: u8) -> ParserOutput<Expr> {
        // Track our recursion depth
        self.recursion_counter += 1;
        if self.recursion_counter > 30 {
            panic!("maximum recursion depth exceeded while parsing an expression!")
        }
        // First parse a prefix expression
        let mut left = self.parse_prefix();
        if left.output.is_none() {
            return left;
        }

        // Keep parsing infix expressions as long as they have higher precedence
        loop {
            self.skip_whitespace(); // Safe to skip here - we're looking for infix operators

            if let Some(op_precedence) = self.peek_precedence() {
                if op_precedence < min_precedence {
                    break;
                }
                left = self.parse_infix(left.output.unwrap());
                if left.output.is_none() {
                    break;
                }
            } else {
                break;
            }
        }

        left
    }

    fn parse_prefix(&mut self) -> ParserOutput<Expr> {
        // Don't skip whitespace here - we need to properly detect unary operators
        // We have to clone to avoid mut+immutable issues
        match &self.peek().symbol.clone() {
            Symbol::Dash => {
                self.consume();
                self.skip_whitespace(); // Safe to skip after consuming the unary operator
                                        // Parse the operand with high precedence to ensure right association
                let operand = self.parse_expr(6);
                if operand.output.is_none() {
                    return operand;
                }
                ParserOutput::okay(Expr::UnaryOp {
                    operator: UnaryOperator::Negate,
                    operand: Box::new(operand.output.unwrap()),
                })
            }
            Symbol::Integer(n) => {
                self.consume();
                ParserOutput::okay(Expr::IntegerLiteral(*n))
            }
            Symbol::Float(f) => {
                self.consume();
                ParserOutput::okay(Expr::FloatLiteral(*f))
            }
            Symbol::StringLiteral(s) => {
                self.consume();
                ParserOutput::okay(Expr::StringLiteral(s.clone()))
            }
            Symbol::ParenOpen => {
                self.consume();
                self.skip_whitespace(); // Safe to skip inside parentheses
                self.parse_expr(0).and_then(|expr| {
                    self.skip_whitespace(); // Safe to skip before closing paren
                    self.then_ignore(Symbol::ParenClose).map(|_| expr)
                })
            }
            Symbol::Identifier(name) => {
                self.consume();
                self.skip_whitespace(); // Safe to skip after identifier
                                        // Look ahead to see if this is a function call
                if self.peek().symbol == Symbol::ParenOpen {
                    self.parse_function_call(name.clone())
                } else {
                    ParserOutput::okay(Expr::Variable(name.clone()))
                }
            }
            other => self.single_error(&format!(
                "Expected the beginning of an expression, but found {:?}",
                other
            )),
        }
    }

    fn parse_function_call(&mut self, name: String) -> ParserOutput<Expr> {
        // Consume opening parenthesis
        // self.then_ignore(Symbol::ParenOpen);
        self.consume();

        // Parse comma-separated arguments
        self.parse_list_comma_separated(|p| p.parse_expr(0))
            .and_then(|args| {
                self.then_ignore(Symbol::ParenClose)
                    .map(|_| Expr::FunctionCall {
                        name: name,
                        arguments: args,
                    })
            })
    }

    fn parse_infix(&mut self, left: Expr) -> ParserOutput<Expr> {
        match &self.peek().symbol {
            Symbol::Plus
            | Symbol::Dash
            | Symbol::Times
            | Symbol::Divide
            | Symbol::Modulo
            | Symbol::LeftAngle
            | Symbol::RightAngle
            | Symbol::And
            | Symbol::Or => {
                let op_precedence = precedence(&self.peek().symbol);
                let operator = self.parse_binary_operator();
                if operator.output.is_none() {
                    return operator.transmute_error::<Expr>();
                }
                self.consume();
                self.skip_whitespace(); // Safe to skip after operator

                // Parse the right side with precedence one higher for left association
                let right = self.parse_expr(op_precedence + 1);
                if right.output.is_none() {
                    return right.transmute_error::<Expr>();
                }

                ParserOutput::okay(Expr::BinaryOp {
                    left: Box::new(left),
                    operator: operator.output.unwrap(),
                    right: Box::new(right.output.unwrap()),
                })
            }
            Symbol::Dot => {
                self.consume();
                match &self.peek().symbol.clone() {
                    Symbol::Identifier(name) => {
                        self.consume();
                        if self.peek().symbol == Symbol::ParenOpen {
                            // Method call
                            self.consume();
                            let arguments: Vec<Expr>;
                            if self.peek().symbol == Symbol::ParenClose {
                                arguments = vec![]
                            } else {
                                let possible = self.parse_list_comma_separated(|p| p.parse_expr(0));
                                if possible.output.is_none() {
                                    return possible.transmute_error::<Expr>();
                                } else {
                                    arguments = possible.output.unwrap();
                                }
                            };
                            self.then_ignore(Symbol::ParenClose);

                            ParserOutput::okay(Expr::MethodCall {
                                object: Box::new(left),
                                method: name.clone(),
                                arguments,
                            })
                        } else {
                            // Property access
                            ParserOutput::okay(Expr::PropertyAccess {
                                object: Box::new(left),
                                property: name.clone(),
                            })
                        }
                    }
                    _ => self.single_error("Expected property or method name after dot"),
                }
            }
            Symbol::BracketOpen => {
                self.consume();
                let index = self.parse_expr(0);
                self.then_ignore(Symbol::BracketClose);

                if index.output.is_none() {
                    return index;
                }

                ParserOutput::okay(Expr::IndexAccess {
                    object: Box::new(left),
                    index: Box::new(index.output.unwrap()),
                })
            }
            _ => self.single_error("Expected operator, dot, or index access"),
        }
    }

    fn parse_binary_operator(&mut self) -> ParserOutput<BinaryOperator> {
        match &self.peek().symbol {
            Symbol::Plus => ParserOutput::okay(BinaryOperator::Add),
            Symbol::Dash => ParserOutput::okay(BinaryOperator::Subtract),
            Symbol::Times => ParserOutput::okay(BinaryOperator::Multiply),
            Symbol::Divide => ParserOutput::okay(BinaryOperator::Divide),
            Symbol::Modulo => ParserOutput::okay(BinaryOperator::Modulo),
            Symbol::LeftAngle => ParserOutput::okay(BinaryOperator::LessThan),
            Symbol::RightAngle => ParserOutput::okay(BinaryOperator::GreaterThan),
            Symbol::And => ParserOutput::okay(BinaryOperator::And),
            Symbol::Or => ParserOutput::okay(BinaryOperator::Or),
            _ => self.single_error("Expected binary operator"),
        }
    }

    fn peek_precedence(&self) -> Option<u8> {
        // If we have any of these symbols, run the precedence function on it
        match &self.peek().symbol {
            Symbol::Plus
            | Symbol::Dash
            | Symbol::Times
            | Symbol::Divide
            | Symbol::Modulo
            | Symbol::LeftAngle
            | Symbol::RightAngle
            | Symbol::And
            | Symbol::Or
            | Symbol::Dot
            | Symbol::BracketOpen => Some(precedence(&self.peek().symbol)),
            _ => None,
        }
    }
}

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn expr_1() {
        let program_text = "5";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        let expected = Expr::IntegerLiteral(5);
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_2() {
        let program_text = "5.39";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        let expected = Expr::FloatLiteral(5.39);
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_3() {
        let program_text = "-5";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        println!("{:#?}", out);
        let expected = Expr::UnaryOp {
            operator: UnaryOperator::Negate,
            operand: Box::new(Expr::IntegerLiteral(5)),
        };
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_4() {
        let program_text = "2+5";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        println!("{:#?}", out);
        let expected = Expr::BinaryOp {
            left: Box::new(Expr::IntegerLiteral(2)),
            operator: BinaryOperator::Add,
            right: Box::new(Expr::IntegerLiteral(5)),
        };
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_5() {
        let program_text = "2 + 5";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        println!("{:#?}", out);
        let expected = Expr::BinaryOp {
            left: Box::new(Expr::IntegerLiteral(2)),
            operator: BinaryOperator::Add,
            right: Box::new(Expr::IntegerLiteral(5)),
        };
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_6() {
        let program_text = "add(2, 5)";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        println!("{:#?}", out);
        let expected = Expr::FunctionCall {
            name: "add".to_string(),
            arguments: vec![Expr::IntegerLiteral(2), Expr::IntegerLiteral(5)],
        };
        assert_eq!(expected, out.output.unwrap());
    }

    #[test]
    fn expr_7() {
        let program_text = "add(2, 5 * a)";
        // Lex
        let mut lexer = Lexer::new("test");
        lexer.lex(&program_text);
        println!("{:#?}", lexer.token_stream);
        // Parse
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_expr(0);
        println!("{:#?}", out);
        let expected = Expr::FunctionCall {
            name: "add".to_string(),
            arguments: vec![
                Expr::IntegerLiteral(2),
                Expr::BinaryOp {
                    left: Box::new(Expr::IntegerLiteral(5)),
                    operator: BinaryOperator::Multiply,
                    right: Box::new(Expr::Variable("a".to_string())),
                },
            ],
        };
        assert_eq!(expected, out.output.unwrap());
    }
}
