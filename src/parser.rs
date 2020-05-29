// Please refer to grammar.ebnf for the outline of how the parser works. It is a mandatory read,
// and every change you make here, you must update the grammar with.

//! For using the symbols generated by the lexer and making sense of them in the context of
//! mathematical expressions.

use std::slice::Iter;
use std::iter::Peekable;
use std::fmt::Debug;

use crate::lexer::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'a, T: Clone> {
    // Booleans and comparisons
    BoolOp(Keyword, Box<Expr<'a, T>>, Box<Expr<'a, T>>),
    Bool(bool),
    BoolNot(Box<Expr<'a, T>>),
    BinCmp(Operator, Box<Expr<'a, T>>, Box<Expr<'a, T>>),

    BinOp(Operator, Box<Expr<'a, T>>, Box<Expr<'a, T>>),
    Pow(Box<Expr<'a, T>>, Box<Expr<'a, T>>),
    Neg(Box<Expr<'a, T>>),
    Abs(Box<Expr<'a, T>>),
    Factorial(Box<Expr<'a, T>>),
    Function(&'a str, Box<Expr<'a, T>>),
    Assignment(&'a str, Box<Expr<'a, T>>),
    Constant(T),
    Identifier(&'a str),
}

impl<'a, T: Clone> Expr<'a, T> {
    /// Replaces all instances of `old` with `new`. Returns the number of elements that have been replaced.
    /// # Example
    /// One could use this function to replace all references to an identifier "x" with the constant `20`.
    /// ```
    /// use rsc::{
    ///     lexer::tokenize,
    ///     parser::{parse, Expr},
    ///     computer::Computer,
    /// };
    /// let tokens = lexer::tokenize("x^2 * 4").unwrap();
    /// let replacement = parser::Expr::Constant(20.);
    /// let mut ast = parser::parse(&tokens).unwrap();
    /// ast.replace(&parser::Expr::Identifier(String::from("x")), &replacement, false);
    /// assert_eq!(Computer::new(std::f64::consts::PI, std::f64::consts::E).compute(&ast), Ok(1600.0);
    /// ```
    #[allow(dead_code)]
    pub fn replace(&mut self, old: &Expr<'static, T>, new: &Expr<'static, T>, ignore_fields: bool) -> u32
            where T: Clone + PartialEq {
        if ignore_fields {
            if std::mem::discriminant(self) == std::mem::discriminant(old) {
                *self = new.clone();
                return 1;
            }
        } else {
            if self == old {
                *self = new.clone();
                return 1;
            }
        }

        let mut replaced = 0;
        match self {
            Expr::BinOp(_, a, b) => {
                replaced += a.replace(old, new, ignore_fields);
                replaced += b.replace(old, new, ignore_fields);
            }
            Expr::Pow(a, b) => {
                replaced += a.replace(old, new, ignore_fields);
                replaced += b.replace(old, new, ignore_fields);
            }
            Expr::Neg(a) => {
                replaced += a.replace(old, new, ignore_fields);
            }
            Expr::Function(_, a) => {
                replaced += a.replace(old, new, ignore_fields);
            }
            _ => {}
        }

        replaced
    }
}

/// # Error Lookup Table
/// | Error ID                   | Description                                                                  |
/// |----------------------------|------------------------------------------------------------------------------|
/// | ExpectedClosingParenthesis | When the input is missing a right parenthesis ')'.                           |
/// | ExpectedClosingPipe        | When the input is missing a final pipe '|' on an abs expression, like: '|-2' |
/// | ExpectedFactor             | Expected to find a definite value like a variable or number, but did not.    |
/// | UnexpectedNumber           | A number was found in place of some other vital structure, ex: '24 3'        |
/// | UnexpectedToken            | A token has found to be remaining even after analysis: we don't know what to do with it.|
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError<'a, T: Clone + Debug> {
    ExpectedClosingParenthesis,
    ExpectedClosingPipe,
    /// Its value is the `Token` that was found instead of a factor.
    ExpectedFactor(Option<Token<'a, T>>),
    UnexpectedToken(Expr<'a, T>, Vec<Token<'a, T>>), // Collected only after parsing has finished... trailing tokens
    UnexpectedNumber(Token<'a, T>),
}
use self::ParserError::*;

pub type ParserResult<'a, T> = Result<Expr<'a, T>, ParserError<'a, T>>;

/// For detecting parsing errors using an iterative solution. This function can tell when
/// users accidentally enter an expression such as "2x" (when they mean "2(x)"). But just
/// as easily detects unknowingly valid expressions like "neg 3" where "neg" is currently
/// `Token::Identifier`.
pub fn preprocess<'a, T: Clone + Debug>(tokens: &[Token<'a, T>]) -> Option<ParserError<'a, T>> { // TODO: should return Result
    // Preprocess and preemptive erroring on inputs like "2x"
    let mut t = tokens.iter().peekable();
    while let Some(tok) = t.next() {
        match tok {
            Token::Number(_) => {
                if let Some(peek_tok) = t.peek() {
                    match peek_tok {
                        Token::Identifier(_) => {
                            return Some(UnexpectedNumber((*peek_tok).clone()));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Turn an array of tokens into an expression, which can be computed into a final number.
pub fn parse<'a, T: Clone + Debug>(tokens: &[Token<'a, T>]) -> ParserResult<'a, T> {
    let mut t = tokens.iter().peekable();
    match preprocess(tokens) {
        Some(e) => Err(e),
        None => {
            let expr = parse_bool_and(&mut t);
            if expr.is_ok() {
                // Are there any remaining tokens? That's an unexpected token error...
                if let Some(_) = t.peek() {
                    let mut all_tokens = vec!(t.next().unwrap().clone());
                    while let Some(next) = t.next() { // Collect every offending token
                        all_tokens.push(next.clone());
                    }
                    Err(UnexpectedToken(expr.unwrap(), all_tokens))
                } else {
                    expr
                }
            } else {
                expr
            }
        }
    }
}

/// Same as `parse`, except this does not automatically run `preprocess`. There are a few reasons one may use this function:
/// * Performance or timing
/// * AST will have identifiers that act as functions
/// * You have your own preprocess function
/// If you are not sure, use default `parse` instead.
#[allow(dead_code)]
pub fn parse_no_preprocess<'a, T: Clone + Debug>(tokens: &[Token<'a, T>]) -> ParserResult<'a, T> {
    parse_bool_and(&mut tokens.iter().peekable())
}

fn parse_bool_and<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_bool_or(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Keyword(kwd)) if kwd == &Keyword::And => {
                tokens.next();
                let r_expr = parse_bool_or(tokens)?;
                expr = Expr::BoolOp(*kwd, Box::new(expr), Box::new(r_expr));
            }
            _ => break,
        }
    }
    Ok(expr)
}

fn parse_bool_or<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_bool(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Keyword(kwd)) if kwd == &Keyword::Or => {
                tokens.next();
                let r_expr = parse_bool(tokens)?;
                expr = Expr::BoolOp(*kwd, Box::new(expr), Box::new(r_expr));
            }
            _ => break,
        }
    }
    Ok(expr)
}

fn parse_bool<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    match tokens.peek() {
        Some(Token::Keyword(Keyword::True)) => {
            tokens.next();
            Ok(Expr::Bool(true))
        }
        Some(Token::Keyword(Keyword::False)) => {
            tokens.next();
            Ok(Expr::Bool(false))
        }
        Some(Token::Operator(Operator::Exclamation)) => {
            tokens.next(); // Consume !
            Ok(Expr::BoolNot(Box::new(parse_bool(tokens)?)))
        }
        _ => parse_comparison(tokens),
    }
}

fn parse_comparison<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_additive_expr(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Operator(op)) if op == &Operator::Equals || op == &Operator::Greater
                || op == &Operator::GreaterEqual || op == &Operator::Lesser || op == &Operator::LesserEqual
                || op == &Operator::NotEquals => {
                tokens.next();
                let r_expr = parse_additive_expr(tokens)?;
                expr = Expr::BinCmp(*op, Box::new(expr), Box::new(r_expr));
            }
            _ => break,
        }
    }
    Ok(expr)
}

/// Additive expressions are things like `expr + expr`, or `expr - expr`. It reads a multiplicative
/// expr first, which allows precedence to exist.
fn parse_additive_expr<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_multiplicative_expr(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Operator(op)) if op == &Operator::Plus || op == &Operator::Minus => {
                tokens.next();
                let r_expr = parse_multiplicative_expr(tokens)?;
                expr = Expr::BinOp(*op, Box::new(expr), Box::new(r_expr));
            }
            _ => break,
        }
    }
    Ok(expr)
}

/// Multiplicative expressions are `expr * expr`, or `expr / expr`.
fn parse_multiplicative_expr<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_parenthetical_multiplicative_expr(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Operator(op)) if op == &Operator::Star || op == &Operator::Slash => {
                tokens.next();
                let r_expr = parse_parenthetical_multiplicative_expr(tokens)?;
                expr = Expr::BinOp(*op, Box::new(expr), Box::new(r_expr));
            }
            _ => break,
        }
    }
    Ok(expr)
}

/// Parenthetical, multiplicative expressions are just expressions times an expression wrapped in parenthesis: `expr(expr)`, which is
/// the same as `expr * expr`.
fn parse_parenthetical_multiplicative_expr<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_power_expr(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Operator(op)) if op == &Operator::LParen => {
                tokens.next();
                let mut internal_expr = parse_additive_expr(tokens)?;
                match tokens.next() {
                    Some(Token::Operator(op)) if op == &Operator::RParen => {
                        loop { // parse '^2' or likewise power expressions on individual parenthesis-covered expressions
                            match tokens.peek() {
                                Some(Token::Operator(op)) if op == &Operator::Caret => {
                                    tokens.next();
                                    let exponent = parse_factorial_expr(tokens)?;
                                    internal_expr = Expr::Pow(Box::new(internal_expr), Box::new(exponent));
                                }
                                _ => break,
                            }
                        }

                        expr = Expr::BinOp(Operator::Star, Box::new(expr), Box::new(internal_expr));
                    }
                    _ => return Err(ExpectedClosingParenthesis),
                }
            }
            _ => break,
        }
    }
    Ok(expr)
}

/// Power expressions are any expressions with an exponential: `factor ^ factor`.
fn parse_power_expr<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let mut expr = parse_factorial_expr(tokens)?;
    loop {
        match tokens.peek() {
            Some(Token::Operator(op)) if op == &Operator::Caret => {
                tokens.next();
                let exponent = parse_factorial_expr(tokens)?;
                expr = Expr::Pow(Box::new(expr), Box::new(exponent));
            }
            _ => break,
        }
    }
    Ok(expr)
}

fn parse_factorial_expr<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    let expr = parse_factor(tokens)?;
    match tokens.peek() {
        Some(Token::Operator(Operator::Exclamation)) => {
            tokens.next();
            Ok(Expr::Factorial(Box::new(expr)))
        }
        _ => Ok(expr),
    }
}

/// The most important item -- a factor. A factor is generally the bottom level ideas
/// like numbers or expressions in parenthesis. The factor makes the recursion in `Expr`
/// finite.
fn parse_factor<'a, T: Clone + Debug>(tokens: &mut Peekable<Iter<Token<'a, T>>>) -> ParserResult<'a, T> {
    match tokens.next() {
        // Parenthetical expressions such as `(expr)`.
        Some(Token::Operator(Operator::LParen)) => {
            let expr = parse_additive_expr(tokens);
            match tokens.next() {
                Some(Token::Operator(Operator::RParen)) => expr,
                _ => Err(ExpectedClosingParenthesis),
            }
        }
        Some(Token::Operator(Operator::Pipe)) => {
            let expr = parse_additive_expr(tokens)?;
            match tokens.next() {
                Some(Token::Operator(Operator::Pipe)) => Ok(Expr::Abs(Box::new(expr))),
                _ => return Err(ExpectedClosingPipe),
            }
        }
        Some(Token::Identifier(id)) => {
            match tokens.peek() {
                Some(Token::Operator(Operator::LParen)) => { // CONSTRUCT FUNCTION_OR_ID
                    tokens.next(); // Consume '('
                    let expr = parse_additive_expr(tokens)?;
                    match tokens.next() {
                        Some(Token::Operator(Operator::RParen)) => Ok(Expr::Function(id, Box::new(expr))),
                        _ => Err(ExpectedClosingParenthesis),
                    }
                }

                // Functions (if next is LP or PIPE or NUM or ID)
                Some(Token::Operator(Operator::Pipe)) => {
                    tokens.next(); // Consume '|'
                    let expr = parse_additive_expr(tokens)?;
                    match tokens.next() {
                        Some(Token::Operator(Operator::Pipe)) => Ok(Expr::Abs(Box::new(expr))),
                        _ => return Err(ExpectedClosingPipe),
                    }
                }
                // Some(Token::Operator(Operator::Minus)) => { Subtraction / negative arguments is probably a mixed case
                //     tokens.next(); // Consume '-'
                //     Ok(Expr::Function(id.clone(), Box::new(Expr::Neg(Box::new(parse_factor(tokens)?)))))
                // }
                Some(Token::Number(n)) => {
                    tokens.next(); // Consume number
                    Ok(Expr::Function(id, Box::new(Expr::Constant(n.clone()))))
                }
                Some(Token::Identifier(_)) => { // Function-in-a-function OR a variable being used as a function argument
                    Ok(Expr::Function(id, Box::new(parse_factor(tokens)?)))
                }

                // This is probably variable recall or variable assignment, but there is still hope...
                t => match t {
                    Some(Token::Operator(Operator::Equals)) => {
                        tokens.next();
                        Ok(Expr::Assignment(id, Box::new(parse_additive_expr(tokens)?)))
                    }
                    _ => Ok(Expr::Identifier(id)),
                    //None => Ok(Expr::Identifier(id.clone())),
                    //_ => Ok(Expr::Function(id.clone(), Box::new(parse_additive_expr(tokens)?))), // <--- HOPE
                }
            }
        }
        Some(Token::Operator(Operator::Minus)) => {
            Ok(Expr::Neg(Box::new(parse_factor(tokens)?))) // Unary negative expressions like `-factor`.
        }
        Some(Token::Number(n)) => Ok(Expr::Constant(n.clone())), // Number constants like `3`, `2.21`, `.34` or `-.2515262`.
        t => Err(ExpectedFactor(t.cloned())), // The token being read isn't in the right place.
    }
}
