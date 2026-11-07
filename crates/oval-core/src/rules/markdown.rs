use oval_syntax::TextRange;
use oval_syntax::ast::{AstNode, Document, MarkdownBody, MarkdownField};
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag};

use super::report;
use crate::{Diagnostic, ValidationRule};

pub(crate) fn check(document: &Document, out: &mut Vec<Diagnostic>) {
    for field in document
        .syntax()
        .descendants()
        .filter_map(MarkdownField::cast)
    {
        if let Some(body) = field.body() {
            profile(&body, out);
        }
    }
}

fn profile(body: &MarkdownBody, out: &mut Vec<Diagnostic>) {
    let value = body.value();
    let parser = Parser::new_ext(value, Options::ENABLE_TABLES);
    let definitions: Vec<(String, std::ops::Range<usize>)> = parser
        .reference_definitions()
        .iter()
        .map(|(_, definition)| (definition.dest.to_string(), definition.span.clone()))
        .collect();
    for (event, range) in parser.into_offset_iter() {
        let message = match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                "an indented code block is not part of the Markdown profile; indent this text less"
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) | Event::Code(_) => {
                "code is not part of the Markdown profile"
            }
            Event::Start(Tag::HtmlBlock) | Event::Html(_) | Event::InlineHtml(_) => {
                "raw HTML is not part of the Markdown profile"
            }
            Event::Start(Tag::Image { .. }) => "images are not part of the Markdown profile",
            Event::Rule => "thematic breaks are not part of the Markdown profile",
            Event::Start(Tag::Link {
                link_type: LinkType::Inline | LinkType::Autolink | LinkType::Email,
                dest_url,
                ..
            }) => {
                let destination = destination_range(value, &range, &dest_url);
                destination_check(&dest_url, body.source_range(destination), out);
                continue;
            }
            _ => continue,
        };
        out.push(report(
            ValidationRule::UnsupportedMarkdown,
            body.source_range(range),
            message.to_owned(),
        ));
    }
    for (destination, span) in definitions {
        let range = destination_range(value, &span, &destination);
        destination_check(&destination, body.source_range(range), out);
    }
}

fn destination_range(
    value: &str,
    span: &std::ops::Range<usize>,
    destination: &str,
) -> std::ops::Range<usize> {
    value[span.clone()]
        .rfind(destination)
        .filter(|_| !destination.is_empty())
        .map_or(span.clone(), |i| {
            span.start + i..span.start + i + destination.len()
        })
}

fn destination_check(destination: &str, range: TextRange, out: &mut Vec<Diagnostic>) {
    if destination.starts_with("https://") {
        return;
    }
    if destination.starts_with("http://") {
        out.push(report(
            ValidationRule::InsecureUrl,
            range,
            "this link uses `http`; use `https`".to_owned(),
        ));
    } else {
        out.push(report(
            ValidationRule::UnsupportedMarkdown,
            range,
            "a link destination must be an absolute `http` or `https` URL".to_owned(),
        ));
    }
}
