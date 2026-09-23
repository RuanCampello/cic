use crate::lexer::{LexError, LexErrorKind, Lexer, Punct, Span, Spanned, Token, TokenKind};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// the next token we have from the current [lexer](Self::lexer)
    lookahead: Token<'src>,
}

pub type Expression<'i> = Spanned<ExpressionKind<'i>>;
pub type ParseError<'src> = Spanned<ParseErrorKind<'src>>;

#[derive(Debug, PartialEq)]
pub enum ExpressionKind<'i> {
    Integer(i64),
    Binary {
        left: Box<Expression<'i>>,
        operator: BinaryOperator,
        right: Box<Expression<'i>>,
    },
    /// same reason as [crate::lexer::TokenKind::_M]
    #[doc(hidden)]
    _M(std::marker::PhantomData<&'i ()>),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq)]
pub enum ParseErrorKind<'src> {
    Lex(LexErrorKind<'src>),
    Expected { found: TokenKind<'src>, expected: TokenKind<'src> },
    ExpectedExpression { found: TokenKind<'src> },
    ExpectedOperator { found: TokenKind<'src> },
}

pub trait Parsable<'src>: Sized {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>>;
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Result<Self, ParseError<'src>> {
        let mut lexer = Lexer::new(src);
        let lookahead = lexer.next_token()?;
        Ok(Self { lexer, lookahead })
    }

    pub fn parse_node<T: Parsable<'src>>(&mut self) -> Result<T, ParseError<'src>> {
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

    #[inline(always)]
    fn error(&self, kind: ParseErrorKind<'src>) -> ParseError<'src> {
        Spanned::new(kind, self.lookahead.span)
    }
}

impl<'src> Parsable<'src> for Expression<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        let token = parser.peek();

        match token.kind {
            TokenKind::Integer(int) => {
                parser.advance()?;
                Ok(Spanned::new(ExpressionKind::Integer(int), token.span))
            },
            TokenKind::Punct(Punct::OpenParen) => {
                parser.advance()?;

                let left = parser.parse_node::<Expression>()?;
                let operator = parser.parse_node::<BinaryOperator>()?;
                let right = parser.parse_node::<Expression>()?;

                let close = parser.expect_punct(Punct::CloseParen)?;

                let kind = ExpressionKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                };

                Ok(Spanned::new(kind, token.span.merge(close)))
            },
            found => Err(parser.error(ParseErrorKind::ExpectedExpression { found })),
        }
    }
}

impl<'src> Parsable<'src> for BinaryOperator {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        let operator = match parser.peek().kind {
            TokenKind::Punct(Punct::Plus) => BinaryOperator::Add,
            TokenKind::Punct(Punct::Minus) => BinaryOperator::Sub,
            TokenKind::Punct(Punct::Star) => BinaryOperator::Mul,
            TokenKind::Punct(Punct::Slash) => BinaryOperator::Div,
            found => return Err(parser.error(ParseErrorKind::ExpectedOperator { found })),
        };

        parser.advance()?;
        Ok(operator)
    }
}

impl<'src> From<LexError<'src>> for ParseError<'src> {
    fn from(error: LexError<'src>) -> Self {
        error.map(ParseErrorKind::Lex)
    }
}

impl std::fmt::Display for ParseErrorKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(lex) => write!(f, "{lex}"),
            Self::Expected { found, expected } => {
                write!(f, "expected {expected} but found {found}")
            },
            Self::ExpectedExpression { found } => {
                write!(f, "expected a number or '(', but found {found}")
            },
            Self::ExpectedOperator { found } => {
                write!(f, "expected an operator, but found {found}")
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Result<Expression<'_>, ParseError<'_>> {
        let mut parser = Parser::new(src)?;
        let expr = parser.parse_node::<Expression<'_>>()?;
        parser.expect_eof()?;

        Ok(expr)
    }

    #[test]
    fn specification_example_structure() {
        let expr = parse("(33 + (912 * 11))").unwrap();
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
}
