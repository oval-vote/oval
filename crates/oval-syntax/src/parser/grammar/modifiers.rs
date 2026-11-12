use crate::parser::Parser;
use crate::{Keyword, SyntaxKind};

pub(super) struct EntryHead {
    pub(super) requirement: bool,
    pub(super) extensibility: bool,
    pub(super) keyword: Option<Keyword>,
}

impl EntryHead {
    pub(super) fn has_modifiers(&self) -> bool {
        self.requirement || self.extensibility
    }
}

impl Parser<'_> {
    pub(super) fn entry_head(&self) -> EntryHead {
        let mut n = 0;
        let requirement = matches!(
            self.nth_keyword(n),
            Some(Keyword::Required | Keyword::Optional)
        );
        if requirement {
            n += 1;
        }
        let extensibility = matches!(
            self.nth_keyword(n),
            Some(Keyword::Extensible | Keyword::Exhaustive)
        );
        if extensibility {
            n += 1;
        }
        EntryHead {
            requirement,
            extensibility,
            keyword: self.nth_keyword(n),
        }
    }

    pub(super) fn modifiers(&mut self) {
        if matches!(
            self.current_keyword(),
            Some(Keyword::Required | Keyword::Optional)
        ) {
            self.start_node(SyntaxKind::Requirement);
            self.bump();
            self.finish_node();
        }
        if matches!(
            self.current_keyword(),
            Some(Keyword::Extensible | Keyword::Exhaustive)
        ) {
            self.start_node(SyntaxKind::Extensibility);
            self.bump();
            self.finish_node();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tests::check;
    use crate::{Expected, Keyword, SyntaxError, SyntaxErrorKind, SyntaxKind, parse};
    use expect_test::expect;

    fn ballot_child_nodes(text: &str) -> Vec<SyntaxKind> {
        let tree = parse(text).unwrap().syntax();
        let ballot = tree.first_child().expect("the document has a ballot");
        ballot.children().map(|node| node.kind()).collect()
    }

    fn unexpected_entry(offset: u32, len: u32) -> SyntaxError {
        SyntaxError {
            kind: SyntaxErrorKind::UnexpectedEntry,
            offset,
            len,
            expected: vec![
                Expected::Keyword(Keyword::Eligibility),
                Expected::Keyword(Keyword::Note),
                Expected::Keyword(Keyword::Section),
                Expected::Keyword(Keyword::Contest),
                Expected::Keyword(Keyword::Ballot),
            ],
        }
    }

    fn check_errors(text: &str, expected: &[SyntaxError]) {
        assert_eq!(parse(text).unwrap().syntax_errors, expected, "{text:?}");
    }

    #[test]
    fn modifiers_are_nodes_before_the_keyword() {
        let text = "required extensible ballot @city \"City\"\n  eligibility \"E\"\n";
        check_errors(text, &[]);
        check(
            text,
            &expect![[r#"
            Document@0..58
              Ballot@0..58
                Requirement@0..8
                  Word@0..8 "required"
                Whitespace@8..9 " "
                Extensibility@9..19
                  Word@9..19 "extensible"
                Whitespace@19..20 " "
                Word@20..26 "ballot"
                Whitespace@26..27 " "
                Identifier@27..32
                  AtSign@27..28 "@"
                  Word@28..32 "city"
                Whitespace@32..33 " "
                String@33..39 "\"City\""
                Newline@39..40 "\n"
                Whitespace@40..42 "  "
                Block@42..58
                  Indent@42..42 ""
                  StringField@42..58
                    Word@42..53 "eligibility"
                    Whitespace@53..54 " "
                    String@54..57 "\"E\""
                    Newline@57..58 "\n"
                  Dedent@58..58 ""
        "#]],
        );
    }

    #[test]
    fn a_ballot_can_have_either_modifier_alone() {
        assert_eq!(
            ballot_child_nodes("optional ballot @a \"A\"\n  eligibility \"E\"\n"),
            [
                SyntaxKind::Requirement,
                SyntaxKind::Identifier,
                SyntaxKind::Block
            ]
        );
        assert_eq!(
            ballot_child_nodes("exhaustive ballot @a \"A\"\n  eligibility \"E\"\n"),
            [
                SyntaxKind::Extensibility,
                SyntaxKind::Identifier,
                SyntaxKind::Block
            ]
        );
    }

    #[test]
    fn a_sub_ballot_can_have_modifiers() {
        check_errors(
            "ballot @a \"A\"\n  required exhaustive ballot @b \"B\"\n    eligibility \"E\"\n",
            &[],
        );
    }

    #[test]
    fn a_modifier_alone_on_its_line_is_an_unexpected_entry() {
        check_errors(
            "ballot @a \"A\"\n  required\n  ballot @b \"B\"\n    eligibility \"E\"\n",
            &[unexpected_entry(16, 8)],
        );
    }

    #[test]
    fn a_modifier_before_a_field_is_an_unexpected_entry() {
        check_errors(
            "ballot @a \"A\"\n  required eligibility \"E\"\n",
            &[unexpected_entry(16, 24)],
        );
    }

    #[test]
    fn two_requirements_are_an_unexpected_entry_with_its_block() {
        check_errors(
            "ballot @a \"A\"\n  optional required ballot @b \"B\"\n    eligibility \"E\"\n",
            &[unexpected_entry(16, 51)],
        );
    }

    #[test]
    fn modifiers_in_the_wrong_order_are_an_unexpected_entry() {
        check_errors(
            "ballot @a \"A\"\n  extensible required ballot @b \"B\"\n",
            &[unexpected_entry(16, 33)],
        );
    }
}
