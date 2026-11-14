use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, AstToken, Document, MarkdownBody};
use pulldown_cmark::{Event, Options, Parser, Tag};

use super::{diagnostic, kept_tokens};
use crate::{Diagnostic, ValidationRule};

pub(super) fn markdown_profile(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
    for body in kept_tokens(document.syntax()).filter_map(MarkdownBody::cast) {
        let value = body.value();
        for (event, range) in Parser::new_ext(&value, Options::ENABLE_TABLES).into_offset_iter() {
            let outside = match &event {
                Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::Image { .. })
                | Event::Code(_)
                | Event::InlineHtml(_)
                | Event::Rule => true,
                Event::Start(Tag::Link { dest_url, .. }) => !is_absolute_http(dest_url),
                _ => false,
            };
            if !outside {
                continue;
            }
            let construct = &value[range.clone()];
            let start =
                range.start + (construct.len() - construct.trim_start_matches(is_space).len());
            let end = range.start + construct.trim_end_matches(is_space).len();
            if start < end {
                diagnostics.push(diagnostic(
                    ValidationRule::MarkdownProfile,
                    TextRange::new(body.source_offset(start), body.source_offset(end)),
                ));
            }
        }
    }
}

fn is_absolute_http(destination: &str) -> bool {
    ["http://", "https://"].iter().any(|scheme| {
        destination
            .strip_prefix(scheme)
            .is_some_and(|rest| !rest.is_empty())
    })
}

fn is_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n')
}

#[cfg(test)]
mod tests {
    use crate::validate::{reported, reported_as};

    const BODIES: &str = "oval 1\nguide \"G\"\nelection \"E\"\ndate 2026-11-03\nnote:\n  Text `span` and ![alt](https://x.example/i.png) and <b>b</b>.\n\n  [ok](https://ok.example) [plain](http://x.example) [rel](/rel) [ref][d] <me@x.example> [up](HTTPS://Z) [empty]() [bare](https://)\n\n      indented code\n\n  ***\n\n  [d]: ftp://z\nnote:\n  `set aside`\nballot @b \"B\"\n  note:\n    ```\n    fenced\n    ```\n  measure @m \"M\"\n    recommend yes\n";

    fn constructs(text: &str) -> Vec<&str> {
        reported_as(text, "markdown-profile")
            .into_iter()
            .map(|(start, end)| &text[start as usize..end as usize])
            .collect()
    }

    #[test]
    fn code_html_images_and_thematic_breaks_are_outside_the_profile() {
        let found = constructs(BODIES);
        assert_eq!(
            found
                .iter()
                .filter(|construct| !construct.starts_with('['))
                .collect::<Vec<_>>(),
            [
                &"`span`",
                &"![alt](https://x.example/i.png)",
                &"<b>",
                &"</b>",
                &"<me@x.example>",
                &"indented code",
                &"***",
                &"```\n    fenced\n    ```",
            ]
        );
    }

    #[test]
    fn a_link_needs_an_absolute_lowercase_http_or_https_destination() {
        let links: Vec<&str> = constructs(BODIES)
            .into_iter()
            .filter(|construct| construct.starts_with('['))
            .collect();
        assert_eq!(
            links,
            [
                "[rel](/rel)",
                "[ref][d]",
                "[up](HTTPS://Z)",
                "[empty]()",
                "[bare](https://)",
            ]
        );
    }

    #[test]
    fn a_construct_spans_its_text_in_the_document() {
        assert_eq!(
            reported_as(BODIES, "markdown-profile")[..2],
            [(59, 65), (70, 101)]
        );
        let text = "oval 1\r\nguide \"G\"\r\nelection \"E\"\r\ndate 2026-11-03\r\nnote:\r\n  A\r\n  `b` <i>\r\nballot @b \"B\"\r\n  measure @m \"M\"\r\n    recommend yes\r\n";
        assert_eq!(
            reported(text),
            [("markdown-profile", 64, 67), ("markdown-profile", 68, 71)]
        );
    }

    #[test]
    fn a_body_set_aside_is_not_checked() {
        assert_eq!(reported_as(BODIES, "repeated-field"), [(293, 313)]);
        assert!(!constructs(BODIES).contains(&"`set aside`"));
    }
}
