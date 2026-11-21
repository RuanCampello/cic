//! Code (assembly) generator

use crate::{
    frontend::parser::{BinaryOperator, Command, CommandKind, Expression, ExpressionKind, Program},
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
        "modelo.s must contain the variables marker"
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
    let mut labels = 0;
    emit_commands(&program.commands, symbols, &mut labels, &mut code);
    emit(&program.result, symbols, &mut code);

    TEMPLATE.replace(VARIABLES_MARKER, &variables).replace(CODE_MARKER, &code)
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
                BinaryOperator::Eq => compare(out, "setz"),
                BinaryOperator::Gt => compare(out, "setg"),
                BinaryOperator::Lt => compare(out, "setl"),
                BinaryOperator::Div => {
                    inst!(out, "cqo");
                    inst!(out, "idiv %rbx")
                },
            }
        },
    }
}

fn emit_commands(
    commands: &[Command<'_>],
    symbols: &SymbolTable<'_>,
    labels: &mut usize,
    out: &mut String,
) {
    for command in commands {
        match &command.kind {
            CommandKind::Assign { name, value } => {
                emit(value, symbols, out);
                inst!(out, "mov %rax, {}", Symbol(name.kind));
            },
            CommandKind::If { condition, then, otherwise } => {
                let id = *labels;
                *labels += 1;

                emit(condition, symbols, out);
                inst!(out, "cmp $0, %rax");
                inst!(out, "jz Lfalso{id}");
                emit_commands(then, symbols, labels, out);
                inst!(out, "jmp Lfim{id}");
                writeln!(out, "Lfalso{id}:").unwrap();
                emit_commands(otherwise, symbols, labels, out);
                writeln!(out, "Lfim{id}:").unwrap();
            },
            CommandKind::While { condition, body } => {
                let id = *labels;
                *labels += 1;

                writeln!(out, "Linicio{id}:").unwrap();
                emit(condition, symbols, out);
                inst!(out, "cmp $0, %rax");
                inst!(out, "jz Lfim{id}");
                emit_commands(body, symbols, labels, out);
                inst!(out, "jmp Linicio{id}");
                writeln!(out, "Lfim{id}:").unwrap();
            },
        }
    }
}

/// rbx holds the left operand and rax the right one
/// the set* instruction only takes 8 bits, so the flag goes through rcx :X
fn compare(out: &mut String, set: &str) {
    inst!(out, "xor %rcx, %rcx");
    inst!(out, "cmp %rbx, %rax");
    inst!(out, "{set} %cl");
    inst!(out, "mov %rcx, %rax");
}

impl std::fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var_{}", self.0)
    }
}
