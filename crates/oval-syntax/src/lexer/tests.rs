use super::*;
use LexicalErrorKind::{
    IllegalTab, InvalidEscape, MalformedNumber, MalformedUrl, UnexpectedCharacter,
    UnterminatedString,
};
use TokenKind::{
    AtSign, BlankLine, ByteOrderMark, Colon, Comma, Comment, Date, Error, Integer, Newline, Url,
    Whitespace, Word,
};

fn check_lexer(text: &str, tokens: &[(TokenKind, &str)], errors: &[LexicalError]) {
    let result = lex(text).expect("the text is accepted");
    let mut pos = 0;
    let mut actual = Vec::new();
    for token in &result.tokens {
        let len = token.len as usize;
        actual.push((token.kind, &text[pos..pos + len]));
        pos += len;
    }
    assert_eq!(actual, tokens, "tokens of {text:?}");
    assert_eq!(pos, text.len(), "the tokens of {text:?} do not cover it");
    assert_eq!(result.errors, errors, "errors of {text:?}");
}

fn error(kind: LexicalErrorKind, offset: u32, len: u32) -> LexicalError {
    LexicalError { kind, offset, len }
}

#[test]
fn empty_input_has_no_tokens() {
    check_lexer("", &[], &[]);
}

#[test]
fn nul_rejects_the_whole_document() {
    assert_eq!(lex("a\0b"), Err(InputRejection::NulCharacter { offset: 1 }));
}

#[test]
fn rejection_reports_the_first_nul() {
    assert_eq!(
        lex("ab\0\0"),
        Err(InputRejection::NulCharacter { offset: 2 })
    );
}

#[cfg(target_pointer_width = "64")]
#[test]
fn offsets_past_u32_max_are_too_large() {
    let max = usize::try_from(u32::MAX).unwrap();
    assert_eq!(to_offset(max), Ok(u32::MAX));
    assert_eq!(to_offset(max + 1), Err(InputRejection::TooLarge));
}

#[test]
fn bytes_are_lexed_like_the_text_they_decode_to() {
    assert_eq!(lex_bytes("-é_".as_bytes()), lex("-é_"));
}

#[test]
fn invalid_utf8_rejects_the_whole_document() {
    assert_eq!(
        lex_bytes(b"ab\xffc"),
        Err(InputRejection::InvalidUtf8 { offset: 2 })
    );
}

#[test]
fn truncated_sequence_is_invalid_utf8() {
    assert_eq!(
        lex_bytes(b"a\xc3"),
        Err(InputRejection::InvalidUtf8 { offset: 1 })
    );
}

#[test]
fn encoded_surrogate_is_invalid_utf8() {
    assert_eq!(
        lex_bytes(b"\xed\xa0\x80"),
        Err(InputRejection::InvalidUtf8 { offset: 0 })
    );
}

#[test]
fn nul_in_bytes_rejects_the_whole_document() {
    assert_eq!(
        lex_bytes(b"a\0"),
        Err(InputRejection::NulCharacter { offset: 1 })
    );
}

#[test]
fn invalid_utf8_is_reported_before_nul() {
    assert_eq!(
        lex_bytes(b"\0\xff"),
        Err(InputRejection::InvalidUtf8 { offset: 1 })
    );
}

#[test]
fn each_unexpected_character_is_its_own_error_token() {
    // "é" is one character but two bytes in UTF-8.
    check_lexer(
        "-é_",
        &[(Error, "-"), (Error, "é"), (Error, "_"), (Newline, "")],
        &[
            error(UnexpectedCharacter, 0, 1),
            error(UnexpectedCharacter, 1, 2),
            error(UnexpectedCharacter, 3, 1),
        ],
    );
}

#[test]
fn errors_are_reported_in_source_order() {
    check_lexer(
        "- \"x",
        &[
            (Error, "-"),
            (Whitespace, " "),
            (Error, "\"x"),
            (Newline, ""),
        ],
        &[
            error(UnexpectedCharacter, 0, 1),
            error(UnterminatedString, 2, 2),
        ],
    );
}

#[test]
fn every_error_token_has_one_error_spanning_it() {
    for text in ["-é_", "a \"b\nc", "\u{FEFF}\u{FEFF}@-x", "# ok\n\"\\"] {
        let result = lex(text).unwrap();
        let mut pos = 0;
        let mut spans = Vec::new();
        for token in &result.tokens {
            if token.kind == Error {
                spans.push((pos, token.len));
            }
            pos += token.len;
        }
        let reported: Vec<(u32, u32)> = result
            .errors
            .iter()
            .map(|error| (error.offset, error.len))
            .collect();
        assert_eq!(reported, spans, "input: {text:?}");
    }
}

#[test]
fn leading_byte_order_mark_is_its_own_token() {
    check_lexer(
        "\u{FEFF}a",
        &[(ByteOrderMark, "\u{FEFF}"), (Word, "a"), (Newline, "")],
        &[],
    );
}

#[test]
fn byte_order_mark_elsewhere_is_an_error() {
    check_lexer(
        "a\u{FEFF}",
        &[(Word, "a"), (Error, "\u{FEFF}"), (Newline, "")],
        &[error(UnexpectedCharacter, 1, 3)],
    );
}

#[test]
fn only_the_first_byte_order_mark_is_ignored() {
    check_lexer(
        "\u{FEFF}\u{FEFF}",
        &[
            (ByteOrderMark, "\u{FEFF}"),
            (Error, "\u{FEFF}"),
            (Newline, ""),
        ],
        &[error(UnexpectedCharacter, 3, 3)],
    );
}

#[test]
fn byte_order_mark_survives_decoding() {
    assert_eq!(lex_bytes(b"\xef\xbb\xbfa"), lex("\u{FEFF}a"));
}

// Whitespace

#[test]
fn a_run_of_spaces_is_one_whitespace_token() {
    check_lexer("   ", &[(Whitespace, "   ")], &[]);
}

#[test]
fn whitespace_sits_between_other_tokens() {
    check_lexer(
        "a  b",
        &[(Word, "a"), (Whitespace, "  "), (Word, "b"), (Newline, "")],
        &[],
    );
}

#[test]
fn a_tab_is_not_whitespace() {
    check_lexer(
        " \t ",
        &[
            (Whitespace, " "),
            (Error, "\t"),
            (Whitespace, " "),
            (Newline, ""),
        ],
        &[error(IllegalTab, 1, 1)],
    );
}

#[test]
fn a_run_of_tabs_is_one_error() {
    check_lexer(
        "\t\t\ta",
        &[(Error, "\t\t\t"), (Word, "a"), (Newline, "")],
        &[error(IllegalTab, 0, 3)],
    );
}

#[test]
fn tabs_separated_by_a_space_are_two_errors() {
    check_lexer(
        "\t \t",
        &[
            (Error, "\t"),
            (Whitespace, " "),
            (Error, "\t"),
            (Newline, ""),
        ],
        &[error(IllegalTab, 0, 1), error(IllegalTab, 2, 1)],
    );
}

#[test]
fn a_tab_between_tokens_is_an_error() {
    // The specification's example for the illegal-tab error.
    check_lexer(
        "subtitle\t\"Vote for one\"",
        &[
            (Word, "subtitle"),
            (Error, "\t"),
            (TokenKind::String, "\"Vote for one\""),
            (Newline, ""),
        ],
        &[error(IllegalTab, 8, 1)],
    );
}

#[test]
fn a_tab_inside_a_string_is_part_of_it() {
    check_lexer(
        "\"a\tb\"",
        &[(TokenKind::String, "\"a\tb\""), (Newline, "")],
        &[],
    );
}

#[test]
fn a_tab_inside_a_comment_is_part_of_it() {
    check_lexer("# a\tb", &[(Comment, "# a\tb")], &[]);
}

#[test]
fn a_line_with_only_a_tab_ends_in_a_newline() {
    // An error is a token, so the line is not blank.
    check_lexer(
        "\t\n",
        &[(Error, "\t"), (Newline, "\n")],
        &[error(IllegalTab, 0, 1)],
    );
}

#[test]
fn a_comment_runs_to_the_end_of_the_line() {
    check_lexer("# a note", &[(Comment, "# a note")], &[]);
}

#[test]
fn a_comment_stops_before_a_line_terminator() {
    for terminator in ["\n", "\r", "\r\n"] {
        check_lexer(
            &format!("# a{terminator}b"),
            &[
                (Comment, "# a"),
                (BlankLine, terminator),
                (Word, "b"),
                (Newline, ""),
            ],
            &[],
        );
    }
}

#[test]
fn a_lone_hash_is_a_comment() {
    check_lexer("#", &[(Comment, "#")], &[]);
}

#[test]
fn a_comment_may_follow_other_tokens() {
    check_lexer(
        "a # x",
        &[
            (Word, "a"),
            (Whitespace, " "),
            (Comment, "# x"),
            (Newline, ""),
        ],
        &[],
    );
}

#[test]
fn a_comment_may_contain_anything_but_a_line_terminator() {
    check_lexer("#\t@\"é#", &[(Comment, "#\t@\"é#")], &[]);
}

#[test]
fn a_line_feed_after_tokens_is_a_newline() {
    check_lexer("a\n", &[(Word, "a"), (Newline, "\n")], &[]);
}

#[test]
fn a_line_feed_on_an_empty_line_is_a_blank_line() {
    check_lexer("\n", &[(BlankLine, "\n")], &[]);
}

#[test]
fn a_line_of_whitespace_and_a_comment_is_a_blank_line() {
    check_lexer(
        "  # x\n",
        &[(Whitespace, "  "), (Comment, "# x"), (BlankLine, "\n")],
        &[],
    );
}

#[test]
fn a_line_with_only_a_byte_order_mark_is_a_blank_line() {
    check_lexer(
        "\u{FEFF}\n",
        &[(ByteOrderMark, "\u{FEFF}"), (BlankLine, "\n")],
        &[],
    );
}

#[test]
fn a_comment_after_tokens_keeps_the_newline() {
    check_lexer(
        "a # x\n",
        &[
            (Word, "a"),
            (Whitespace, " "),
            (Comment, "# x"),
            (Newline, "\n"),
        ],
        &[],
    );
}

#[test]
fn a_line_with_an_error_ends_in_a_newline() {
    check_lexer(
        "-\n",
        &[(Error, "-"), (Newline, "\n")],
        &[error(UnexpectedCharacter, 0, 1)],
    );
}

#[test]
fn each_line_is_judged_on_its_own_tokens() {
    check_lexer(
        "a\n\nb\n",
        &[
            (Word, "a"),
            (Newline, "\n"),
            (BlankLine, "\n"),
            (Word, "b"),
            (Newline, "\n"),
        ],
        &[],
    );
}

#[test]
fn every_line_terminator_ends_a_line() {
    for terminator in ["\n", "\r\n", "\r"] {
        check_lexer(
            &format!("a{terminator}{terminator}"),
            &[(Word, "a"), (Newline, terminator), (BlankLine, terminator)],
            &[],
        );
    }
}

#[test]
fn a_carriage_return_and_line_feed_are_one_terminator() {
    check_lexer("\r\n", &[(BlankLine, "\r\n")], &[]);
}

#[test]
fn a_line_feed_before_a_carriage_return_is_two_terminators() {
    check_lexer("\n\r", &[(BlankLine, "\n"), (BlankLine, "\r")], &[]);
}

#[test]
fn a_carriage_return_before_a_crlf_is_two_terminators() {
    check_lexer(
        "a\r\r\n",
        &[(Word, "a"), (Newline, "\r"), (BlankLine, "\r\n")],
        &[],
    );
}

#[test]
fn terminators_may_be_mixed_in_one_document() {
    check_lexer(
        "a\r\nb\rc\n",
        &[
            (Word, "a"),
            (Newline, "\r\n"),
            (Word, "b"),
            (Newline, "\r"),
            (Word, "c"),
            (Newline, "\n"),
        ],
        &[],
    );
}

#[test]
fn a_comment_line_ending_in_crlf_is_a_blank_line() {
    check_lexer("# x\r\n", &[(Comment, "# x"), (BlankLine, "\r\n")], &[]);
}

#[test]
fn a_last_line_without_a_terminator_ends_in_an_empty_newline() {
    check_lexer("a", &[(Word, "a"), (Newline, "")], &[]);
}

#[test]
fn a_last_line_with_a_terminator_gets_no_extra_newline() {
    check_lexer("a\n", &[(Word, "a"), (Newline, "\n")], &[]);
}

#[test]
fn a_last_line_with_only_ignored_tokens_gets_no_newline() {
    check_lexer("  ", &[(Whitespace, "  ")], &[]);
    check_lexer("# c", &[(Comment, "# c")], &[]);
    check_lexer("\u{FEFF}", &[(ByteOrderMark, "\u{FEFF}")], &[]);
    check_lexer(
        "a\n  # c",
        &[
            (Word, "a"),
            (Newline, "\n"),
            (Whitespace, "  "),
            (Comment, "# c"),
        ],
        &[],
    );
}

#[test]
fn a_last_line_with_only_an_error_ends_in_an_empty_newline() {
    check_lexer(
        "-",
        &[(Error, "-"), (Newline, "")],
        &[error(UnexpectedCharacter, 0, 1)],
    );
}

#[test]
fn a_blank_last_line_gets_no_newline() {
    check_lexer(
        "a\n\n",
        &[(Word, "a"), (Newline, "\n"), (BlankLine, "\n")],
        &[],
    );
}

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

#[test]
fn a_keyword_is_a_word() {
    check_lexer("contest", &[(Word, "contest"), (Newline, "")], &[]);
}

#[test]
fn a_hyphenated_keyword_is_one_word() {
    check_lexer("no-position", &[(Word, "no-position"), (Newline, "")], &[]);
}

#[test]
fn a_word_continues_through_letters_digits_hyphens_and_underscores() {
    for text in ["if_yes", "x1", "A-b_C", "z-9_"] {
        check_lexer(text, &[(Word, text), (Newline, "")], &[]);
    }
}

#[test]
fn an_identifier_is_an_at_sign_and_a_word() {
    check_lexer(
        "@dana-lombardi",
        &[(AtSign, "@"), (Word, "dana-lombardi"), (Newline, "")],
        &[],
    );
}

#[test]
fn a_word_cannot_start_with_a_hyphen() {
    check_lexer(
        "@-lombardi",
        &[
            (AtSign, "@"),
            (Error, "-"),
            (Word, "lombardi"),
            (Newline, ""),
        ],
        &[error(UnexpectedCharacter, 1, 1)],
    );
}

#[test]
fn a_word_cannot_start_with_an_underscore() {
    check_lexer(
        "_x",
        &[(Error, "_"), (Word, "x"), (Newline, "")],
        &[error(UnexpectedCharacter, 0, 1)],
    );
}

#[test]
fn letters_in_a_word_are_ascii_only() {
    check_lexer(
        "café",
        &[(Word, "caf"), (Error, "é"), (Newline, "")],
        &[error(UnexpectedCharacter, 3, 2)],
    );
}

#[test]
fn a_word_cannot_start_with_a_digit() {
    // A run that starts with a digit is a number, so this is one malformed number.
    check_lexer(
        "9abc",
        &[(Error, "9abc"), (Newline, "")],
        &[error(MalformedNumber, 0, 4)],
    );
}

#[test]
fn digits_are_an_integer() {
    for text in ["0", "1", "42", "2026"] {
        check_lexer(text, &[(Integer, text), (Newline, "")], &[]);
    }
}

#[test]
fn leading_zeros_are_part_of_an_integer() {
    check_lexer("007", &[(Integer, "007"), (Newline, "")], &[]);
}

#[test]
fn a_date_has_the_shape_yyyy_mm_dd() {
    check_lexer("2026-11-03", &[(Date, "2026-11-03"), (Newline, "")], &[]);
}

#[test]
fn a_date_is_checked_for_its_shape_only() {
    // February 30 does not exist, but that is a validation error, not a lexical one.
    check_lexer("2026-02-30", &[(Date, "2026-02-30"), (Newline, "")], &[]);
}

#[test]
fn a_run_that_is_neither_an_integer_nor_a_date_is_one_malformed_number() {
    // The specification's examples of malformed numbers.
    for text in ["12abc", "3_000", "2026-11-5", "1.1", "1.x"] {
        let len = u32::try_from(text.len()).unwrap();
        check_lexer(
            text,
            &[(Error, text), (Newline, "")],
            &[error(MalformedNumber, 0, len)],
        );
    }
}

#[test]
fn a_date_with_the_wrong_shape_is_a_malformed_number() {
    for text in [
        "2026-11-3",
        "26-11-03",
        "2026-1103",
        "2026-11-033",
        "2026-11-03x",
    ] {
        let len = u32::try_from(text.len()).unwrap();
        check_lexer(
            text,
            &[(Error, text), (Newline, "")],
            &[error(MalformedNumber, 0, len)],
        );
    }
}

#[test]
fn a_number_stops_at_whitespace_and_punctuators() {
    check_lexer(
        "1 2,3:4@5",
        &[
            (Integer, "1"),
            (Whitespace, " "),
            (Integer, "2"),
            (Comma, ","),
            (Integer, "3"),
            (Colon, ":"),
            (Integer, "4"),
            (AtSign, "@"),
            (Integer, "5"),
            (Newline, ""),
        ],
        &[],
    );
}

#[test]
fn a_number_stops_before_a_character_outside_ascii() {
    check_lexer(
        "1é",
        &[(Integer, "1"), (Error, "é"), (Newline, "")],
        &[error(UnexpectedCharacter, 1, 2)],
    );
}

#[test]
fn the_version_marker_is_a_word_and_an_integer() {
    check_lexer(
        "oval 1\n",
        &[
            (Word, "oval"),
            (Whitespace, " "),
            (Integer, "1"),
            (Newline, "\n"),
        ],
        &[],
    );
}

#[test]
fn a_date_field() {
    // The specification's example for the malformed-number error.
    check_lexer(
        "date 2026-11-03\n",
        &[
            (Word, "date"),
            (Whitespace, " "),
            (Date, "2026-11-03"),
            (Newline, "\n"),
        ],
        &[],
    );
}

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

#[test]
fn words_are_separated_by_whitespace_and_punctuators() {
    check_lexer(
        "a b,c:d",
        &[
            (Word, "a"),
            (Whitespace, " "),
            (Word, "b"),
            (Comma, ","),
            (Word, "c"),
            (Colon, ":"),
            (Word, "d"),
            (Newline, ""),
        ],
        &[],
    );
}

#[test]
fn a_line_with_a_word_ends_in_a_newline() {
    check_lexer("guide\n", &[(Word, "guide"), (Newline, "\n")], &[]);
}

#[test]
fn a_whole_string_is_one_token() {
    for text in [
        r#""Vote for one""#,
        r#""""#,
        r#""a \"b\" c""#,
        r#""a \\ b""#,
        r##""# @ : , é""##,
    ] {
        check_lexer(text, &[(TokenKind::String, text), (Newline, "")], &[]);
    }
}

#[test]
fn an_escaped_backslash_before_the_closing_quote_does_not_escape_it() {
    check_lexer(
        r#""a\\" x"#,
        &[
            (TokenKind::String, r#""a\\""#),
            (Whitespace, " "),
            (Word, "x"),
            (Newline, ""),
        ],
        &[],
    );
}

#[test]
fn an_invalid_escape_is_still_a_string() {
    // The specification's example for the invalid-escape error: the error spans the backslash
    // and the character after it, and the token is still a String.
    check_lexer(
        r#""Measure \#3""#,
        &[(TokenKind::String, r#""Measure \#3""#), (Newline, "")],
        &[error(InvalidEscape, 9, 2)],
    );
}

#[test]
fn any_character_but_a_quote_or_backslash_makes_an_invalid_escape() {
    // A backslash and "n" is not a line break in Oval, just an invalid escape.
    check_lexer(
        r#""a\nb""#,
        &[(TokenKind::String, r#""a\nb""#), (Newline, "")],
        &[error(InvalidEscape, 2, 2)],
    );
}

#[test]
fn an_invalid_escape_spans_a_whole_multibyte_character() {
    // "é" is two bytes, so the escape is three.
    check_lexer(
        r#""\é""#,
        &[(TokenKind::String, r#""\é""#), (Newline, "")],
        &[error(InvalidEscape, 1, 3)],
    );
}

#[test]
fn each_invalid_escape_is_reported() {
    check_lexer(
        r#""\a\b""#,
        &[(TokenKind::String, r#""\a\b""#), (Newline, "")],
        &[error(InvalidEscape, 1, 2), error(InvalidEscape, 3, 2)],
    );
}

#[test]
fn an_invalid_escape_is_reported_where_it_is() {
    check_lexer(
        r#"x "\#""#,
        &[
            (Word, "x"),
            (Whitespace, " "),
            (TokenKind::String, r#""\#""#),
            (Newline, ""),
        ],
        &[error(InvalidEscape, 3, 2)],
    );
}

#[test]
fn an_unterminated_string_reports_no_invalid_escapes() {
    check_lexer(
        "\"\\#\n",
        &[(Error, "\"\\#"), (Newline, "\n")],
        &[error(UnterminatedString, 0, 3)],
    );
}

#[test]
fn an_unterminated_string_is_an_error_up_to_the_line_terminator() {
    check_lexer(
        "\"November 3\ndate",
        &[
            (Error, "\"November 3"),
            (Newline, "\n"),
            (Word, "date"),
            (Newline, ""),
        ],
        &[error(UnterminatedString, 0, 11)],
    );
}

#[test]
fn an_unterminated_string_stops_before_a_carriage_return() {
    check_lexer(
        "\"a\r\n",
        &[(Error, "\"a"), (Newline, "\r\n")],
        &[error(UnterminatedString, 0, 2)],
    );
}

#[test]
fn an_unterminated_string_may_end_at_the_end_of_input() {
    check_lexer(
        "\"abc",
        &[(Error, "\"abc"), (Newline, "")],
        &[error(UnterminatedString, 0, 4)],
    );
}

#[test]
fn an_unterminated_string_is_reported_where_it_starts() {
    check_lexer(
        "a \"b",
        &[
            (Word, "a"),
            (Whitespace, " "),
            (Error, "\"b"),
            (Newline, ""),
        ],
        &[error(UnterminatedString, 2, 2)],
    );
}

#[test]
fn a_backslash_before_a_line_terminator_is_part_of_an_unterminated_string() {
    check_lexer(
        "\"x\\\n",
        &[(Error, "\"x\\"), (Newline, "\n")],
        &[error(UnterminatedString, 0, 3)],
    );
}

#[test]
fn a_backslash_at_the_end_of_input_is_part_of_an_unterminated_string() {
    check_lexer(
        "\"x\\",
        &[(Error, "\"x\\"), (Newline, "")],
        &[error(UnterminatedString, 0, 3)],
    );
}

#[test]
fn a_field_with_a_string_value() {
    check_lexer(
        r#"subtitle "Vote for one""#,
        &[
            (Word, "subtitle"),
            (Whitespace, " "),
            (TokenKind::String, r#""Vote for one""#),
            (Newline, ""),
        ],
        &[],
    );
}

#[test]
fn token_lengths_cover_the_input() {
    for text in [
        "",
        "-é_",
        "ballot @city \"City\"\n",
        "  # comment\r\n",
        "日本語",
        "\u{FEFF}oval 1",
        "  a   b \t  ",
        "contest @mayor \"Mayor\" single\n  campaign @bsantos\n",
        "a \"b \\\" c\" \"unterminated\n\"x\\",
    ] {
        let total: usize = lex(text)
            .unwrap()
            .tokens
            .iter()
            .map(|token| token.len as usize)
            .sum();
        assert_eq!(total, text.len(), "input: {text:?}");
    }
}
