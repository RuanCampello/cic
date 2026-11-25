use std::collections::HashMap;

use thiserror::Error;

use crate::frontend::{
    lexer::Spanned,
    parser::{
        BinaryOperator, Body, Command, CommandKind, Expression, ExpressionKind, Function, Item,
        Program,
    },
};

struct Interpreter<'p, 'src> {
    functions: HashMap<&'src str, &'p Function<'src>>,
    globals: Environment<'src>,
}

#[derive(Debug, PartialEq, Error)]
pub enum EvalErrorKind {
    #[error("arithmetic overflow")]
    Overflow,
    #[error("division by zero")]
    DivisionByZero,
}

pub type EvalError = Spanned<EvalErrorKind>;
pub type Environment<'src> = HashMap<&'src str, i64>;

/// evaluates operands and arguments right to left, the same order as the generated code
pub fn evaluate<'src>(program: &Program<'src>) -> Result<i64, EvalError> {
    let mut interpreter = Interpreter { functions: HashMap::new(), globals: Environment::new() };
    let mut locals = Environment::new();

    for item in &program.items {
        match item {
            Item::Var(var) => {
                let value = interpreter.eval(&var.value, &locals)?;
                interpreter.globals.insert(var.name.kind, value);
            },
            Item::Fn(function) => {
                interpreter.functions.insert(function.name.kind, function);
            },
        }
    }

    interpreter.body(&program.main, &mut locals)
}

impl<'p, 'src> Interpreter<'p, 'src> {
    fn body(
        &mut self,
        body: &Body<'src>,
        locals: &mut Environment<'src>,
    ) -> Result<i64, EvalError> {
        self.exec(&body.commands, locals)?;
        self.eval(&body.result, locals)
    }

    fn exec(
        &mut self,
        commands: &[Command<'src>],
        locals: &mut Environment<'src>,
    ) -> Result<(), EvalError> {
        Ok(for command in commands {
            match &command.kind {
                CommandKind::Assign { name, value } => {
                    let value = self.eval(value, locals)?;
                    match locals.get_mut(name.kind) {
                        Some(slot) => *slot = value,
                        None => {
                            self.globals.insert(name.kind, value);
                        },
                    }
                },
                CommandKind::If { then, otherwise, condition } => {
                    match self.eval(condition, locals)? {
                        0 => self.exec(otherwise, locals)?,
                        _ => self.exec(then, locals)?,
                    }
                },
                CommandKind::While { body, condition } => {
                    while self.eval(condition, locals)? != 0 {
                        self.exec(body, locals)?;
                    }
                },
            }
        })
    }

    fn eval(
        &mut self,
        expr: &Expression<'src>,
        locals: &Environment<'src>,
    ) -> Result<i64, EvalError> {
        match &expr.kind {
            ExpressionKind::Integer(int) => Ok(*int),
            ExpressionKind::Variable(name) => Ok(*locals
                .get(name)
                .or_else(|| self.globals.get(name))
                .unwrap_or_else(|| unreachable!("semantic analysis let undeclared `{name}`"))),
            ExpressionKind::Call { name, args } => {
                let function = self.functions[name.kind];
                let mut frame = Environment::new();

                for (param, arg) in function.params.iter().zip(args).rev() {
                    let value = self.eval(arg, locals)?;
                    frame.insert(param.kind, value);
                }
                for local in &function.locals {
                    let value = self.eval(&local.value, &frame)?;
                    frame.insert(local.name.kind, value);
                }

                self.body(&function.body, &mut frame)
            },
            ExpressionKind::Binary { left, operator, right } => {
                let rhs = self.eval(right, locals)?;
                let lhs = self.eval(left, locals)?;

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
        assert_eq!(run("main { return ((427 / 7) + (11 * (231 + 5))); }"), Ok(2657))
    }
}
