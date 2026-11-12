use crate::parser::Parser;
use crate::{Expected, Keyword, SyntaxErrorKind, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn block(
        &mut self,
        accepts: &[Expected],
        mut entry: impl FnMut(&mut Self, &mut Vec<Keyword>) -> bool,
    ) {
        self.start_node(SyntaxKind::Block);
        self.bump();
        let mut seen = Vec::new();
        while !self.at(TokenKind::Dedent) && self.current().is_some() {
            if !entry(self, &mut seen) {
                self.skip_entry(accepts);
            }
        }
        self.eat(TokenKind::Dedent);
        self.finish_node();
    }

    fn skip_entry(&mut self, accepts: &[Expected]) {
        self.start_node(SyntaxKind::ErrorNode);
        let start = self.offset;
        while !self.at_line_end() {
            self.bump();
        }
        self.eat(TokenKind::Newline);
        if !self.eat(TokenKind::MarkdownBlock) && self.at(TokenKind::Indent) {
            self.skip_block();
        }
        self.finish_node();
        let skipped_text = self.line_end > start;
        if skipped_text {
            self.report(
                SyntaxErrorKind::UnexpectedEntry,
                start,
                self.line_end,
                accepts.to_vec(),
            );
        }
    }

    fn skip_block(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.current() {
                Some(TokenKind::Indent) => depth += 1,
                Some(TokenKind::Dedent) => depth -= 1,
                Some(_) => {}
                None => return,
            }
            self.bump();
            if depth == 0 {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, parse};
    use expect_test::expect;

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    fn unexpected_entry(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedEntry,
            offset,
            len,
            expected: vec![
                Expected::Keyword(Keyword::Abbreviation),
                Expected::Keyword(Keyword::Website),
                Expected::Keyword(Keyword::Note),
            ],
        }
    }

    #[test]
    fn an_entry_the_block_does_not_accept_is_skipped() {
        let text = "party @dem \"D\"\n  designation \"D\"\n  abbreviation \"D\"\n";
        check_errors(text, &[unexpected_entry(17, 15)]);
        check(
            text,
            &expect![[r#"
            Document@0..52
              Party@0..52
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..14 "\"D\""
                Newline@14..15 "\n"
                Whitespace@15..17 "  "
                Block@17..52
                  Indent@17..17 ""
                  ErrorNode@17..33
                    Word@17..28 "designation"
                    Whitespace@28..29 " "
                    String@29..32 "\"D\""
                    Newline@32..33 "\n"
                  Whitespace@33..35 "  "
                  StringField@35..52
                    Word@35..47 "abbreviation"
                    Whitespace@47..48 " "
                    String@48..51 "\"D\""
                    Newline@51..52 "\n"
                  Dedent@52..52 ""
        "#]],
        );
    }

    #[test]
    fn a_string_cannot_begin_an_entry() {
        check_errors("party @dem \"D\"\n  \"D\"\n", &[unexpected_entry(17, 3)]);
    }

    #[test]
    fn an_unexpected_entry_is_skipped_with_its_body() {
        let text = "party @dem \"D\"\n  designation:\n    Text.\n  abbreviation \"D\"\n";
        check_errors(text, &[unexpected_entry(17, 23)]);
        check(
            text,
            &expect![[r#"
            Document@0..59
              Party@0..59
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..14 "\"D\""
                Newline@14..15 "\n"
                Whitespace@15..17 "  "
                Block@17..59
                  Indent@17..17 ""
                  ErrorNode@17..40
                    Word@17..28 "designation"
                    Colon@28..29 ":"
                    Newline@29..30 "\n"
                    MarkdownBlock@30..40 "    Text.\n"
                  Whitespace@40..42 "  "
                  StringField@42..59
                    Word@42..54 "abbreviation"
                    Whitespace@54..55 " "
                    String@55..58 "\"D\""
                    Newline@58..59 "\n"
                  Dedent@59..59 ""
        "#]],
        );
    }

    #[test]
    fn an_unexpected_entry_is_skipped_with_its_block() {
        let text = "party @dem \"D\"\n  designation \"D\"\n    website https://x.example\n  abbreviation \"D\"\nparty @rep \"R\"\n";
        check_errors(text, &[unexpected_entry(17, 45)]);
        check(
            text,
            &expect![[r#"
            Document@0..97
              Party@0..82
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..14 "\"D\""
                Newline@14..15 "\n"
                Whitespace@15..17 "  "
                Block@17..82
                  Indent@17..17 ""
                  ErrorNode@17..65
                    Word@17..28 "designation"
                    Whitespace@28..29 " "
                    String@29..32 "\"D\""
                    Newline@32..33 "\n"
                    Whitespace@33..37 "    "
                    Indent@37..37 ""
                    Word@37..44 "website"
                    Whitespace@44..45 " "
                    Url@45..62 "https://x.example"
                    Newline@62..63 "\n"
                    Whitespace@63..65 "  "
                    Dedent@65..65 ""
                  StringField@65..82
                    Word@65..77 "abbreviation"
                    Whitespace@77..78 " "
                    String@78..81 "\"D\""
                    Newline@81..82 "\n"
                  Dedent@82..82 ""
              Party@82..97
                Word@82..87 "party"
                Whitespace@87..88 " "
                Identifier@88..92
                  AtSign@88..89 "@"
                  Word@89..92 "rep"
                Whitespace@92..93 " "
                String@93..96 "\"R\""
                Newline@96..97 "\n"
        "#]],
        );
    }

    #[test]
    fn a_nested_block_is_skipped_to_its_own_dedent() {
        let text = "party @dem \"D\"\n  designation \"D\"\n    a\n      b\n    c\n  abbreviation \"D\"\nparty @rep \"R\"\n";
        check_errors(text, &[unexpected_entry(17, 35)]);
        check(
            text,
            &expect![[r#"
            Document@0..87
              Party@0..72
                Word@0..5 "party"
                Whitespace@5..6 " "
                Identifier@6..10
                  AtSign@6..7 "@"
                  Word@7..10 "dem"
                Whitespace@10..11 " "
                String@11..14 "\"D\""
                Newline@14..15 "\n"
                Whitespace@15..17 "  "
                Block@17..72
                  Indent@17..17 ""
                  ErrorNode@17..55
                    Word@17..28 "designation"
                    Whitespace@28..29 " "
                    String@29..32 "\"D\""
                    Newline@32..33 "\n"
                    Whitespace@33..37 "    "
                    Indent@37..37 ""
                    Word@37..38 "a"
                    Newline@38..39 "\n"
                    Whitespace@39..45 "      "
                    Indent@45..45 ""
                    Word@45..46 "b"
                    Newline@46..47 "\n"
                    Whitespace@47..51 "    "
                    Dedent@51..51 ""
                    Word@51..52 "c"
                    Newline@52..53 "\n"
                    Whitespace@53..55 "  "
                    Dedent@55..55 ""
                  StringField@55..72
                    Word@55..67 "abbreviation"
                    Whitespace@67..68 " "
                    String@68..71 "\"D\""
                    Newline@71..72 "\n"
                  Dedent@72..72 ""
              Party@72..87
                Word@72..77 "party"
                Whitespace@77..78 " "
                Identifier@78..82
                  AtSign@78..79 "@"
                  Word@79..82 "rep"
                Whitespace@82..83 " "
                String@83..86 "\"R\""
                Newline@86..87 "\n"
        "#]],
        );
    }

    #[test]
    fn an_unexpected_entry_can_end_two_blocks_at_once() {
        check_errors(
            "party @dem \"D\"\n  designation \"D\"\n    website https://x.example\nparty @rep \"R\"\n  abbreviation \"R\"\n",
            &[unexpected_entry(17, 45)],
        );
    }

    #[test]
    fn an_unexpected_entry_ends_at_its_last_token_the_parser_does_not_skip() {
        let result = parse("party @dem \"D\"\n  designation ~  # aside\n").unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, [unexpected_entry(17, 11)]);
    }

    #[test]
    fn a_block_of_only_error_tokens_after_a_field_is_not_an_error() {
        let result = parse("party @dem \"D\"\n  abbreviation \"D\"\n    \t\n").unwrap();
        assert_eq!(result.lexical_errors.len(), 1);
        assert_eq!(result.syntax_errors, []);
    }
}
