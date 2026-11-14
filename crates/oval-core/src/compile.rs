use oval_syntax::ast::{AstToken, Document, MarkdownField, StringField};

use crate::model::{Compiled, Guide};

pub(crate) fn compile(document: &Document) -> Option<Compiled> {
    Some(Compiled {
        oval: document.version_marker()?.version()?.value()?,
        guide: Guide {
            name: string(document.guide())?,
            election: string(document.election())?,
            date: document.date()?.value()?.text().to_owned(),
            note: markdown(document.note()),
        },
    })
}

fn string(field: Option<StringField>) -> Option<String> {
    Some(field?.value()?.value().into_owned())
}

fn markdown(field: Option<MarkdownField>) -> Option<String> {
    Some(field?.body()?.value())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::check;
    use crate::validate::VALID;

    fn compiled(text: &str) -> serde_json::Value {
        let compiled = check(text)
            .unwrap()
            .compile()
            .expect("the document compiles");
        serde_json::from_str(&compiled.to_json()).unwrap()
    }

    #[test]
    fn the_guide_fields_compile_with_their_values() {
        let text = VALID
            .replacen("\"G\"", "\"Franklin \\\"Votes\\\"\"", 1)
            .replacen(
                "date 2026-11-03\n",
                "date 2026-11-03\nnote:\n    Vote *early*.\n\n      Really.\n",
                1,
            );
        assert_eq!(
            compiled(&text),
            json!({
                "oval": 1,
                "guide": {
                    "name": "Franklin \"Votes\"",
                    "election": "E",
                    "date": "2026-11-03",
                    "note": "Vote *early*.\n\n  Really.",
                },
            })
        );
    }

    #[test]
    fn a_note_the_author_did_not_write_is_left_out() {
        assert_eq!(
            compiled(VALID)["guide"],
            json!({ "name": "G", "election": "E", "date": "2026-11-03" })
        );
    }
}
