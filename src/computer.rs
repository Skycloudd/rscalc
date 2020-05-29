//! This module is for taking instructions generated by the parser (an AST)
//! and producing real numbers.
//! 
//! # Custom Numbers
//! The only type supported out of the box is the f64.
//! 
//! If you are implementing a number type that is not included by default, you will
//! need to implement numerous traits for that type. Here are the traits required:
//! `Num + Clone + PartialOrd + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>`

use crate::lexer::*;
use crate::parser::*;
use crate::EvalError;

use std::collections::HashMap;
use std::ops::*;

/// Because Rust does not have a generic *number* trait, any other types of
/// numbers than the support out of the box `f64` will need to have this trait
/// implemented on it.
pub trait Num {
    /// Return a zero / 0.
    fn zero() -> Self;
    /// Return a one / 1.
    fn one() -> Self;
    /// True if this Num has no decimal attached,
    /// e.g. true if 1 or 352, false if 1.14 or 352.7.
    fn is_integer(&self) -> bool;
    /// Returns the absolute value of the number.
    fn abs(&self) -> Self;
    /// Raises this number to the power of another number.
    fn pow(&self, other: &Self) -> Self;
    fn from_flt64_str(s: &str) -> Option<Self> where Self: std::marker::Sized;
}

/// Errors generated when computing for numbers.
/// 
/// # Error Lookup Table
/// | Error ID               | Description                                                                             |
/// |------------------------|-----------------------------------------------------------------------------------------|
/// | InvalidFactorial       | When trying to compute a factorial with a decimal or a number less than zero.           |
/// | VariableIsConstant     | When trying to set a constant variable's value.                                         |
/// | UnrecognizedIdentifier | When an identifier could not be resolved: it was not found in the Computer's variables. |
/// | UnrecognizedFunctionIdentifier | When the identifier could not be found in the Computer's functions.             |
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeError {
    InvalidFactorial,
    VariableIsConstant(String),
    UnrecognizedIdentifier(String),
    UnrecognizedFunctionIdentifier(String),
}
use self::ComputeError::*;

/// A Computer object calculates expressions and has variables.
/// ```
/// use rsc::{
///     EvalError,
///     computer::{Computer, ComputeError},
/// };
///
/// let mut computer = Computer::new();
/// assert_eq!(computer.eval("a = 2").unwrap(), 2.0);
/// assert_eq!(computer.eval("a * 3").unwrap(), 6.0);
///
/// // Err(EvalError::ComputeError(ComputeError::UnrecognizedIdentifier("a")))
/// Computer::new().eval("a");
/// ```
#[derive(Clone)]
pub struct Computer<'fun, T> {
    pub variables: HashMap<String, (T, bool)>, // (T, is_constant?)
    pub functions: HashMap<String, &'fun dyn Fn(T) -> T>,
}

impl<'fun> std::default::Default for Computer<'fun, f64> {
    fn default() -> Self {
        Self {
            variables: {
                let mut map = HashMap::new();
                map.insert(String::from("pi"), (std::f64::consts::PI, true));
                map.insert(String::from("e"), (std::f64::consts::E, true));
                map
            },
            functions: {
                let mut map = HashMap::<String, &'fun dyn Fn(f64) -> f64>::new();
                map.insert("sqrt".to_owned(), &|n| n.sqrt());
                map.insert("sin".to_owned(), &|n| n.sin());
                map.insert("cos".to_owned(), &|n| n.cos());
                map.insert("tan".to_owned(), &|n| n.tan());
                map.insert("log".to_owned(), &|n| n.log10());
                map
            },
        }
    }
}

impl<'fun, T: Num + Clone + PartialOrd + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>> Computer<'fun, T> {
    /// Create an empty, unconfigured Computer.
    pub fn new() -> Computer<'fun, T> {
        Computer {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// Lexically analyze, parse, and compute the given equation in string form. This does every step for you,
    /// in a single helper function.
    pub fn eval<'a>(&mut self, expr: &'a str) -> Result<T, EvalError<'a, T>> where T: std::fmt::Debug + std::str::FromStr {
        match tokenize(expr) {
            Ok(tokens) => match parse(&tokens) {
                Ok(ast) => match self.compute(&ast) {
                    Ok(num) => Ok(num),
                    Err(compute_err) => Err(EvalError::ComputeError(compute_err)),
                },
                Err(parser_err) => Err(EvalError::ParserError(parser_err)),
            },
            Err(lexer_err) => Err(EvalError::LexerError(lexer_err)),
        }
    }

    fn compute_expr<'a>(&mut self, expr: &Expr<'a, T>) -> Result<T, ComputeError> { // TODO: a lot of .to_owned() happens here to compare &'a str to Strings: there must be a more efficient way
        match expr {
            // Boolean
            Expr::BoolOp(kwd, lexpr, rexpr) => unimplemented!(),
            Expr::Bool(b) => unimplemented!(),
            Expr::BoolNot(expr) => unimplemented!(),
            Expr::BinCmp(op, lexpr, rexpr) => {
                let lval = self.compute_expr(&lexpr)?;
                let rval = self.compute_expr(&rexpr)?;
                
                match op {
                    Operator::Equals => unimplemented!(),
                    Operator::Greater => unimplemented!(),
                    Operator::GreaterEqual => unimplemented!(),
                    Operator::Lesser => unimplemented!(),
                    Operator::LesserEqual => unimplemented!(),
                    Operator::NotEquals => unimplemented!(),
                    
                    _ => unimplemented!(),
                }
            }

            // Numerical
            Expr::Constant(num) => Ok(num.clone()),
            Expr::Identifier(id) => match self.variables.get(id.to_owned()) {
                Some(value) => Ok(value.0.clone()),
                None => Err(UnrecognizedIdentifier(id.to_string())),
            },
            Expr::Neg(expr) => Ok(-self.compute_expr(expr)?),
            Expr::BinOp(op, lexpr, rexpr) => {
                let lnum = self.compute_expr(&lexpr)?;
                let rnum = self.compute_expr(&rexpr)?;

                match op {
                    Operator::Plus => Ok(lnum + rnum),
                    Operator::Minus => Ok(lnum - rnum),
                    Operator::Star => Ok(lnum * rnum),
                    Operator::Slash => Ok(lnum / rnum),

                    _ => unimplemented!(),
                }
            }
            Expr::Abs(expr) => Ok(self.compute_expr(expr)?.abs()),
            Expr::Function(id, expr) => {
                let value = self.compute_expr(&expr)?;
                match self.functions.get(id.to_owned()) {
                    Some(func) => Ok(func(value)),
                    None => Err(UnrecognizedFunctionIdentifier(id.to_string())),
                }
            }
            Expr::Assignment(id, expr) => {
                let value = self.compute_expr(&expr)?;
                if self.variables.contains_key(id.to_owned()) && self.variables.get(id.to_owned()).unwrap().1 == true {
                    return Err(VariableIsConstant(id.to_string()));
                }
                self.variables.insert((*id).to_owned(), (value.clone(), false));
                Ok(value)
            }
            Expr::Pow(lexpr, rexpr) => {
                Ok(self.compute_expr(&lexpr)?.pow(&self.compute_expr(&rexpr)?))
            }
            Expr::Factorial(expr) => {
                let mut value = self.compute_expr(&expr)?;
                if value < T::zero() || !value.is_integer() {
                    Err(InvalidFactorial)
                } else if value == T::zero() || value == T::one() {
                    Ok(T::one())
                } else {
                    let mut factor = value.clone() - T::one();
                    while factor > T::one() {
                        value = value * factor.clone();
                        factor = factor - T::one();
                    }
                    Ok(value)
                }
            }
        }
    }

    /// Solve an already parsed [Expr](../parser/struct.Expr.html) (AST).
    /// ```
    /// let ast = parse(/*...*/);
    /// let mut computer = Computer::<f64>::default();
    /// // Using this function to create the result from the `Expr`.
    /// let result = computer.compute(&ast).unwrap();
    /// ```
    pub fn compute<'a>(&mut self, expr: &Expr<'a, T>) -> Result<T, ComputeError> {
        let val = self.compute_expr(expr);
        match &val {
            Ok(n) => {
                self.variables
                    .insert(String::from("ans"), (n.clone(), true));
            }
            _ => {}
        }
        val
    }
}
