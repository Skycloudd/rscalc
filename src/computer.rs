//! This module is for taking instructions generated by the parser (an AST)
//! and producing real numbers.

use crate::lexer::*;
use crate::parser::*;
use crate::EvalError;

use std::collections::HashMap;
use std::ops::*;

pub trait Num {
    /// Zero, 0, none
    fn zero() -> Self;
    /// One, 1, singular
    fn one() -> Self;
    /// True if this Num has no decimal attached,
    /// i.e. 1 or 352, not 1.14 or 352.7.
    fn is_integer(&self) -> bool;
    fn abs(&self) -> Self;
    fn pow(&self, other: &Self) -> Self;
}

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
/// let mut computer = Computer::new(std::f64::consts::PI, std::f64::consts::E);
/// assert_eq!(computer.eval("a = 2").unwrap(), 2.0);
/// assert_eq!(computer.eval("a * 3").unwrap(), 6.0);
/// 
/// // Err(EvalError::ComputeError(ComputeError::UnrecognizedIdentifier("a")))
/// Computer::new(std::f64::consts::PI, std::f64::consts::E).eval("a");
/// ```
#[derive(Clone)]
pub struct Computer<'a, T> {
    pub variables: HashMap<String, (T, bool)>, // (T, is_constant?)
    pub functions: HashMap<String, &'a Fn(T) -> T>,
}

pub enum ComputerResult<T> {
    Number(T),
    Frac(T, T),
}

impl<'a> std::default::Default for Computer<'a, f64> {
    fn default() -> Self {
        Self {
            variables: {
                let mut map = HashMap::new();
                map.insert(String::from("pi"), (std::f64::consts::PI, true));
                map.insert(String::from("e"), (std::f64::consts::E, true));
                map
            },
            functions: {
                let mut map = HashMap::<String, &'a Fn(f64) -> f64>::new();
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

impl<'a, T: Num + Clone + PartialOrd + Neg<Output = T> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>> Computer<'a, T> {
    pub fn new(pi_val: T, e_val: T) -> Computer<'a, T> {
        Computer {
            variables: {
                let mut map = HashMap::new();
                map.insert(String::from("pi"), (pi_val, true));
                map.insert(String::from("e"), (e_val, true));
                map
            },
            functions: {
                let mut map = HashMap::<String, &'a Fn(T) -> T>::new();
                
                map
            },
        }
    }

    /// Lexically analyze, parse, and compute the given equation in string form. This does every step for you,
    /// in a single helper function.
    pub fn eval(&mut self, expr: &str) -> Result<T, EvalError<T>> where T: std::fmt::Debug + std::str::FromStr {
        match tokenize(expr, false) {
            Ok(tokens) => match parse(&tokens) {
                Ok(ast) => match self.compute(&ast) {
                    Ok(num) => Ok(num),
                    Err(compute_err) => Err(EvalError::ComputeError(compute_err)),
                }
                Err(parser_err) => Err(EvalError::ParserError(parser_err)),
            }
            Err(lexer_err) => Err(EvalError::LexerError(lexer_err)),
        }
    }

    fn compute_expr(&mut self, expr: &Expr<T>) -> Result<T, ComputeError> {
        match expr {
            Expr::Constant(num) => Ok(num.clone()),
            Expr::Identifier(id) => {
                match self.variables.get(id) {
                    Some(value) => Ok(value.0.clone()),
                    None => Err(UnrecognizedIdentifier(id.clone())),
                }
            }
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
                match self.functions.get(id) {
                    Some(func) => Ok(func(value)),
                    None => Err(UnrecognizedFunctionIdentifier(id.clone())),
                }
            }
            Expr::Assignment(id, expr) => {
                let value = self.compute_expr(&expr)?;
                if self.variables.contains_key(id) {
                    if self.variables.get(id).unwrap().1 == true {
                        return Err(VariableIsConstant(id.clone()));
                    }
                }
                self.variables.insert(id.clone(), (value.clone(), false));
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

    /// Solve an already parsed `Expr` (AST).
    pub fn compute(&mut self, expr: &Expr<T>) -> Result<T, ComputeError> {
        let val = self.compute_expr(expr);
        match &val {
            Ok(n) => {
                self.variables.insert(String::from("ans"), (n.clone(), true));
            }
            _ => {}
        }
        val
    }
}
