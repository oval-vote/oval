use crate::TokenKind;
use crate::lexer::Lexer;

impl Lexer<'_> {
    pub(in crate::lexer) fn at_sign(&mut self, start: usize) {
        self.single(TokenKind::AtSign, '@', start);
    }

    pub(in crate::lexer) fn colon(&mut self, start: usize) {
        self.single(TokenKind::Colon, ':', start);
    }

    pub(in crate::lexer) fn comma(&mut self, start: usize) {
        self.single(TokenKind::Comma, ',', start);
    }

    fn single(&mut self, kind: TokenKind, c: char, start: usize) {
        self.bump(c);
        self.push(kind, start);
    }
}

#[cfg(test)]
mod tests {
    use crate::TokenKind::{AtSign, Colon, Comma, Comment, Newline};
    use crate::lexer::tests::check_lexer;

    #[test]
    fn each_punctuator_is_its_own_token() {
        check_lexer("@", &[(AtSign, "@"), (Newline, "")], &[]);
        check_lexer(":", &[(Colon, ":"), (Newline, "")], &[]);
        check_lexer(",", &[(Comma, ","), (Newline, "")], &[]);
    }

    #[test]
    fn adjacent_punctuators_are_separate_tokens() {
        check_lexer(
            "@:,",
            &[(AtSign, "@"), (Colon, ":"), (Comma, ","), (Newline, "")],
            &[],
        );
    }

    #[test]
    fn a_line_with_a_punctuator_ends_in_a_newline() {
        for (punctuator, kind) in [("@", AtSign), (":", Colon), (",", Comma)] {
            check_lexer(
                &format!("{punctuator}\n"),
                &[(kind, punctuator), (Newline, "\n")],
                &[],
            );
        }
    }

    #[test]
    fn punctuators_inside_a_comment_are_part_of_it() {
        check_lexer("# @:,", &[(Comment, "# @:,")], &[]);
    }
}
