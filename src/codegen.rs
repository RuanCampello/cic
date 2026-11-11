//! Code (assembly) generator

use crate::{
    frontend::parser::{BinaryOperator, Expression, ExpressionKind, Program},
    semantic::SymbolTable,
};
use std::fmt::Write;

struct Symbol<'n>(&'n str);

pub const RUNTIME: &str = include_str!("../asm/runtime.s");
const TEMPLATE: &str = include_str!("../asm/modelo.s");
const VARIABLES_MARKER: &str = "  ## variaveis devem ser declaradas aqui\n";
const CODE_MARKER: &str = "  ## saida do compilador deve ser inserida aqui\n";

/// how much indentation we will have for asm instruction :D
const INDENT: usize = 2;

macro_rules! inst {
    ($out:expr, $($args:tt)*) => {
        writeln!($out, "{:INDENT$}{}", "", format_args!($($args)*)).unwrap()
    };
}

/// generates assembly string from a given [program](Program) that passed the semantic analysis
pub fn generate(program: &Program<'_>, symbols: &SymbolTable<'_>) -> String {
    assert!(
        TEMPLATE.contains(VARIABLES_MARKER),
        "modelo.s must contains the insertion marker"
    );
    assert!(TEMPLATE.contains(CODE_MARKER), "modelo.s must contain the code marker");

    let mut variables = String::new();
    for name in symbols.variables() {
        inst!(variables, ".lcomm {}, 8", Symbol(name));
    }

    let mut code = String::new();
    for declaration in &program.declarations {
        emit(&declaration.value, symbols, &mut code);
        inst!(code, "mov %rax, {}", Symbol(declaration.name.kind));
    }
    TEMPLATE.replace(VARIABLES_MARKER, &code)
}

fn emit(expr: &Expression<'_>, symbols: &SymbolTable<'_>, out: &mut String) {
    match &expr.kind {
        ExpressionKind::Integer(int) => inst!(out, "mov ${int}, %rax"),
        ExpressionKind::Variable(name) => {
            assert!(symbols.contains(name), "semantic analysis let undeclared `{name}`");
            inst!(out, "mov {}, %rax", Symbol(name))
        },
        ExpressionKind::Binary { left, operator, right } => {
            emit(&right, symbols, out);
            inst!(out, "push %rax");
            emit(&left, symbols, out);
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
    }
}

impl std::fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var_{}", self.0)
    }
}
