use crate::Keyword;
use crate::parser::Parser;

impl Parser<'_> {
    pub(super) fn declaration(&mut self) -> bool {
        match self.current_keyword() {
            Some(Keyword::Party) => self.party(),
            Some(Keyword::Candidate) => self.candidate(),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;
    use crate::parser::tests::check;
    use expect_test::expect;

    #[test]
    fn declarations_follow_the_guide_fields() {
        let text = "guide \"G\"\n\nparty @dem \"Democratic\"\nparty @rep \"Republican\"\n";
        assert_eq!(parse(text).unwrap().syntax_errors, []);
        check(
            text,
            &expect![[r#"
            Document@0..59
              StringField@0..10
                Word@0..5 "guide"
                Whitespace@5..6 " "
                String@6..9 "\"G\""
                Newline@9..10 "\n"
              BlankLine@10..11 "\n"
              Party@11..35
                Word@11..16 "party"
                Whitespace@16..17 " "
                Identifier@17..21
                  AtSign@17..18 "@"
                  Word@18..21 "dem"
                Whitespace@21..22 " "
                String@22..34 "\"Democratic\""
                Newline@34..35 "\n"
              Party@35..59
                Word@35..40 "party"
                Whitespace@40..41 " "
                Identifier@41..45
                  AtSign@41..42 "@"
                  Word@42..45 "rep"
                Whitespace@45..46 " "
                String@46..58 "\"Republican\""
                Newline@58..59 "\n"
        "#]],
        );
    }
}
