/// A token in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The length of the token in bytes
    pub len: u32,
}

/// The kinds of tokens in an Oval document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A word token
    Word,
    /// A string token
    String,
    /// An integer token
    Integer,
    /// A date token
    Date,
    /// An at sign
    At,
    /// A URL
    Url,
    /// A colon
    Colon,
    /// A comma
    Comma,
    /// A markdown block
    MarkdownBlock,
    /// A white space
    Whitespace,
    /// A comment
    Comment,
    /// A blank line
    BlankLine,
    /// A new line
    Newline,
    /// An indent
    Indent,
    /// A dedent
    Dedent,
    /// An error
    Error,
}
