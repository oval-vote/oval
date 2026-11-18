use oval_analysis::{CompletionKind, OutlineItem, OutlineKind};
use oval_core::{Declaration, Entity};
use oval_syntax::ast::AstNode;
use serde::Serialize;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::utf16::{Range, Utf16Index};
use crate::{Diagnostic, rejected, to_js};

/// A checked document, ready to answer an editor's questions about it.
///
/// Exported to JavaScript as the class `Analysis`, made with `new Analysis(source)`. The document
/// is checked once, so an editor can keep one for each version of a document and ask it many
/// questions. A document with errors still has answers. Positions given to its methods are string
/// indices, and an index between the halves of a surrogate pair means the character they make up.
///
/// Its memory belongs to WebAssembly. The garbage collector frees it in time, and `free()` frees it
/// at once.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Analysis {
    analysis: oval_analysis::Analysis,
    index: Utf16Index,
}

#[wasm_bindgen]
impl Analysis {
    /// Checks `source`.
    ///
    /// Throws an `Error` when the source can't be an Oval document at all, because it contains
    /// U+0000 or is too large.
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str) -> Result<Self, JsValue> {
        let index = Utf16Index::new(source);
        let analysis = oval_analysis::Analysis::new(source)
            .map_err(|rejection| rejected(rejection, &index))?;
        Ok(Self { analysis, index })
    }

    /// Returns every problem found in the document, as `check` does.
    #[wasm_bindgen(unchecked_return_type = "Diagnostic[]")]
    pub fn diagnostics(&self) -> Result<JsValue, JsValue> {
        to_js(&Diagnostic::all(self.analysis.report(), &self.index))
    }

    /// Returns the document's ballots as a tree of ballots, sections, contests, and measures.
    ///
    /// Parties and candidates aren't in it, since they're declarations rather than places on a
    /// ballot.
    #[wasm_bindgen(unchecked_return_type = "OutlineItem[]")]
    pub fn outline(&self) -> Result<JsValue, JsValue> {
        to_js(&self.outline_items())
    }

    /// Returns the declaration the identifier at `index` names, or `null` when there's no
    /// identifier there or nothing declares its name.
    ///
    /// An index just past an identifier's end still counts, so a cursor right after `@dem` finds
    /// it.
    #[wasm_bindgen(unchecked_return_type = "Definition | null")]
    pub fn definition(&self, index: u32) -> Result<JsValue, JsValue> {
        to_js(&self.definition_at(index))
    }

    /// Returns every place the name of the identifier at `index` is written, declarations and
    /// references alike, in document order.
    ///
    /// It's empty when there's no identifier at `index`.
    #[wasm_bindgen(unchecked_return_type = "Occurrence[]")]
    pub fn occurrences(&self, index: u32) -> Result<JsValue, JsValue> {
        to_js(&self.occurrences_at(index))
    }

    /// Describes the identifier or keyword at `index`, or returns `null` when there's neither.
    #[wasm_bindgen(unchecked_return_type = "Hover | null")]
    pub fn hover(&self, index: u32) -> Result<JsValue, JsValue> {
        to_js(&self.hover_at(index))
    }

    /// Returns everything the grammar accepts at `index`, keywords and identifiers alike.
    ///
    /// The word before `index` is treated as unfinished, and the rest of the line as not yet
    /// written, so the completions are what could begin there. An editor filters them by the text
    /// in each one's `replace`.
    #[wasm_bindgen(unchecked_return_type = "Completion[]")]
    pub fn completions(&self, index: u32) -> Result<JsValue, JsValue> {
        to_js(&self.completions_at(index))
    }

    /// Returns a fix for each diagnostic that has a mechanical repair, in the order of the
    /// diagnostics.
    #[wasm_bindgen(unchecked_return_type = "Fix[]")]
    pub fn fixes(&self) -> Result<JsValue, JsValue> {
        to_js(&self.fix_all())
    }
}

impl Analysis {
    fn outline_items(&self) -> Vec<Outline> {
        self.analysis
            .outline()
            .iter()
            .map(|item| Outline::new(item, &self.index))
            .collect()
    }

    fn definition_at(&self, index: u32) -> Option<Definition> {
        let declaration = self.analysis.definition(self.index.offset(index))?;
        Definition::new(declaration, &self.index)
    }

    fn occurrences_at(&self, index: u32) -> Vec<Occurrence> {
        self.analysis
            .occurrences(self.index.offset(index))
            .into_iter()
            .map(|occurrence| Occurrence {
                range: self.index.range(occurrence.range),
                id_range: self.index.range(occurrence.name),
                is_declaration: occurrence.is_declaration,
            })
            .collect()
    }

    fn completions_at(&self, index: u32) -> Vec<Completion> {
        self.analysis
            .completions(self.index.offset(index))
            .into_iter()
            .map(|completion| Completion {
                label: completion.label,
                kind: match completion.kind {
                    CompletionKind::Keyword => "keyword",
                    CompletionKind::Identifier(declared) => entity(declared),
                },
                detail: completion.detail,
                replace: self.index.range(completion.replace),
            })
            .collect()
    }

    fn fix_all(&self) -> Vec<Fix> {
        self.analysis
            .fixes()
            .into_iter()
            .map(|fix| {
                let text = Utf16Index::new(&fix.edit.text);
                Fix {
                    diagnostic: fix.diagnostic,
                    title: fix.title,
                    edit: Edit {
                        range: self.index.range(fix.edit.range),
                        placeholder: fix
                            .edit
                            .placeholder
                            .map(|placeholder| text.range(placeholder)),
                        text: fix.edit.text,
                    },
                }
            })
            .collect()
    }

    fn hover_at(&self, index: u32) -> Option<Hover> {
        let hover = self.analysis.hover(self.index.offset(index))?;
        Some(Hover {
            range: self.index.range(hover.range),
            contents: hover.contents,
        })
    }
}

#[derive(Debug, PartialEq, Serialize)]
struct Outline {
    kind: &'static str,
    id: Option<String>,
    title: Option<String>,
    range: Range,
    selection: Range,
    children: Vec<Outline>,
}

impl Outline {
    fn new(item: &OutlineItem, index: &Utf16Index) -> Self {
        Self {
            kind: match item.kind {
                OutlineKind::Ballot => "ballot",
                OutlineKind::Section => "section",
                OutlineKind::Contest => "contest",
                OutlineKind::Measure => "measure",
            },
            id: item.id.clone(),
            title: item.title.clone(),
            range: index.range(item.range),
            selection: index.range(item.selection),
            children: item
                .children
                .iter()
                .map(|child| Self::new(child, index))
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    entity: &'static str,
    id: String,
    name: Option<String>,
    range: Range,
    id_range: Range,
}

impl Definition {
    fn new(declaration: &Declaration, index: &Utf16Index) -> Option<Self> {
        let identifier = &declaration.identifier;
        let id = identifier.name()?;
        Some(Self {
            entity: entity(declaration.entity),
            id: id.text().to_owned(),
            name: declaration.name().map(|name| name.value().into_owned()),
            range: index.range(identifier.syntax().text_range()),
            id_range: index.range(id.text_range()),
        })
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Occurrence {
    range: Range,
    id_range: Range,
    is_declaration: bool,
}

#[derive(Debug, PartialEq, Serialize)]
struct Hover {
    range: Range,
    contents: String,
}

#[derive(Debug, PartialEq, Serialize)]
struct Completion {
    label: String,
    kind: &'static str,
    detail: Option<String>,
    replace: Range,
}

#[derive(Debug, PartialEq, Serialize)]
struct Fix {
    diagnostic: usize,
    title: String,
    edit: Edit,
}

#[derive(Debug, PartialEq, Serialize)]
struct Edit {
    range: Range,
    text: String,
    placeholder: Option<Range>,
}

fn entity(entity: Entity) -> &'static str {
    match entity {
        Entity::Party => "party",
        Entity::Candidate => "candidate",
        Entity::Ballot => "ballot",
        Entity::Contest => "contest",
        Entity::Measure => "measure",
    }
}

#[cfg(test)]
mod tests {
    use super::{Analysis, Completion, Definition, Edit, Hover, Occurrence, Outline};
    use crate::utf16::Range;

    const SOURCE: &str = "oval 1\nguide \"🗳 Guía\"\nparty @dem \"Demócrata\"\ncandidate @n \"Nadia\"\n  party @dem\nballot @b \"Ciudad\"\n  section \"Óficinas\"\n    contest @k \"Alcalde\" single\n      campaign @n\n      recommend\n        support @n\n";

    fn index(needle: &str, nth: usize) -> u32 {
        let offset = SOURCE.match_indices(needle).nth(nth).unwrap().0;
        u32::try_from(SOURCE[..offset].encode_utf16().count()).unwrap()
    }

    fn range(needle: &str, nth: usize) -> Range {
        let start = index(needle, nth);
        let units = u32::try_from(needle.encode_utf16().count()).unwrap();
        Range {
            start,
            end: start + units,
        }
    }

    fn analysis() -> Analysis {
        Analysis::new(SOURCE).unwrap()
    }

    #[test]
    fn a_definition_names_its_declaration_in_string_indices() {
        assert_eq!(
            analysis().definition_at(index("@dem", 1)),
            Some(Definition {
                entity: "party",
                id: "dem".to_owned(),
                name: Some("Demócrata".to_owned()),
                range: range("@dem", 0),
                id_range: range("dem", 0),
            })
        );
        assert_eq!(analysis().definition_at(index("Nadia", 0)), None);
    }

    #[test]
    fn occurrences_are_every_place_a_name_is_written() {
        assert_eq!(
            analysis().occurrences_at(index("@n", 2) + 1),
            [0, 1, 2].map(|nth| Occurrence {
                range: range("@n", nth),
                id_range: Range {
                    start: range("@n", nth).start + 1,
                    end: range("@n", nth).end,
                },
                is_declaration: nth == 0,
            })
        );
    }

    #[test]
    fn a_hover_spans_what_it_describes() {
        assert_eq!(
            analysis().hover_at(index("single", 0)),
            Some(Hover {
                range: range("single", 0),
                contents: "`single` makes the contest a vote for one choice.".to_owned(),
            })
        );
    }

    #[test]
    fn the_outline_is_in_string_indices() {
        let outline = analysis().outline_items();
        let section = &outline[0].children[0];
        assert_eq!(
            section,
            &Outline {
                kind: "section",
                id: None,
                title: Some("Óficinas".to_owned()),
                range: Range {
                    start: index("section", 0),
                    end: index("support @n\n", 0) + 11,
                },
                selection: range("\"Óficinas\"", 0),
                children: vec![Outline {
                    kind: "contest",
                    id: Some("k".to_owned()),
                    title: Some("Alcalde".to_owned()),
                    range: Range {
                        start: index("contest", 0),
                        end: index("support @n\n", 0) + 11,
                    },
                    selection: range("@k", 0),
                    children: Vec::new(),
                }],
            }
        );
    }

    #[test]
    fn a_completion_replaces_the_word_before_the_index() {
        let source = format!("{SOURCE}        support @");
        let analysis = Analysis::new(&source).unwrap();
        let end = u32::try_from(source.encode_utf16().count()).unwrap();
        assert_eq!(
            analysis.completions_at(end),
            [Completion {
                label: "@n".to_owned(),
                kind: "candidate",
                detail: Some("Nadia".to_owned()),
                replace: Range {
                    start: end - 1,
                    end,
                },
            }]
        );
        assert_eq!(
            analysis.completions_at(index("single", 0) + 3)[0].kind,
            "keyword"
        );
    }

    #[test]
    fn a_fix_edits_the_document_and_marks_a_placeholder_in_its_own_text() {
        let source = "# Guía 🗳\noval 1\nguide \"G\"\ndate 2026-11-03\n";
        let analysis = Analysis::new(source).unwrap();
        let fixes = analysis.fix_all();
        let missing = fixes
            .iter()
            .find(|fix| fix.title.contains("election"))
            .unwrap();
        let after_guide = source.find("date").unwrap();
        let start = u32::try_from(source[..after_guide].encode_utf16().count()).unwrap();
        assert_eq!(
            missing.edit,
            Edit {
                range: Range { start, end: start },
                text: "election \"\"\n".to_owned(),
                placeholder: Some(Range { start: 10, end: 10 }),
            }
        );
        assert_eq!(
            analysis.analysis.report().diagnostics()[missing.diagnostic]
                .code
                .spec_id(),
            "missing-guide-field"
        );
    }
}
