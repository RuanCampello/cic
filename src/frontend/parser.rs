use crate::frontend::lexer::{
    Keyword, LexError, LexErrorKind, Lexer, Punct, Span, Spanned, Token, TokenKind,
};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// the next token we have from the current [lexer](Self::lexer)
    lookahead: Token<'src>,
}

#[derive(Debug, PartialEq)]
pub struct Program<'i> {
    pub items: Vec<Item<'i>>,
    pub main: Body<'i>,
}

/// `'var' <id> '=' <exp> ';'`
#[derive(Debug, PartialEq)]
pub struct Var<'i> {
    pub name: Spanned<&'i str>,
    pub value: Expression<'i>,
}

#[derive(Debug, PartialEq)]
pub struct Function<'i> {
    pub name: Spanned<&'i str>,
    pub params: Vec<Spanned<&'i str>>,
    pub locals: Vec<Var<'i>>,
    pub body: Body<'i>,
}

/// `<cmd>* 'return' <exp> ';'` shared by `main` and every function
#[derive(Debug, PartialEq)]
pub struct Body<'i> {
    pub commands: Vec<Command<'i>>,
    pub result: Expression<'i>,
}

pub type Expression<'i> = Spanned<ExpressionKind<'i>>;
pub type ParseError<'src> = Spanned<ParseErrorKind<'src>>;
pub type Command<'i> = Spanned<CommandKind<'i>>;

#[derive(Debug, PartialEq)]
pub enum Item<'i> {
    Var(Var<'i>),
    Fn(Function<'i>),
}

#[derive(Debug, PartialEq)]
pub enum ExpressionKind<'i> {
    Variable(&'i str),
    Integer(i64),
    Binary { left: Box<Expression<'i>>, operator: BinaryOperator, right: Box<Expression<'i>> },
    Call { name: Spanned<&'i str>, args: Vec<Expression<'i>> },
}

#[derive(Debug, PartialEq)]
pub enum CommandKind<'i> {
    Assign { name: Spanned<&'i str>, value: Expression<'i> },
    If { condition: Expression<'i>, then: Vec<Command<'i>>, otherwise: Vec<Command<'i>> },
    While { condition: Expression<'i>, body: Vec<Command<'i>> },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Eq,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParseErrorKind<'src> {
    #[error(transparent)]
    Lex(LexErrorKind<'src>),
    #[error("expected {expected} but found {found}")]
    Expected { found: TokenKind<'src>, expected: TokenKind<'src> },
    #[error("expected a number, identifier or '(' but found {found}")]
    ExpectedExpression { found: TokenKind<'src> },
    #[error("expected `if`, `while` or an identifier but found {found}")]
    ExpectedCommand { found: TokenKind<'src> },
    #[error("expected an identifier but found {found}")]
    ExpectedIdentifier { found: TokenKind<'src> },
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

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<Span, ParseError<'src>> {
        match self.lookahead.kind {
            TokenKind::Keyword(found) if found == keyword => {
                let span = self.lookahead.span;
                self.advance()?;
                Ok(span)
            },
            found => Err(self.error(ParseErrorKind::Expected { found, expected: keyword.into() })),
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

    /// `<ident> '=' <exp> ';'`
    fn binding(&mut self) -> Result<(Spanned<&'src str>, Expression<'src>), ParseError<'src>> {
        let name = self.expect_ident()?;
        self.expect_punct(Punct::Eq)?;
        let exp = self.parse_node()?;
        self.expect_punct(Punct::Semicolon)?;

        Ok((name, exp))
    }

    fn expect_ident(&mut self) -> Result<Spanned<&'src str>, ParseError<'src>> {
        let token = self.peek();
        match token.kind {
            TokenKind::Ident(ident) => {
                self.advance()?;
                Ok(Spanned::new(ident, token.span))
            },
            found => Err(self.error(ParseErrorKind::ExpectedIdentifier { found })),
        }
    }

    /// parses `<item> (',' <item>)*` up to the closing `)`, which is consumed
    fn list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Result<T, ParseError<'src>>,
    ) -> Result<(Vec<T>, Span), ParseError<'src>> {
        let mut items = Vec::new();

        if self.peek().kind != TokenKind::Punct(Punct::CloseParen) {
            items.push(item(self)?);
            while self.peek().kind == TokenKind::Punct(Punct::Comma) {
                self.advance()?;
                items.push(item(self)?);
            }
        }

        let close = self.expect_punct(Punct::CloseParen)?;
        Ok((items, close))
    }

    /// `<cmd>* 'return' <exp> ';' '}'`, the opening brace was already consumed
    fn body(&mut self) -> Result<Body<'src>, ParseError<'src>> {
        let mut commands = Vec::new();
        while !matches!(
            self.peek().kind,
            TokenKind::Keyword(Keyword::Return) | TokenKind::Punct(Punct::CloseBrace)
        ) {
            commands.push(self.parse_node()?);
        }

        self.expect_keyword(Keyword::Return)?;
        let result = self.parse_node()?;
        self.expect_punct(Punct::Semicolon)?;
        self.expect_punct(Punct::CloseBrace)?;

        Ok(Body { commands, result })
    }

    fn block(&mut self) -> Result<(Vec<Command<'src>>, Span), ParseError<'src>> {
        self.expect_punct(Punct::OpenBrace)?;

        let mut commands = Vec::new();
        while self.peek().kind != TokenKind::Punct(Punct::CloseBrace) {
            commands.push(self.parse_node()?);
        }

        let close = self.expect_punct(Punct::CloseBrace)?;
        Ok((commands, close))
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
                if self.peek().kind != TokenKind::Punct(Punct::OpenParen) {
                    return Ok(Spanned::new(ExpressionKind::Variable(ident), token.span));
                }

                self.advance()?;
                let (args, close) = self.list(Self::parse_node)?;
                let name = Spanned::new(ident, token.span);
                Ok(Spanned::new(ExpressionKind::Call { name, args }, token.span.merge(close)))
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
        let mut items = Vec::new();

        loop {
            match parser.peek().kind {
                TokenKind::Keyword(Keyword::Var) => items.push(Item::Var(parser.parse_node()?)),
                TokenKind::Keyword(Keyword::Fun) => items.push(Item::Fn(parser.parse_node()?)),
                _ => break,
            }
        }

        parser.expect_keyword(Keyword::Main)?;
        parser.expect_punct(Punct::OpenBrace)?;
        let main = parser.body()?;

        Ok(Self { items, main })
    }
}

impl<'src> Parsable<'src> for Expression<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        parser.binary(BinaryOperator::LOWEST_PRECEDENCE)
    }
}

impl<'src> Parsable<'src> for Var<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        parser.expect_keyword(Keyword::Var)?;
        let (name, value) = parser.binding()?;
        Ok(Self { name, value })
    }
}

impl<'src> Parsable<'src> for Function<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        parser.expect_keyword(Keyword::Fun)?;
        let name = parser.expect_ident()?;

        parser.expect_punct(Punct::OpenParen)?;
        let (params, _) = parser.list(Parser::expect_ident)?;

        parser.expect_punct(Punct::OpenBrace)?;
        let mut locals = Vec::new();
        while parser.peek().kind == TokenKind::Keyword(Keyword::Var) {
            locals.push(parser.parse_node()?);
        }
        let body = parser.body()?;

        Ok(Self { name, params, locals, body })
    }
}

impl<'src> Parsable<'src> for Command<'src> {
    fn parse(parser: &mut Parser<'src>) -> Result<Self, ParseError<'src>> {
        let token = parser.peek();

        match token.kind {
            TokenKind::Ident(_) => {
                let (name, value) = parser.binding()?;
                let span = name.span.merge(value.span);

                Ok(Self::new(CommandKind::Assign { name, value }, span))
            },
            TokenKind::Keyword(Keyword::If) => {
                parser.advance()?;
                let condition = parser.parse_node()?;
                let (then, _) = parser.block()?;

                parser.expect_keyword(Keyword::Else)?;
                let (otherwise, close) = parser.block()?;

                let cmd = CommandKind::If { condition, then, otherwise };
                Ok(Self::new(cmd, token.span.merge(close)))
            },
            TokenKind::Keyword(Keyword::While) => {
                parser.advance()?;
                let condition = parser.parse_node()?;
                let (body, close) = parser.block()?;

                let cmd = CommandKind::While { condition, body };
                Ok(Self::new(cmd, token.span.merge(close)))
            },
            found => Err(parser.error(ParseErrorKind::ExpectedCommand { found })),
        }
    }
}

impl BinaryOperator {
    const LOWEST_PRECEDENCE: u8 = 1;

    fn from_token(kind: TokenKind<'_>) -> Option<Self> {
        match kind {
            TokenKind::Punct(Punct::Plus) => Some(Self::Add),
            TokenKind::Punct(Punct::Minus) => Some(Self::Sub),
            TokenKind::Punct(Punct::Star) => Some(Self::Mul),
            TokenKind::Punct(Punct::Slash) => Some(Self::Div),
            TokenKind::Punct(Punct::Lt) => Some(Self::Lt),
            TokenKind::Punct(Punct::Gt) => Some(Self::Gt),
            TokenKind::Punct(Punct::EqEq) => Some(Self::Eq),
            _ => None,
        }
    }

    const fn precedence(self) -> u8 {
        match self {
            Self::Lt | Self::Gt | Self::Eq => 1,
            Self::Add | Self::Sub => 2,
            Self::Mul | Self::Div => 3,
        }
    }

    pub const fn as_str(&self) -> &str {
        match self {
            BinaryOperator::Eq => "==",
            BinaryOperator::Lt => "<",
            BinaryOperator::Gt => ">",
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
        }
    }
}

impl<'src> From<LexError<'src>> for ParseError<'src> {
    fn from(error: LexError<'src>) -> Self {
        error.map(ParseErrorKind::Lex)
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
        assert!(program.items.is_empty());
        program.main.result
    }

    #[test]
    fn specification_example_structure() {
        let expr = result("main { return (33 + (912 * 11)); }");
        assert_eq!(expr.span, Span::new(14, 31));

        let ExpressionKind::Binary { right, left, operator } = &expr.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*operator, BinaryOperator::Add);
        assert_eq!(left.kind, ExpressionKind::Integer(33));
        assert_eq!(right.span, Span::new(20, 30));

        let ExpressionKind::Binary { operator, left, right } = &right.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*operator, BinaryOperator::Mul);
        assert_eq!(left.kind, ExpressionKind::Integer(912));
        assert_eq!(right.kind, ExpressionKind::Integer(11));
    }

    #[test]
    fn unclosed_paren_is_rejected() {
        let error = parse("main { return (1 + 2").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: TokenKind::Eof, expected: Punct::CloseParen.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(20, 20));
    }

    #[test]
    fn missing_operator_is_rejected() {
        let error = parse("main { return (1 2); }").unwrap_err();
        let expected = ParseErrorKind::Expected {
            found: TokenKind::Integer(2),
            expected: Punct::CloseParen.into(),
        };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(17, 18));
    }

    #[test]
    fn trailing_tokens_are_rejected() {
        let error = parse("main { return 1; } 2").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: TokenKind::Integer(2), expected: TokenKind::Eof };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(19, 20));
    }

    #[test]
    fn empty_parens_are_rejected() {
        let error = parse("main { return (); }").unwrap_err();
        let expected = ParseErrorKind::ExpectedExpression { found: Punct::CloseParen.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(15, 16));
    }

    #[test]
    fn empty_input_is_rejected() {
        let error = parse("").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: TokenKind::Eof, expected: Keyword::Main.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(0, 0));
    }

    #[test]
    fn lexical_error_surfaces_through_parser() {
        let error = parse("main { return (1 + $); }").unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::Lex(LexErrorKind::UnexpectedChar('$')));
        assert_eq!(error.span, Span::new(19, 20));
    }

    #[test]
    fn declarations_and_variables_are_parsed() {
        let program = parse("var x = 7 * 8;\nvar y = x - 1;\nmain { return x * y; }").unwrap();
        let names: Vec<_> = program
            .items
            .iter()
            .map(|item| match item {
                Item::Var(var) => var.name.kind,
                Item::Fn(_) => panic!("expected only variables"),
            })
            .collect();
        assert_eq!(names, ["x", "y"]);

        let ExpressionKind::Binary { left, operator, right } = &program.main.result.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*operator, BinaryOperator::Mul);
        assert_eq!(left.kind, ExpressionKind::Variable("x"));
        assert_eq!(right.kind, ExpressionKind::Variable("y"));
    }

    #[test]
    fn precedence_and_left_associativity() {
        let program = parse("main { return 10 - 2 - 3 * 2; }").unwrap();
        let ExpressionKind::Binary { left, operator: BinaryOperator::Sub, right } =
            &program.main.result.kind
        else {
            panic!("expected a subtraction at the root");
        };
        assert!(matches!(
            left.kind,
            ExpressionKind::Binary { operator: BinaryOperator::Sub, .. }
        ));
        assert!(matches!(
            right.kind,
            ExpressionKind::Binary { operator: BinaryOperator::Mul, .. }
        ));
    }

    #[test]
    fn declaration_without_semicolon_is_rejected() {
        let error = parse("var x = 1\n= x").unwrap_err();
        let expected =
            ParseErrorKind::Expected { found: Punct::Eq.into(), expected: Punct::Semicolon.into() };
        assert_eq!(error.kind, expected);
        assert_eq!(error.span, Span::new(10, 11));
    }

    #[test]
    fn functions_and_calls_are_parsed() {
        let program =
            parse("fun f(x, y) { var z = x; return g(z, 1) + h(); }\nmain { return 0; }").unwrap();
        let [Item::Fn(function)] = program.items.as_slice() else {
            panic!("expected a single function");
        };

        assert_eq!(function.name.kind, "f");
        assert_eq!(function.params.iter().map(|param| param.kind).collect::<Vec<_>>(), ["x", "y"]);
        assert_eq!(function.locals.len(), 1);

        let ExpressionKind::Binary { left, right, .. } = &function.body.result.kind else {
            panic!("expected a binary expression");
        };
        let ExpressionKind::Call { name, args } = &left.kind else {
            panic!("expected a call")
        };
        assert_eq!(name.kind, "g");
        assert_eq!(args[0].kind, ExpressionKind::Variable("z"));
        assert_eq!(args[1].kind, ExpressionKind::Integer(1));
        assert!(matches!(&right.kind, ExpressionKind::Call { args, .. } if args.is_empty()));
    }

    #[test]
    fn trailing_comma_is_rejected() {
        let error = parse("fun f(x,) { return x; }\nmain { return 0; }").unwrap_err();
        let expected = ParseErrorKind::ExpectedIdentifier { found: Punct::CloseParen.into() };
        assert_eq!(error.kind, expected);
    }

    #[test]
    fn var_inside_main_is_rejected() {
        let error = parse("main { var x = 1; return x; }").unwrap_err();
        let expected = ParseErrorKind::ExpectedCommand { found: Keyword::Var.into() };
        assert_eq!(error.kind, expected);
    }
}
