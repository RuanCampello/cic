#[derive(Debug, Clone, Copy, PartialEq)]
enum Token<'src> {
    Integer(u64),
    Punct(Punct),
    Eof,
    /// we will probably have some variant to hold an str, I'm used to that
    /// and have have passed the pain to add a lifetime to an enum that doesn't have it
    /// too many times in rust so I'll just put this here now in case we need in the future
    #[doc(hidden)]
    _M(std::marker::PhantomData<&'src ()>),
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
