//! Code (assembly) generator

use crate::{
    frontend::parser::{
        BinaryOperator, Body, Command, CommandKind, Expression, ExpressionKind, Function, Item,
        Program,
    },
    semantic::{Scope, SymbolTable},
};
use std::fmt::Write;

struct Global<'n>(&'n str);
struct Label<'n>(&'n str);

struct Generator<'t, 'src> {
    symbols: &'t SymbolTable<'src>,
    /// the function being generated, `None` inside `main`
    scope: Option<Scope<'t, 'src>>,
    labels: usize,
    out: String,
}

pub const RUNTIME: &str = include_str!("../asm/runtime.s");
const TEMPLATE: &str = include_str!("../asm/modelo.s");
const VARIABLES_MARKER: &str = "  ## variaveis devem ser declaradas aqui\n";
const CODE_MARKER: &str = "  ## saida do compilador deve ser inserida aqui\n";
const FUNCTIONS_MARKER: &str = "  ## funcoes devem ser inseridas aqui\n";

/// how much indentation we will have for asm instruction :D
const INDENT: usize = 2;

macro_rules! inst {
    ($out:expr, $($args:tt)*) => {
        writeln!($out, "{:INDENT$}{}", "", format_args!($($args)*)).unwrap()
    };
}

/// generates assembly string from a given [program](Program) that passed the semantic analysis
pub fn generate(program: &Program<'_>, symbols: &SymbolTable<'_>) -> String {
    for marker in [VARIABLES_MARKER, CODE_MARKER, FUNCTIONS_MARKER] {
        assert!(TEMPLATE.contains(marker), "modelo.s must contain the marker {marker:?}");
    }

    let mut variables = String::new();
    for name in symbols.globals() {
        inst!(variables, ".lcomm {}, 8", Global(name));
    }

    let mut generator = Generator { symbols, scope: None, labels: 0, out: String::new() };
    for item in &program.items {
        if let Item::Var(var) = item {
            generator.expression(&var.value);
            generator.store(var.name.kind);
        }
    }
    generator.body(&program.main);
    let code = std::mem::take(&mut generator.out);

    for item in &program.items {
        if let Item::Fn(function) = item {
            generator.function(function);
        }
    }

    TEMPLATE
        .replace(VARIABLES_MARKER, &variables)
        .replace(CODE_MARKER, &code)
        .replace(FUNCTIONS_MARKER, &generator.out)
}

impl<'t, 'src> Generator<'t, 'src> {
    /// `push rbp; sub locals, rsp; mov rsp, rbp`, the caller pushes the arguments
    /// last to first and pops them after the `call`
    fn function(&mut self, function: &Function<'src>) {
        let scope = self.symbols.scope(function.name.kind);
        let size = scope.frame().size();
        self.scope = Some(scope);

        writeln!(self.out, "{}:", Label(function.name.kind)).unwrap();
        inst!(self.out, "push %rbp");
        inst!(self.out, "sub ${size}, %rsp");
        inst!(self.out, "mov %rsp, %rbp");

        for local in &function.locals {
            self.expression(&local.value);
            self.scope.as_mut().expect("a function scope").visible += 1;
            self.store(local.name.kind);
        }
        self.body(&function.body);

        inst!(self.out, "add ${size}, %rsp");
        inst!(self.out, "pop %rbp");
        inst!(self.out, "ret");
        self.scope = None;
    }

    fn body(&mut self, body: &Body<'src>) {
        self.commands(&body.commands);
        self.expression(&body.result);
    }

    fn expression(&mut self, expr: &Expression<'src>) {
        match &expr.kind {
            ExpressionKind::Integer(int) => inst!(self.out, "mov ${int}, %rax"),
            ExpressionKind::Variable(name) => match self.offset(name) {
                Some(offset) => inst!(self.out, "mov {offset}(%rbp), %rax"),
                None => inst!(self.out, "mov {}, %rax", Global(name)),
            },
            ExpressionKind::Call { name, args } => {
                for arg in args.iter().rev() {
                    self.expression(arg);
                    inst!(self.out, "push %rax");
                }
                inst!(self.out, "call {}", Label(name.kind));
                if !args.is_empty() {
                    inst!(self.out, "add ${}, %rsp", 8 * args.len());
                }
            },
            ExpressionKind::Binary { left, operator, right } => {
                self.expression(right);
                inst!(self.out, "push %rax");
                self.expression(left);
                inst!(self.out, "pop %rbx");

                match operator {
                    BinaryOperator::Add => inst!(self.out, "add %rbx, %rax"),
                    BinaryOperator::Sub => inst!(self.out, "sub %rbx, %rax"),
                    BinaryOperator::Mul => inst!(self.out, "imul %rbx, %rax"),
                    BinaryOperator::Eq => compare(&mut self.out, "setz"),
                    BinaryOperator::Gt => compare(&mut self.out, "setg"),
                    BinaryOperator::Lt => compare(&mut self.out, "setl"),
                    BinaryOperator::Div => {
                        inst!(self.out, "cqo");
                        inst!(self.out, "idiv %rbx")
                    },
                }
            },
        }
    }

    fn commands(&mut self, commands: &[Command<'src>]) {
        for command in commands {
            match &command.kind {
                CommandKind::Assign { name, value } => {
                    self.expression(value);
                    self.store(name.kind);
                },
                CommandKind::If { condition, then, otherwise } => {
                    let id = self.label();

                    self.expression(condition);
                    inst!(self.out, "cmp $0, %rax");
                    inst!(self.out, "jz Lfalso{id}");
                    self.commands(then);
                    inst!(self.out, "jmp Lfim{id}");
                    writeln!(self.out, "Lfalso{id}:").unwrap();
                    self.commands(otherwise);
                    writeln!(self.out, "Lfim{id}:").unwrap();
                },
                CommandKind::While { condition, body } => {
                    let id = self.label();

                    writeln!(self.out, "Linicio{id}:").unwrap();
                    self.expression(condition);
                    inst!(self.out, "cmp $0, %rax");
                    inst!(self.out, "jz Lfim{id}");
                    self.commands(body);
                    inst!(self.out, "jmp Linicio{id}");
                    writeln!(self.out, "Lfim{id}:").unwrap();
                },
            }
        }
    }

    /// stores `rax` in the local, parameter or global named `name`
    fn store(&mut self, name: &str) {
        match self.offset(name) {
            Some(offset) => inst!(self.out, "mov %rax, {offset}(%rbp)"),
            None => inst!(self.out, "mov %rax, {}", Global(name)),
        }
    }

    #[inline]
    fn offset(&self, name: &str) -> Option<usize> {
        self.scope.and_then(|scope| scope.offset(name))
    }

    #[inline]
    fn label(&mut self) -> usize {
        self.labels += 1;
        self.labels - 1
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

impl std::fmt::Display for Global<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var_{}", self.0)
    }
}

/// prefixed so user functions can't clash with `_start` or the runtime labels
impl std::fmt::Display for Label<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fun_{}", self.0)
    }
}
