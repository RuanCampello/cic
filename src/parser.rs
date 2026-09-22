use crate::lexer::{LexError, LexErrorKind, Lexer, Spanned, Token, TokenKind};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// the next token we have from the current [lexer](Self::lexer)
    lookahead: Token<'src>,
}

pub type Expression<'i> = Spanned<ExpressionKind<'i>>;
pub type ParseError<'src> = Spanned<ParseErrorKind<'src>>;

pub enum ExpressionKind<'i> {
    Integer(i64),
    Binary {},
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
    UnexpectedToken { find: TokenKind<'src>, expected: TokenKind<'src> },
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
}

impl<'src> From<LexError<'src>> for ParseError<'src> {
    fn from(error: LexError<'src>) -> Self {
        error.map(ParseErrorKind::Lex)
    }
}
