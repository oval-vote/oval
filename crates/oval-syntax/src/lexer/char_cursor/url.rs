use super::{CharCursor, is_line_terminator};
use crate::{LexicalErrorKind, TokenKind};

impl CharCursor<'_> {
    pub(super) fn at_url(&self) -> bool {
        self.rest().starts_with("http://") || self.rest().starts_with("https://")
    }

    pub(super) fn url(&mut self, start: usize) {
        self.eat_while(|c| c != ':');
        self.pos += "://".len();
        let path_start = self.pos;
        self.eat_while(|c| !matches!(c, ' ' | '\t') && !is_line_terminator(c));
        if self.pos == path_start {
            self.push_error(LexicalErrorKind::MalformedUrl, start);
        } else {
            self.push(TokenKind::Url, start);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::LexicalErrorKind::{IllegalTab, MalformedUrl, UnexpectedCharacter};
    use crate::TokenKind::{Colon, Error, Newline, Url, Whitespace, Word};
    use crate::lexer::tests::{check_lexer, error};

    #[test]
    fn a_url_is_one_token() {
        for text in [
            "https://santosformayor.example",
            "http://x",
            "https://a.example/b?c=d",
        ] {
            check_lexer(text, &[(Url, text), (Newline, "")], &[]);
        }
    }

    #[test]
    fn a_url_includes_characters_that_begin_other_tokens() {
        // A URL is written bare, so `#`, `"`, `@`, `:`, `,`, and characters outside ASCII are part
        // of it rather than the start of a comment, a string, or an error.
        let text = "https://a.example/#top\"@:,é";
        check_lexer(text, &[(Url, text), (Newline, "")], &[]);
    }

    #[test]
    fn a_url_ends_at_a_space_a_tab_or_a_line_terminator() {
        check_lexer(
            "https://a b",
            &[
                (Url, "https://a"),
                (Whitespace, " "),
                (Word, "b"),
                (Newline, ""),
            ],
            &[],
        );
        check_lexer(
            "https://a\tb",
            &[
                (Url, "https://a"),
                (Error, "\t"),
                (Word, "b"),
                (Newline, ""),
            ],
            &[error(IllegalTab, 9, 1)],
        );
        check_lexer(
            "https://a\nb",
            &[
                (Url, "https://a"),
                (Newline, "\n"),
                (Word, "b"),
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_scheme_with_nothing_after_it_is_a_malformed_url() {
        for scheme in ["https://", "http://"] {
            let len = u32::try_from(scheme.len()).unwrap();
            check_lexer(
                scheme,
                &[(Error, scheme), (Newline, "")],
                &[error(MalformedUrl, 0, len)],
            );
        }
    }

    #[test]
    fn a_malformed_url_ends_before_a_space_or_a_line_terminator() {
        check_lexer(
            "https:// x",
            &[
                (Error, "https://"),
                (Whitespace, " "),
                (Word, "x"),
                (Newline, ""),
            ],
            &[error(MalformedUrl, 0, 8)],
        );
        check_lexer(
            "https://\n",
            &[(Error, "https://"), (Newline, "\n")],
            &[error(MalformedUrl, 0, 8)],
        );
    }

    #[test]
    fn only_http_and_https_begin_a_url() {
        check_lexer(
            "ftp://x",
            &[
                (Word, "ftp"),
                (Colon, ":"),
                (Error, "/"),
                (Error, "/"),
                (Word, "x"),
                (Newline, ""),
            ],
            &[
                error(UnexpectedCharacter, 4, 1),
                error(UnexpectedCharacter, 5, 1),
            ],
        );
    }

    #[test]
    fn a_scheme_is_lowercase() {
        check_lexer(
            "HTTPS://x",
            &[
                (Word, "HTTPS"),
                (Colon, ":"),
                (Error, "/"),
                (Error, "/"),
                (Word, "x"),
                (Newline, ""),
            ],
            &[
                error(UnexpectedCharacter, 6, 1),
                error(UnexpectedCharacter, 7, 1),
            ],
        );
    }

    #[test]
    fn a_scheme_needs_both_slashes() {
        check_lexer(
            "https:x",
            &[(Word, "https"), (Colon, ":"), (Word, "x"), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn a_longer_word_is_not_a_scheme() {
        check_lexer(
            "httpsx://y",
            &[
                (Word, "httpsx"),
                (Colon, ":"),
                (Error, "/"),
                (Error, "/"),
                (Word, "y"),
                (Newline, ""),
            ],
            &[
                error(UnexpectedCharacter, 7, 1),
                error(UnexpectedCharacter, 8, 1),
            ],
        );
    }

    #[test]
    fn the_scheme_words_alone_are_words() {
        check_lexer(
            "https http",
            &[
                (Word, "https"),
                (Whitespace, " "),
                (Word, "http"),
                (Newline, ""),
            ],
            &[],
        );
    }

    #[test]
    fn a_website_field() {
        // The specification's example for the malformed-url error.
        check_lexer(
            "website https://santosformayor.example\n",
            &[
                (Word, "website"),
                (Whitespace, " "),
                (Url, "https://santosformayor.example"),
                (Newline, "\n"),
            ],
            &[],
        );
    }
}
