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

    // List indexing
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
        Symbol::LessThan | Symbol::GreaterThan => 3,
        Symbol::Plus | Symbol::Minus => 4,
        Symbol::Times | Symbol::Divide | Symbol::Modulo => 5,
        Symbol::Dot => 6, // Property access and method calls
        _ => 0,
    }
}

impl Parser {
    fn parse_expr(&mut self, min_precedence: u8) -> ParserOutput<Expr> {
        // First parse a prefix expression
        let mut left = self.parse_prefix();
        if left.output.is_none() {
            return left;
        }

        // Keep parsing infix expressions as long as they have higher precedence
        while let Some(op_precedence) = self.peek_precedence() {
            if op_precedence < min_precedence {
                break;
            }
            left = self.parse_infix(left.output.unwrap());
        }

        left
    }

    fn parse_prefix(&mut self) -> ParserOutput<Expr> {
        match &self.peek().symbol.clone() {
            Symbol::Integer(n) => {
                self.consume();
                ParserOutput::okay(Expr::IntegerLiteral(*n))
            }
            Symbol::Float(f) => {
                self.consume();
                ParserOutput::okay(Expr::FloatLiteral(*f))
            }
            // Symbol::String(s) => {
            //     self.consume();
            //     ParserOutput::okay(Expr::StringLiteral(s.clone()))
            // }
            Symbol::Minus => {
                self.consume();
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
            Symbol::ParenOpen => {
                self.consume();
                // Parse the expression inside the parentheses.
                self.parse_expr(0).and_then(|expr| {
                    // Once the expression is successfully parsed, try to consume the closing parenthesis.
                    self.then_ignore(Symbol::ParenClose).map(|_| expr) // If successful, return the parsed expression.
                })
            }
            Symbol::Identifier(name) => {
                self.consume();
                // Look ahead to see if this is a function call
                if self.peek().symbol == Symbol::ParenOpen {
                    self.parse_function_call(name.clone())
                } else {
                    ParserOutput::okay(Expr::Variable(name.clone()))
                }
            }
            _ => self.single_error("Expected an expression"),
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
            | Symbol::Minus
            | Symbol::Times
            | Symbol::Divide
            | Symbol::Modulo
            | Symbol::LessThan
            | Symbol::GreaterThan
            | Symbol::And
            | Symbol::Or => {
                let operator = self.parse_binary_operator();
                if operator.output.is_none() {
                    return operator.transmute_error::<Expr>();
                }
                let op_precedence = precedence(&self.peek().symbol);
                self.consume();

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
            Symbol::Minus => ParserOutput::okay(BinaryOperator::Subtract),
            Symbol::Times => ParserOutput::okay(BinaryOperator::Multiply),
            Symbol::Divide => ParserOutput::okay(BinaryOperator::Divide),
            Symbol::Modulo => ParserOutput::okay(BinaryOperator::Modulo),
            Symbol::LessThan => ParserOutput::okay(BinaryOperator::LessThan),
            Symbol::GreaterThan => ParserOutput::okay(BinaryOperator::GreaterThan),
            Symbol::And => ParserOutput::okay(BinaryOperator::And),
            Symbol::Or => ParserOutput::okay(BinaryOperator::Or),
            _ => self.single_error("Expected binary operator"),
        }
    }

    fn peek_precedence(&self) -> Option<u8> {
        match &self.peek().symbol {
            Symbol::Plus
            | Symbol::Minus
            | Symbol::Times
            | Symbol::Divide
            | Symbol::Modulo
            | Symbol::LessThan
            | Symbol::GreaterThan
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
    fn test_expr_1() {
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
}
