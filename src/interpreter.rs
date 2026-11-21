use std::collections::HashMap;

use thiserror::Error;

use crate::frontend::{
    lexer::Spanned,
    parser::{BinaryOperator, Command, CommandKind, Expression, ExpressionKind, Program},
};

#[derive(Debug, PartialEq, Error)]
pub enum EvalErrorKind {
    #[error("arithmetic overflow")]
    Overflow,
    #[error("division by zero")]
    DivisionByZero,
}

pub type EvalError = Spanned<EvalErrorKind>;
pub type Environment<'src> = HashMap<&'src str, i64>;

pub fn evaluate<'src>(program: &Program<'src>) -> Result<i64, EvalError> {
    let mut env = Environment::new();

    for declaration in &program.declarations {
        let value = eval(&declaration.value, &env)?;
        env.insert(declaration.name.kind, value);
    }

    exec(&program.commands, &mut env)?;
    eval(&program.result, &env)
}

fn exec<'src>(commands: &[Command<'src>], env: &mut Environment<'src>) -> Result<(), EvalError> {
    Ok(for command in commands {
        match &command.kind {
            CommandKind::Assign { name, value } => {
                let value = eval(value, env)?;
                env.insert(name.kind, value);
            },
            CommandKind::If { then, otherwise, condition } => match eval(condition, env)? {
                0 => exec(otherwise, env)?,
                _ => exec(then, env)?,
            },
            CommandKind::While { body, condition } => {
                while eval(condition, env)? != 0 {
                    exec(body, env)?;
                }
            },
        }
    })
}

fn eval(expr: &Expression<'_>, env: &Environment) -> Result<i64, EvalError> {
    match &expr.kind {
        ExpressionKind::Integer(int) => Ok(*int),
        ExpressionKind::Variable(name) => Ok(*env
            .get(name)
            .unwrap_or_else(|| unreachable!("semantic analysis let undeclared `{name}`"))),
        ExpressionKind::Binary { left, operator, right } => {
            let (lhs, rhs) = (eval(left, env)?, eval(right, env)?);

            let result = match operator {
                BinaryOperator::Lt => Some(i64::from(lhs < rhs)),
                BinaryOperator::Gt => Some(i64::from(lhs > rhs)),
                BinaryOperator::Eq => Some(i64::from(lhs == rhs)),

                BinaryOperator::Add => lhs.checked_add(rhs),
                BinaryOperator::Sub => lhs.checked_sub(rhs),
                BinaryOperator::Mul => lhs.checked_mul(rhs),
                BinaryOperator::Div if rhs == 0 => {
                    return Err(Spanned::new(EvalErrorKind::DivisionByZero, right.span));
                },
                BinaryOperator::Div => lhs.checked_div(rhs),
            };

            result.ok_or(Spanned::new(EvalErrorKind::Overflow, expr.span))
        },
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evaluation error at {}: {}", self.span, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse;
    use crate::semantic::analyse;

    fn run(src: &str) -> Result<i64, EvalError> {
        let program = parse(src).unwrap();
        let _ = analyse(&program).unwrap();
        evaluate(&program)
    }

    #[test]
    fn evaluate_example() {
        assert_eq!(run("{ return ((427 / 7) + (11 * (231 + 5))); }"), Ok(2657))
    }
}
