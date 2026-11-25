//! Semantic (contextual) analysis over the syntax tree

use crate::frontend::{
    lexer::Spanned,
    parser::{
        Body, Command, CommandKind, Expression, ExpressionKind, Function, Item, Program, Var,
    },
};
use std::collections::BTreeMap;

/// The global namespace: variables and functions share it
#[derive(Default)]
pub struct SymbolTable<'src> {
    symbols: BTreeMap<&'src str, Symbol<'src>>,
}

/// Parameters and locals of a function, in the order of its activation record
#[derive(Debug, PartialEq)]
pub struct Frame<'src> {
    params: Vec<&'src str>,
    locals: Vec<&'src str>,
}

/// Where names resolve inside a function body: only the first `visible` locals
/// were declared so far, the rest are still out of scope
#[derive(Clone, Copy)]
pub struct Scope<'f, 'src> {
    frame: &'f Frame<'src>,
    pub visible: usize,
}

struct Checker<'t, 'src> {
    symbols: &'t SymbolTable<'src>,
    scope: Option<Scope<'t, 'src>>,
}

#[derive(Debug, PartialEq)]
pub enum Symbol<'src> {
    Global,
    Function(Frame<'src>),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SemanticErrorKind<'src> {
    #[error("use of undeclared variable `{0}`")]
    UndeclaredVariable(&'src str),
    #[error("call to undeclared function `{0}`")]
    UndeclaredFunction(&'src str),
    #[error("`{0}` is a variable, not a function")]
    NotAFunction(&'src str),
    #[error("`{0}` is a function, not a variable")]
    NotAVariable(&'src str),
    #[error("`{name}` takes {expected} argument(s) but {found} were given")]
    ArityMismatch { name: &'src str, expected: usize, found: usize },
    #[error("`{0}` is already declared")]
    Redeclared(&'src str),
}

pub type SemanticError<'src> = Spanned<SemanticErrorKind<'src>>;

pub fn analyse<'src>(program: &Program<'src>) -> Result<SymbolTable<'src>, SemanticError<'src>> {
    let mut symbols = SymbolTable::default();

    for item in &program.items {
        match item {
            Item::Var(var) => symbols.global(var)?,
            Item::Fn(function) => symbols.function(function)?,
        }
    }

    Checker { symbols: &symbols, scope: None }.body(&program.main)?;
    Ok(symbols)
}

impl<'src> SymbolTable<'src> {
    fn global(&mut self, var: &Var<'src>) -> Result<(), SemanticError<'src>> {
        Checker { symbols: self, scope: None }.check(&var.value)?;

        if let Some(Symbol::Function(_)) = self.symbols.get(var.name.kind) {
            return Err(redeclared(var.name));
        }
        self.symbols.insert(var.name.kind, Symbol::Global);
        Ok(())
    }

    /// the function is registered before its body is checked, so it can call itself
    fn function(&mut self, function: &Function<'src>) -> Result<(), SemanticError<'src>> {
        let name = function.name;
        if self.symbols.contains_key(name.kind) {
            return Err(redeclared(name));
        }

        let mut frame = Frame { params: Vec::new(), locals: Vec::new() };
        for param in &function.params {
            frame.declare(*param, |frame| &mut frame.params)?;
        }
        for local in &function.locals {
            frame.declare(local.name, |frame| &mut frame.locals)?;
        }
        self.symbols.insert(name.kind, Symbol::Function(frame));

        let mut checker = Checker { symbols: self, scope: Some(self.scope(name.kind)) };
        for local in &function.locals {
            checker.check(&local.value)?;
            checker.scope.as_mut().expect("a function scope").visible += 1;
        }
        checker.body(&function.body)
    }

    /// variables are 64-bit integers placed in the `.bss` section
    #[inline]
    pub fn globals(&self) -> impl Iterator<Item = &'src str> {
        self.symbols
            .iter()
            .filter(|(_, symbol)| **symbol == Symbol::Global)
            .map(|(name, _)| *name)
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<&Symbol<'src>> {
        self.symbols.get(name)
    }

    /// the scope of `function` with none of its locals visible yet
    pub fn scope(&self, function: &str) -> Scope<'_, 'src> {
        match self.symbols.get(function) {
            Some(Symbol::Function(frame)) => Scope { frame, visible: 0 },
            _ => panic!("semantic analysis let unknown function `{function}` through"),
        }
    }
}

impl<'src> Frame<'src> {
    fn declare(
        &mut self,
        name: Spanned<&'src str>,
        slots: impl FnOnce(&mut Self) -> &mut Vec<&'src str>,
    ) -> Result<(), SemanticError<'src>> {
        if self.params.contains(&name.kind) || self.locals.contains(&name.kind) {
            return Err(redeclared(name));
        }
        Ok(slots(self).push(name.kind))
    }

    #[inline]
    pub fn arity(&self) -> usize {
        self.params.len()
    }

    /// bytes reserved below the saved `rbp` for the locals
    #[inline]
    pub fn size(&self) -> usize {
        8 * self.locals.len()
    }
}

impl<'f, 'src> Scope<'f, 'src> {
    #[inline]
    pub const fn frame(&self) -> &'f Frame<'src> {
        self.frame
    }

    /// `rbp`-relative offset of a parameter or visible local, laid out as
    /// `locals | saved rbp | return address | params`
    pub fn offset(&self, name: &str) -> Option<usize> {
        let frame = self.frame;
        let local = frame.locals[..self.visible].iter().position(|local| *local == name);
        let param = || frame.params.iter().position(|param| *param == name);

        local.map(|i| 8 * i).or_else(|| param().map(|i| frame.size() + 16 + 8 * i))
    }
}

impl<'t, 'src> Checker<'t, 'src> {
    fn check(&self, expr: &Expression<'src>) -> Result<(), SemanticError<'src>> {
        match &expr.kind {
            ExpressionKind::Integer(_) => Ok(()),
            ExpressionKind::Variable(name) => self.variable(Spanned::new(name, expr.span)),
            ExpressionKind::Binary { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            },
            ExpressionKind::Call { name, args } => {
                let expected = match self.symbols.get(name.kind) {
                    Some(Symbol::Function(frame)) => frame.arity(),
                    Some(Symbol::Global) => {
                        return Err(Spanned::new(
                            SemanticErrorKind::NotAFunction(name.kind),
                            name.span,
                        ));
                    },
                    None => {
                        let kind = SemanticErrorKind::UndeclaredFunction(name.kind);
                        return Err(Spanned::new(kind, name.span));
                    },
                };

                if args.len() != expected {
                    let kind = SemanticErrorKind::ArityMismatch {
                        name: name.kind,
                        expected,
                        found: args.len(),
                    };
                    return Err(Spanned::new(kind, expr.span));
                }

                args.iter().try_for_each(|arg| self.check(arg))
            },
        }
    }

    fn body(&self, body: &Body<'src>) -> Result<(), SemanticError<'src>> {
        self.commands(&body.commands)?;
        self.check(&body.result)
    }

    fn commands(&self, commands: &[Command<'src>]) -> Result<(), SemanticError<'src>> {
        Ok(for command in commands {
            match &command.kind {
                CommandKind::Assign { name, value } => {
                    self.variable(*name)?;
                    self.check(value)?;
                },
                CommandKind::If { condition, then, otherwise } => {
                    self.check(condition)?;
                    self.commands(then)?;
                    self.commands(otherwise)?;
                },
                CommandKind::While { condition, body } => {
                    self.check(condition)?;
                    self.commands(body)?;
                },
            }
        })
    }

    /// locals and parameters shadow globals of the same name
    fn variable(&self, name: Spanned<&'src str>) -> Result<(), SemanticError<'src>> {
        if self.scope.is_some_and(|scope| scope.offset(name.kind).is_some()) {
            return Ok(());
        }

        match self.symbols.get(name.kind) {
            Some(Symbol::Global) => Ok(()),
            Some(Symbol::Function(_)) => {
                Err(Spanned::new(SemanticErrorKind::NotAVariable(name.kind), name.span))
            },
            None => Err(Spanned::new(SemanticErrorKind::UndeclaredVariable(name.kind), name.span)),
        }
    }
}

#[inline]
fn redeclared<'src>(name: Spanned<&'src str>) -> SemanticError<'src> {
    Spanned::new(SemanticErrorKind::Redeclared(name.kind), name.span)
}

impl std::fmt::Display for SemanticError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "semantic error at {}: {}", self.span, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{lexer::Span, parser::parse};

    fn analyse_src(src: &str) -> Result<Vec<&str>, SemanticError<'_>> {
        let program = parse(src).unwrap();
        analyse(&program).map(|symbols| symbols.globals().collect())
    }

    #[test]
    fn declared_variables_are_accepted() {
        assert_eq!(
            analyse_src("var x = 7 * 8;\nvar y = x * 11 - 17;\nmain { return x * y + 33; }"),
            Ok(vec!["x", "y"])
        );
    }

    #[test]
    fn use_before_declaration_is_rejected() {
        let error =
            analyse_src("var x = 7 + y;\nvar y = x * 11;\nmain { return x * y + z; }").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("y"));
        assert_eq!(error.span, Span::new(12, 13));
    }

    #[test]
    fn undeclared_variable_in_result_is_rejected() {
        let error = analyse_src("var x = 1;\nmain { return x + z; }").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("z"));
    }

    #[test]
    fn self_reference_is_rejected() {
        let error = analyse_src("var x = x + 1;\nmain { return x; }").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("x"));
    }

    #[test]
    fn direct_recursion_is_accepted() {
        let src = "fun f(n) { var r = 0; if n > 0 { r = f(n - 1); } else { } return r; }\nmain { return f(3); }";
        assert_eq!(analyse_src(src), Ok(vec![]));
    }

    #[test]
    fn locals_are_visible_only_after_their_declaration() {
        let error = analyse_src("fun f() { var a = b; var b = 1; return a; }\nmain { return 0; }");
        assert_eq!(error.unwrap_err().kind, SemanticErrorKind::UndeclaredVariable("b"));
    }

    #[test]
    fn function_used_as_variable_is_rejected() {
        let error = analyse_src("fun f() { return 1; }\nmain { return f; }").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::NotAVariable("f"));
    }

    #[test]
    fn redeclarations_are_rejected() {
        let cases = [
            "fun f() { return 1; }\nfun f() { return 2; }\nmain { return 0; }",
            "fun f() { return 1; }\nvar f = 1;\nmain { return 0; }",
            "var f = 1;\nfun f() { return 1; }\nmain { return 0; }",
            "fun f(x, x) { return x; }\nmain { return 0; }",
            "fun f(x) { var x = 1; return x; }\nmain { return 0; }",
        ];
        for src in cases {
            let error = analyse_src(src).unwrap_err();
            assert!(matches!(error.kind, SemanticErrorKind::Redeclared(_)), "{src}");
        }
    }

    #[test]
    fn frame_offsets_follow_the_activation_record() {
        let program =
            parse("fun f(x, y) { var a = 1; var b = 2; return a; }\nmain { return 0; }").unwrap();
        let symbols = analyse(&program).unwrap();
        let mut scope = symbols.scope("f");

        assert_eq!(scope.offset("a"), None);
        scope.visible = 2;
        let offsets = ["a", "b", "x", "y"].map(|name| scope.offset(name));
        assert_eq!(offsets, [Some(0), Some(8), Some(32), Some(40)]);
    }
}
