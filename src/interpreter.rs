use crate::{
    lexer::Spanned,
    parser::{BinaryOperator, Expression, ExpressionKind},
};

pub enum EvalErrorKind {
    Overflow,
    DivisionByZero,
}

pub type EvalError = Spanned<EvalErrorKind>;

pub fn evaluate(expr: &Expression<'_>) -> Result<i64, EvalError> {
    match &expr.kind {
        ExpressionKind::Integer(int) => Ok(*int),
        ExpressionKind::Binary { left, operator, right } => {
            let (lhs, rhs) = (evaluate(left)?, evaluate(right)?);

            let result = match operator {
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
        _ => unreachable!(),
    }
}
