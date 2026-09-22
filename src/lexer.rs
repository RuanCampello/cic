/// A single token produced by the lexer
type Token<'src> = Spanned<TokenKind<'src>>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenKind<'src> {
    Integer(u64),
    Punct(Punct),
    Eof,
    /// we will probably have some variant to hold an str, I'm used to that
    /// and have have passed the pain to add a lifetime to an enum that doesn't have it
    /// too many times in rust so I'll just put this here now in case we need in the future
    #[doc(hidden)]
    _M(std::marker::PhantomData<&'src ()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Spanned<K> {
    kind: K,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A byte range `[start, end)` in the source used to
/// indicate errors on the respective file
struct Span {
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Punct {
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

    OpenParen,  // (
    CloseParen, // )
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        assert!(start <= end, "well, shit happens: {start} past its {end}");
        Self { start, end }
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
