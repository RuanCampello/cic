//! Code (assembly) generator

use crate::frontend::parser::{BinaryOperator, Expression, ExpressionKind};
use std::fmt::Write;

const TEMPLATE: &str = include_str!("../asm/modelo.s");
const MARKER: &str = "  ## saida do compilador deve ser inserida aqui\n";

/// how much indentation we will have for asm instruction :D
const INDENT: usize = 2;

/// generates assembly string from a given [expr](Expression)
pub fn generate(expr: &Expression<'_>) -> String {
    assert!(TEMPLATE.contains(MARKER), "modelo.s must contains the insertion marker");

    let mut code = String::new();
    emit(expr, &mut code);
    TEMPLATE.replace(MARKER, &code)
}

macro_rules! inst {
    ($out:expr, $($args:tt)*) => {
        writeln!($out, "{:INDENT$}{}", "", format_args!($($args)*)).unwrap()
    };
}

fn emit(expr: &Expression<'_>, out: &mut String) {
    match &expr.kind {
        ExpressionKind::Integer(int) => inst!(out, "mov ${int}, %rax"),
        ExpressionKind::Binary { left, operator, right } => {
            emit(&right, out);
            inst!(out, "push %rax");
            emit(&left, out);
            inst!(out, "pop %rbx");

            match operator {
                BinaryOperator::Add => inst!(out, "add %rbx, %rax"),
                BinaryOperator::Sub => inst!(out, "sub %rbx, %rax"),
                BinaryOperator::Mul => inst!(out, "imul %rbx, %rax"),
                BinaryOperator::Div => {
                    inst!(out, "cqo");
                    inst!(out, "idiv %rbx")
                },
            }
        },
        ExpressionKind::_M(_) => unreachable!(),
    }
}
