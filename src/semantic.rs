//! Semantic (contextual) analysis over the syntax tree

use crate::frontend::{
    lexer::Spanned,
    parser::{Expression, ExpressionKind, Program},
};
use std::collections::BTreeSet;

#[derive(Default)]
pub struct SymbolTable<'src> {
    variables: BTreeSet<&'src str>,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SemanticErrorKind<'src> {
    #[error("use of undeclared variable `{0}`")]
    UndeclaredVariable(&'src str),
}

pub type SemanticError<'src> = Spanned<SemanticErrorKind<'src>>;

pub fn analyse<'src>(program: &Program<'src>) -> Result<SymbolTable<'src>, SemanticError<'src>> {
    let mut symbols = SymbolTable::default();

    for declaration in &program.declarations {
        symbols.check(&declaration.value)?;
        symbols.variables.insert(declaration.name.kind);
    }

    symbols.check(&program.result)?;
    Ok(symbols)
}

impl<'src> SymbolTable<'src> {
    fn check(&self, expr: &Expression<'src>) -> Result<(), SemanticError<'src>> {
        match &expr.kind {
            ExpressionKind::Integer(_) => Ok(()),
            ExpressionKind::Variable(name) => match self.contains(name) {
                true => Ok(()),
                _ => Err(Spanned::new(SemanticErrorKind::UndeclaredVariable(name), expr.span)),
            },
            ExpressionKind::Binary { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            },
        }
    }

    #[inline(always)]
    pub fn variables(&self) -> impl Iterator<Item = &'src str> {
        self.variables.iter().copied()
    }

    #[inline(always)]
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains(name)
    }
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
        analyse(&program).map(|symbols| symbols.variables().collect())
    }

    #[test]
    fn declared_variables_are_accepted() {
        assert_eq!(analyse_src("x = 7 * 8;\ny = x * 11 - 17;\n= x * y + 33"), Ok(vec!["x", "y"]));
    }

    #[test]
    fn use_before_declaration_is_rejected() {
        let error = analyse_src("x = 7 + y;\ny = x * 11;\n= x * y + z").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("y"));
        assert_eq!(error.span, Span::new(8, 9));
    }

    #[test]
    fn undeclared_variable_in_result_is_rejected() {
        let error = analyse_src("x = 1;\n= x + z").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("z"));
    }

    #[test]
    fn self_reference_is_rejected() {
        let error = analyse_src("x = x + 1;\n= x").unwrap_err();
        assert_eq!(error.kind, SemanticErrorKind::UndeclaredVariable("x"));
    }
}
