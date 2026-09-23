pub struct Lexer<'src> {
    src: &'src str,
    position: usize,
}

/// A single token produced by the lexer
pub type Token<'src> = Spanned<TokenKind<'src>>;
pub type LexError<'src> = Spanned<LexErrorKind<'src>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind<'src> {
    Integer(i64),
    Punct(Punct),
    Eof,
    /// we will probably have some variant to hold an str, I'm used to that
    /// and have have passed the pain to add a lifetime to an enum that doesn't have it
    /// too many times in rust so I'll just put this here now in case we need in the future
    #[doc(hidden)]
    _M(std::marker::PhantomData<&'src ()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<K> {
    pub kind: K,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A byte range `[start, end)` in the source used to
/// indicate errors on the respective file
pub struct Span {
    start: u32,
    end: u32,
}

#[derive(Debug, PartialEq)]
pub enum LexErrorKind<'src> {
    UnexpectedChar(char),
    InvalidInteger(&'src str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Punct {
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

    OpenParen,  // (
    CloseParen, // )
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { src, position: 0 }
    }

    pub fn next_token(&mut self) -> Result<Token<'src>, LexError<'src>> {
        self.skip_whitespaces();
        let start = self.position;

        let Some(c) = self.src[start..].chars().next() else {
            return Ok(Spanned::new(TokenKind::Eof, Span::new(start, start)));
        };

        let punct = match c {
            '0'..='9' => return self.number(start),
            '+' => Punct::Plus,
            '-' => Punct::Minus,
            '*' => Punct::Star,
            '/' => Punct::Slash,
            '(' => Punct::OpenParen,
            ')' => Punct::CloseParen,
            _ => {
                let span = Span::new(start, start + c.len_utf8());
                return Err(Spanned::new(LexErrorKind::UnexpectedChar(c), span));
            },
        };

        self.position += 1;
        Ok(Spanned::new(TokenKind::Punct(punct), Span::new(start, self.position)))
    }

    fn number(&mut self, start: usize) -> Result<Token<'src>, LexError<'src>> {
        let len = self.src[start..].bytes().take_while(u8::is_ascii_digit).count();
        self.position = start + len;
        let span = Span::new(start, self.position);

        let slice = span.slice(self.src);

        match slice.parse::<i64>() {
            Ok(value) => Ok(Spanned::new(TokenKind::Integer(value), span)),
            _ => Err(Spanned::new(LexErrorKind::InvalidInteger(slice), span)),
        }
    }

    fn skip_whitespaces(&mut self) {
        let len = self.src[self.position..].bytes().take_while(u8::is_ascii_whitespace).count();
        self.position += len;
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "well, shit happens: {start} past its {end}");
        let start = u32::try_from(start).expect("source file to not be greater than u32::MAX");
        let end = u32::try_from(end).expect("source file to not be greater than u32::MAX");

        Self { start, end }
    }

    pub fn slice(self, src: &str) -> &str {
        &src[self.start as usize..self.end as usize]
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl<K> Spanned<K> {
    pub fn new(kind: K, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn map<U>(self, f: impl FnOnce(K) -> U) -> Spanned<U> {
        Spanned { kind: f(self.kind), span: self.span }
    }
}

impl From<Punct> for char {
    fn from(punct: Punct) -> Self {
        match punct {
            Punct::Plus => '+',
            Punct::Minus => '-',
            Punct::Star => '*',
            Punct::Slash => '/',
            Punct::OpenParen => '(',
            Punct::CloseParen => ')',
        }
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let position = self.span.start;

        match self.kind {
            TokenKind::Integer(int) => write!(f, "<Integer, '{int}', {position}>"),
            TokenKind::Punct(punct) => {
                write!(f, "<{punct:?}, '{}', {position}>", char::from(punct))
            },
            TokenKind::Eof => write!(f, "<Eof, {position}>"),
            TokenKind::_M(_) => unreachable!(),
        }
    }
}

impl std::fmt::Display for LexError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let position = self.span.start;
        write!(f, "lexical error at {position}: ")?;

        match self.kind {
            LexErrorKind::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            LexErrorKind::InvalidInteger(int) => {
                write!(f, "integer '{int}' doesn't fit in 64 bits")
            },
        }
    }
}

impl From<Punct> for TokenKind<'_> {
    fn from(value: Punct) -> Self {
        Self::Punct(value)
    }
}

impl From<i64> for TokenKind<'_> {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Result<Vec<Token<'_>>, LexError<'_>> {
        let mut lexer = Lexer::new(src);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            match token.kind {
                TokenKind::Eof => return Ok(tokens),
                _ => tokens.push(token),
            }
        }
    }

    #[test]
    fn specficiation_example() {
        let tokens = lex("(33 + (912 * 11))").unwrap();
        let expected = [
            (Punct::OpenParen.into(), 0, 1),
            (33.into(), 1, 3),
            (Punct::Plus.into(), 4, 5),
            (Punct::OpenParen.into(), 6, 7),
            (912.into(), 7, 10),
            (Punct::Star.into(), 11, 12),
            (11.into(), 13, 15),
            (Punct::CloseParen.into(), 15, 16),
            (Punct::CloseParen.into(), 16, 17),
        ];

        assert_eq!(tokens.len(), expected.len());
        for (token, (kind, start, end)) in tokens.iter().zip(expected) {
            assert_eq!(token.kind, kind);
            assert_eq!(token.span, Span::new(start, end));
        }
    }

    #[test]
    fn display_follows_specification_format() {
        let output: Vec<_> =
            lex("(33 + (912 * 11))").unwrap().iter().map(ToString::to_string).collect();

        let expected = [
            r#"<OpenParen, '(', 0>"#,
            r#"<Integer, '33', 1>"#,
            r#"<Plus, '+', 4>"#,
            r#"<OpenParen, '(', 6>"#,
            r#"<Integer, '912', 7>"#,
            r#"<Star, '*', 11>"#,
            r#"<Integer, '11', 13>"#,
            r#"<CloseParen, ')', 15>"#,
            r#"<CloseParen, ')', 16>"#,
        ];

        assert_eq!(output, expected);
        println!("{output:#?}");
    }

    #[test]
    fn integer_past_i64_max_non_accepted() {
        let tokens = lex("(1 + 9223372036854775808)").unwrap_err();
        assert!(matches!(tokens.kind, LexErrorKind::InvalidInteger("9223372036854775808")));
        assert_eq!(tokens.span, Span::new(5, 24))
    }

    #[test]
    fn unexpected_char_is_rejected() {
        let tokens = lex("(1 + a)").unwrap_err();
        assert!(matches!(tokens.kind, LexErrorKind::UnexpectedChar('a')));
        assert_eq!(tokens.span, Span::new(5, 6))
    }
}
