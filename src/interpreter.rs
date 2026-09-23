use crate::frontend::{
    lexer::Spanned,
    parser::{BinaryOperator, Expression, ExpressionKind},
};

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse;

    #[test]
    fn evaluate_example() {
        let example = "((427 / 7) + (11 * (231 + 5)))";
        assert_eq!(evaluate(&parse(example).unwrap()), Ok(2657));
    }
}
