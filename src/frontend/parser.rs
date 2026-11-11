use crate::frontend::lexer::{
    LexError, LexErrorKind, Lexer, Punct, Span, Spanned, Token, TokenKind,
};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// the next token we have from the current [lexer](Self::lexer)
    lookahead: Token<'src>,
}

#[derive(Debug, PartialEq)]
pub struct Program<'i> {
    pub declarations: Vec<Declaration<'i>>,
    pub result: Expression<'i>,
}

#[derive(Debug, PartialEq)]
pub struct Declaration<'i> {
    pub name: Spanned<&'i str>,
    pub value: Expression<'i>,
}

pub type Expression<'i> = Spanned<ExpressionKind<'i>>;
pub type ParseError<'src> = Spanned<ParseErrorKind<'src>>;

#[derive(Debug, PartialEq)]
pub enum ExpressionKind<'i> {
    Variable(&'i str),
    Integer(i64),
    Binary { left: Box<Expression<'i>>, operator: BinaryOperator, right: Box<Expression<'i>> },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParseErrorKind<'src> {
    #[error(transparent)]
    Lex(LexErrorKind<'src>),
    #[error("expected {expected} but found {found}")]
    Expected { found: TokenKind<'src>, expected: TokenKind<'src> },
    #[error("expected a number or '(' but found {found}")]
    ExpectedExpression { found: TokenKind<'src> },
    #[error("expected an operator but found {found}")]
    ExpectedOperator { found: TokenKind<'src> },
}

pub trait Parsable<'src>: Sized {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>>;
}

pub fn parse(src: &str) -> Result<Program<'_>, ParseError<'_>> {
    let mut parser = Parser::new(src)?;
    let expr = parser.parse_node()?;
    parser.expect_eof()?;

    Ok(expr)
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Result<Self, ParseError<'src>> {
        let mut lexer = Lexer::new(src);
        let lookahead = lexer.next_token()?;
        Ok(Self { lexer, lookahead })
    }

    fn parse_node<T: Parsable<'src>>(&mut self) -> Result<T, ParseError<'src>> {
        T::parse(self)
    }

    #[inline(always)]
    fn peek(&self) -> Token<'src> {
        self.lookahead
    }

    #[inline(always)]
    fn advance(&mut self) -> Result<(), ParseError<'src>> {
        Ok(self.lookahead = self.lexer.next_token()?)
    }

    fn expect_punct(&mut self, punct: Punct) -> Result<Span, ParseError<'src>> {
        match self.lookahead.kind {
            TokenKind::Punct(found) if found == punct => {
                let span = self.lookahead.span;
                self.advance()?;
                Ok(span)
            },
            found => Err(self.error(ParseErrorKind::Expected { found, expected: punct.into() })),
        }
    }

    fn expect_eof(&mut self) -> Result<(), ParseError<'src>> {
        match self.lookahead.kind {
            TokenKind::Eof => Ok(()),
            found => Err(self.error(ParseErrorKind::Expected { found, expected: TokenKind::Eof })),
        }
    }

    fn binary(&mut self, min_precedence: u8) -> Result<Expression<'src>, ParseError<'src>> {
        let mut left = self.primary()?;

        while let Some(operator) = BinaryOperator::from_token(self.peek().kind)
            && operator.precedence() >= min_precedence
        {
            self.advance()?;
            // `+ 1` makes operators of the same level left associative
            let right = self.binary(operator.precedence() + 1)?;
            let span = left.span.merge(right.span);
            let kind =
                ExpressionKind::Binary { left: Box::new(left), operator, right: Box::new(right) };

            left = Spanned::new(kind, span);
        }

        Ok(left)
    }

    fn primary(&mut self) -> Result<Expression<'src>, ParseError<'src>> {
        let token = self.peek();

        match token.kind {
            TokenKind::Integer(int) => {
                self.advance()?;
                Ok(Spanned::new(ExpressionKind::Integer(int), token.span))
            },
            TokenKind::Ident(ident) => {
                self.advance()?;
                Ok(Spanned::new(ExpressionKind::Variable(ident), token.span))
            },
            TokenKind::Punct(Punct::OpenParen) => {
                self.advance()?;
                let inner = self.parse_node::<Expression>()?;
                let close = self.expect_punct(Punct::CloseParen)?;

                Ok(Spanned::new(inner.kind, token.span.merge(close)))
            },
            found => Err(self.error(ParseErrorKind::ExpectedExpression { found })),
        }
    }

    #[inline(always)]
    fn error(&self, kind: ParseErrorKind<'src>) -> ParseError<'src> {
        Spanned::new(kind, self.lookahead.span)
    }
}

impl<'src> Parsable<'src> for Program<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        let mut declarations = Vec::new();

        while let TokenKind::Ident(_) = parser.peek().kind {
            declarations.push(parser.parse_node()?);
        }

        parser.expect_punct(Punct::Equal)?;
        let result = parser.parse_node()?;

        Ok(Self { declarations, result })
    }
}

impl<'src> Parsable<'src> for Expression<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        parser.binary(BinaryOperator::LOWEST_PRECEDENCE)
    }
}

impl<'src> Parsable<'src> for Declaration<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        let token = parser.peek();
        let TokenKind::Ident(ident) = token.kind else {
            unreachable!("declarations are only parsed when the lookahead is an identifier")
        };
        parser.advance()?;

        parser.expect_punct(Punct::Equal)?;
        let value = parser.parse_node()?;
        parser.expect_punct(Punct::Semicolon)?;

        Ok(Self { name: Spanned::new(ident, token.span), value })
    }
}

impl<'src> Parser<'src> {}

impl BinaryOperator {
    const LOWEST_PRECEDENCE: u8 = 1;

    fn from_token(kind: TokenKind<'_>) -> Option<Self> {
        match kind {
            TokenKind::Punct(Punct::Plus) => Some(Self::Add),
            TokenKind::Punct(Punct::Minus) => Some(Self::Sub),
            TokenKind::Punct(Punct::Star) => Some(Self::Mul),
            TokenKind::Punct(Punct::Slash) => Some(Self::Div),
            _ => None,
        }
    }

    const fn precedence(self) -> u8 {
        match self {
            Self::Add | Self::Sub => 1,
            Self::Mul | Self::Div => 2,
        }
    }
}

impl<'src> From<LexError<'src>> for ParseError<'src> {
    fn from(error: LexError<'src>) -> Self {
        error.map(ParseErrorKind::Lex)
    }
}

impl From<BinaryOperator> for char {
    fn from(value: BinaryOperator) -> Self {
        match value {
            BinaryOperator::Add => '+',
            BinaryOperator::Sub => '-',
            BinaryOperator::Mul => '*',
            BinaryOperator::Div => '/',
        }
    }
}

impl std::fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parsing error at {}: {}", self.span, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(src: &str) -> Expression<'_> {
        let program = parse(src).unwrap();
        assert!(program.declarations.is_empty());
        program.result
    }

    #[test]
    fn specification_example_structure() {
        let expr = result("(33 + (912 * 11))");
        assert_eq!(expr.span, Span::new(0, 17));

        let ExpressionKind::Binary { right, left, operator } = &expr.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*operator, BinaryOperator::Add);
        assert_eq!(left.kind, ExpressionKind::Integer(33));
        assert_eq!(right.span, Span::new(6, 16));

        let ExpressionKind::Binary { operator, left, right } = &right.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*operator, BinaryOperator::Mul);
        assert_eq!(left.kind, ExpressionKind::Integer(912));
        assert_eq!(right.kind, ExpressionKind::Integer(11));
    }

    #[test]
    fn unclosed_paren_is_rejected() {
        let error = parse("(1 + 2").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: TokenKind::Eof, expected: Punct::CloseParen.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(6, 6));
    }

    #[test]
    fn missing_operator_is_rejected() {
        let error = parse("(1 2)").unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::ExpectedOperator { found: TokenKind::Integer(2) });
        assert_eq!(error.span, Span::new(3, 4));
    }

    #[test]
    fn trailing_tokens_are_rejected() {
        let error = parse("1 2").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: TokenKind::Integer(2), expected: TokenKind::Eof };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(2, 3));
    }

    #[test]
    fn empty_parens_are_rejected() {
        let error = parse("()").unwrap_err();
        let expected = ParseErrorKind::ExpectedExpression { found: Punct::CloseParen.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(1, 2));
    }

    #[test]
    fn empty_input_is_rejected() {
        let error = parse("").unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::ExpectedExpression { found: TokenKind::Eof });
        assert_eq!(error.span, Span::new(0, 0));
    }

    #[test]
    fn lexical_error_surfaces_through_parser() {
        let error = parse("(1 + a)").unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::Lex(LexErrorKind::UnexpectedChar('a')));
        assert_eq!(error.span, Span::new(5, 6));
    }
}
