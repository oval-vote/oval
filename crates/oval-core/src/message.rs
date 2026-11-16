use oval_syntax::{Expected, LexicalErrorKind, SyntaxErrorKind, TokenKind};

use crate::{DiagnosticCode, ValidationRule};

pub(crate) fn message(code: DiagnosticCode, text: &str, expected: &[Expected]) -> String {
    let (template, quoted) = match code {
        DiagnosticCode::Lexical(kind) => (lexical(kind), text),
        DiagnosticCode::Syntax(SyntaxErrorKind::UnexpectedToken) => {
            (syntax(SyntaxErrorKind::UnexpectedToken), first_token(text))
        }
        DiagnosticCode::Syntax(kind) => (syntax(kind), text),
        DiagnosticCode::Validation(rule) => (validation(rule), text),
    };
    let message = template.replacen("{}", quoted, 1);
    let names_expected = matches!(
        code,
        DiagnosticCode::Syntax(
            SyntaxErrorKind::MissingValue
                | SyntaxErrorKind::UnexpectedToken
                | SyntaxErrorKind::UnexpectedEntry
        )
    );
    match self::expected(expected) {
        Some(expected) if names_expected => format!("{message}; expected {expected}"),
        _ => message,
    }
}

fn lexical(kind: LexicalErrorKind) -> &'static str {
    match kind {
        LexicalErrorKind::UnexpectedCharacter => "`{}` can't begin a token",
        LexicalErrorKind::UnterminatedString => "the string has no closing quote",
        LexicalErrorKind::InvalidEscape => r#"`{}` isn't an escape, since only `\"` and `\\` are"#,
        LexicalErrorKind::IllegalTab => "a tab can only be in a string, a comment, or a body",
        LexicalErrorKind::MalformedNumber => "`{}` is neither an integer nor a date",
        LexicalErrorKind::MalformedUrl => "`{}` has no address after it",
        LexicalErrorKind::InconsistentDedent => "the indentation matches no enclosing block",
    }
}

fn syntax(kind: SyntaxErrorKind) -> &'static str {
    match kind {
        SyntaxErrorKind::MissingValue => "the line ends where a value is required",
        SyntaxErrorKind::UnexpectedToken => "`{}` isn't expected here",
        SyntaxErrorKind::MissingBody => "the Markdown field has no indented body",
        SyntaxErrorKind::RepeatedField => "the field is already given in this block",
        SyntaxErrorKind::MissingIdentifier => "an identifier is required here",
        SyntaxErrorKind::MissingName => "the declaration has no name after its identifier",
        SyntaxErrorKind::MissingContestKind => "the contest has no kind, such as `single`",
        SyntaxErrorKind::RepeatedModifier => "`{}` repeats a modifier in the contest header",
        SyntaxErrorKind::RepeatedFlag => "`{}` is already on this campaign",
        SyntaxErrorKind::EmptyRecommendation => "the recommendation has no `support` or `oppose`",
        SyntaxErrorKind::UnexpectedStance => "a recommendation with no stance can't have this line",
        SyntaxErrorKind::UnexpectedEntry => "the entry doesn't belong in this block",
        SyntaxErrorKind::UnexpectedBlock => "the line above takes no indented block",
        SyntaxErrorKind::MissingBlock => "the line above needs an indented block",
        SyntaxErrorKind::MisplacedEntry => "the entry belongs earlier in the document",
    }
}

fn validation(rule: ValidationRule) -> &'static str {
    match rule {
        ValidationRule::UnsupportedVersion => "this version of Oval reads only `oval 1` documents",
        ValidationRule::MissingGuideField => "the document is missing a guide field",
        ValidationRule::EmptyDocument => "the document has no ballot",
        ValidationRule::InvalidDate => "`{}` isn't a calendar date",
        ValidationRule::UndeclaredReference => "`{}` isn't declared",
        ValidationRule::DuplicateDeclaration => "`{}` is already declared",
        ValidationRule::MismatchedReference => "`{}` names the wrong kind of declaration here",
        ValidationRule::BlockOnReference => "a reference has nothing to declare",
        ValidationRule::MisplacedCandidateField => "no candidate is declared here to describe",
        ValidationRule::EmptyBallot => "the ballot has no items",
        ValidationRule::EmptySection => "the section has no contests or measures",
        ValidationRule::ContestWithoutCampaigns => "the contest has no campaigns",
        ValidationRule::CampaignWithoutMembers => "the campaign has no members",
        ValidationRule::DuplicateMember => "the member is already in this campaign",
        ValidationRule::MemberInTwoCampaigns => "the member is in an earlier campaign too",
        ValidationRule::PartyInTwoCampaigns => "a candidate and their party are both choices",
        ValidationRule::InvalidMultipleCount => "`multiple` gives at least 2 votes, not {}",
        ValidationRule::InvalidRankedMax => "`max` must be at least 1",
        ValidationRule::InvalidSeatCount => "`seats` must be at least 1",
        ValidationRule::InvalidMinimum => "no ballot could meet a `min` of {}",
        ValidationRule::IntegerTooLarge => "`{}` is too large for JSON to hold exactly",
        ValidationRule::RankedAllWithLimit => "`all` can't be used with `max` or `min`",
        ValidationRule::RetentionCampaignCount => "a retention contest has exactly one campaign",
        ValidationRule::RetentionPartyCampaign => "a retention contest can't have a party campaign",
        ValidationRule::MissingRecommendation => "the question has no recommendation",
        ValidationRule::RepeatedRecommendation => "the question already has a recommendation",
        ValidationRule::CampaignNotInContest => "`{}` isn't in any campaign in this contest",
        ValidationRule::RepeatedOppose => "the recommendation already has an `oppose` line",
        ValidationRule::RepeatedStance => "`{}` already appears in this recommendation",
        ValidationRule::TiersInSingle => "a `single` contest takes one `support` line",
        ValidationRule::RetentionStance => "a retention contest takes one answer on its campaign",
        ValidationRule::SupportOverMax => "`{}` is beyond the contest's `max`",
        ValidationRule::AbstainOnRequired => "a required question can't be `abstain`",
        ValidationRule::WriteInNotAccepted => "the contest is exhaustive, so it takes no write-ins",
        ValidationRule::ExtensibleRetention => "a retention contest can't be `extensible`",
        ValidationRule::MarkdownProfile => "the Markdown is outside the profile Oval supports",
        ValidationRule::InsecureUrl => "`{}` uses `http`, not `https`",
        ValidationRule::EmptyString => "the string is empty",
        ValidationRule::UnusedDeclaration => "nothing refers to this declaration",
        ValidationRule::DuplicatePartyName => "another party is also named {}",
        ValidationRule::DuplicateBallotTitle => "another ballot here is also titled {}",
        ValidationRule::SupportOverCount => "`{}` is beyond the votes the contest gives",
        ValidationRule::SupportBelowMinimum => {
            "the choices supported are fewer than a ballot needs"
        }
        ValidationRule::WriteInOnBallot => "{} is on the ballot, so name their campaign instead",
        ValidationRule::UnpairedOutcome => "the measure has only one of `if_yes` and `if_no`",
    }
}

fn first_token(text: &str) -> &str {
    oval_syntax::lex(text)
        .ok()
        .and_then(|result| result.tokens.into_iter().find(|token| token.len > 0))
        .map_or(text, |token| &text[..token.len as usize])
}

fn expected(expected: &[Expected]) -> Option<String> {
    let names: Vec<String> = expected.iter().map(|expected| name(*expected)).collect();
    match names.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        [first, second] => Some(format!("{first} or {second}")),
        [rest @ .., last] => Some(format!("{}, or {last}", rest.join(", "))),
    }
}

fn name(expected: Expected) -> String {
    match expected {
        Expected::Keyword(keyword) => format!("`{}`", keyword.text()),
        Expected::Token(kind) => match kind {
            TokenKind::Word => "a word",
            TokenKind::String => "a string",
            TokenKind::Integer => "an integer",
            TokenKind::Date => "a date",
            TokenKind::AtSign => "`@`",
            TokenKind::Url => "a URL",
            TokenKind::Colon => "`:`",
            TokenKind::Comma => "`,`",
            TokenKind::MarkdownBlock => "an indented Markdown body",
            TokenKind::Whitespace => "a space",
            TokenKind::Comment => "a comment",
            TokenKind::ByteOrderMark => "a byte order mark",
            TokenKind::BlankLine => "a blank line",
            TokenKind::Newline => "the end of the line",
            TokenKind::Indent => "an indented block",
            TokenKind::Dedent => "the end of the block",
            TokenKind::Error => "an error",
        }
        .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use oval_syntax::{Expected, Keyword, TokenKind};

    use crate::check;
    use crate::validate::VALID;

    fn messages(text: &str) -> Vec<String> {
        check(text)
            .unwrap()
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    }

    #[test]
    fn a_message_quotes_the_text_it_is_about() {
        assert_eq!(
            messages(&VALID.replace("2026-11-03", "2026-02-30")),
            ["`2026-02-30` isn't a calendar date"]
        );
        assert_eq!(
            messages(&VALID.replace("    recommend yes\n", "    recommend yes\n  contest @k \"K\" single\n    campaign @nobody\n    recommend no-position\n")),
            ["`@nobody` isn't declared"]
        );
    }

    #[test]
    fn an_invalid_escape_names_the_escapes_a_string_has() {
        assert_eq!(
            messages(&VALID.replace("\"G\"", r#""a\#b""#)),
            [r#"`\#` isn't an escape, since only `\"` and `\\` are"#]
        );
    }

    #[test]
    fn an_unexpected_token_is_quoted_whole() {
        assert_eq!(
            messages(&VALID.replace("date 2026-11-03", "date \"November 3\" 2026")),
            ["`\"November 3\"` isn't expected here; expected a date"]
        );
    }

    #[test]
    fn a_missing_value_or_entry_names_what_was_expected() {
        assert_eq!(
            messages(&VALID.replace("date 2026-11-03", "date")),
            ["the line ends where a value is required; expected a date"]
        );
        assert_eq!(
            messages(&VALID.replace(
                "    recommend yes\n",
                "    recommend yes\n  website https://b.example\n"
            )),
            [
                "the entry doesn't belong in this block; expected `eligibility`, `note`, `section`, `contest`, `measure`, or `ballot`"
            ]
        );
    }

    #[test]
    fn a_missing_block_does_not_repeat_itself() {
        assert_eq!(
            messages(&VALID.replace(
                "    recommend yes\n",
                "    recommend yes\n  measure @n \"N\"\n"
            )),
            [
                "the question has no recommendation",
                "the line above needs an indented block"
            ]
        );
    }

    #[test]
    fn a_missing_guide_field_names_the_field() {
        assert_eq!(
            messages(
                &VALID
                    .replace("election \"E\"\n", "")
                    .replace("guide \"G\"\n", "")
            ),
            [
                "the document has no `guide` field",
                "the document has no `election` field"
            ]
        );
    }

    #[test]
    fn expected_alternatives_are_listed_in_a_sentence() {
        let date = Expected::Token(TokenKind::Date);
        let guide = Expected::Keyword(Keyword::Guide);
        let at = Expected::Token(TokenKind::AtSign);
        assert_eq!(super::expected(&[]), None);
        assert_eq!(super::expected(&[date]).as_deref(), Some("a date"));
        assert_eq!(
            super::expected(&[date, guide]).as_deref(),
            Some("a date or `guide`")
        );
        assert_eq!(
            super::expected(&[date, guide, at]).as_deref(),
            Some("a date, `guide`, or `@`")
        );
    }
}
