//! Lint rules: advice the specification does not require. Each rule has a
//! stable kebab-case code and a default level; `oval.toml` overrides levels.
//!
//! ```toml
//! [lint]
//! no-rationale = "off"
//! candidate-without-designation = "warn"
//! ```

use oval_model::{Candidate, Loaded, Node, Position};
use oval_syntax::inline::Inline;
use oval_syntax::{Diagnostic, Diagnostics};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Off,
    Warn,
    Error,
}

pub struct Rule {
    pub code: &'static str,
    pub default: Level,
    pub summary: &'static str,
}

pub const RULES: &[Rule] = &[
    Rule {
        code: "no-rationale",
        default: Level::Warn,
        summary: "a decided contest, measure, or retention has no rationale",
    },
    Rule { code: "missing-yes", default: Level::Warn, summary: "a measure has no `yes:` summary" },
    Rule { code: "empty-section", default: Level::Warn, summary: "a container section has nothing in it" },
    Rule {
        code: "unverified-in-note",
        default: Level::Warn,
        summary: "a note says the list is unverified; use the `unverified` tag",
    },
    Rule {
        code: "avoid-without-rationale",
        default: Level::Warn,
        summary: "a candidate is marked avoid but nothing explains why",
    },
    Rule {
        code: "candidate-without-designation",
        default: Level::Off,
        summary: "a candidate has neither a party nor a designation",
    },
];

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub lint: BTreeMap<String, Level>,
}

impl Config {
    pub fn level(&self, code: &str) -> Level {
        self.lint
            .get(code)
            .copied()
            .unwrap_or_else(|| RULES.iter().find(|r| r.code == code).map_or(Level::Off, |r| r.default))
    }

    /// Parse `oval.toml` text.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        let c: Config = toml::from_str(text)?;
        Ok(c)
    }

    /// Find and load the nearest `oval.toml` at or above `start`. Returns
    /// the default config when there is none; an unreadable or invalid file
    /// is an error.
    pub fn discover(start: &Path) -> Result<(Self, Option<std::path::PathBuf>), String> {
        let mut dir = if start.is_dir() { Some(start) } else { start.parent() };
        while let Some(d) = dir {
            let p = d.join("oval.toml");
            if p.is_file() {
                let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                let c = Self::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
                return Ok((c, Some(p)));
            }
            dir = d.parent();
        }
        Ok((Self::default(), None))
    }
}

pub fn lint(loaded: &Loaded, config: &Config) -> Vec<Diagnostic> {
    let mut l = Linter { config, out: Diagnostics::new() };
    for n in &loaded.guide.children {
        l.node(n);
    }
    l.out.sort();
    l.out.into_vec()
}

struct Linter<'a> {
    config: &'a Config,
    out: Diagnostics,
}

impl Linter<'_> {
    fn report(&mut self, code: &'static str, range: oval_syntax::TextRange, msg: impl Into<String>) {
        match self.config.level(code) {
            Level::Off => {}
            Level::Warn => self.out.warning(range, code, msg),
            Level::Error => self.out.error(range, code, msg),
        }
    }

    fn node(&mut self, n: &Node) {
        let line = n.span().line;
        match n {
            Node::Section(s) => {
                if s.children.is_empty() {
                    self.report(
                        "empty-section",
                        line,
                        "this section has no contests, measures, or sections in it",
                    );
                }
                for c in &s.children {
                    self.node(c);
                }
            }
            Node::Contest(c) => {
                let decided = c.decision == oval_model::Decision::Decided;
                if decided && c.rationale.is_empty() {
                    self.report("no-rationale", line, "this contest has picks but no rationale");
                }
                if c.rationale.is_empty() && c.candidates.iter().any(|x| x.position == Position::Avoid) {
                    self.report(
                        "avoid-without-rationale",
                        line,
                        "a candidate is marked `-` but no rationale says why",
                    );
                }
                if !c.unverified && c.note.as_ref().is_some_and(|n| mentions_unverified(n)) {
                    self.report(
                        "unverified-in-note",
                        line,
                        "the note says the list is unverified; add the `unverified` tag instead",
                    );
                }
                for cand in &c.candidates {
                    self.candidate(cand);
                }
            }
            Node::Measure(m) => {
                if m.yes.is_none() {
                    self.report("missing-yes", line, "say what a yes vote does with `yes:`");
                }
                if matches!(m.vote, oval_model::Vote::Yes | oval_model::Vote::No) && m.rationale.is_empty() {
                    self.report("no-rationale", line, "this measure has a position but no rationale");
                }
                if !m.unverified && m.note.as_ref().is_some_and(|n| mentions_unverified(n)) {
                    self.report(
                        "unverified-in-note",
                        line,
                        "the note says this is unverified; add the `unverified` tag instead",
                    );
                }
            }
            Node::Retention(r) => {
                if matches!(r.vote, oval_model::Vote::Yes | oval_model::Vote::No) && r.rationale.is_empty() {
                    self.report("no-rationale", line, "this retention has a position but no rationale");
                }
            }
        }
    }

    fn candidate(&mut self, c: &Candidate) {
        if c.party.is_none() && c.designation.is_none() {
            self.report(
                "candidate-without-designation",
                c.span.line,
                format!("`{}` has no party or designation", c.name),
            );
        }
    }
}

fn mentions_unverified(inline: &[Inline]) -> bool {
    let mut text = String::new();
    flatten(inline, &mut text);
    let t = text.to_ascii_lowercase();
    t.contains("not yet verified") || t.contains("unverified") || t.contains("not verified")
}

fn flatten(nodes: &[Inline], out: &mut String) {
    for n in nodes {
        match n {
            Inline::Text { text } | Inline::Code { text } => out.push_str(text),
            Inline::Emphasis { children }
            | Inline::Strong { children }
            | Inline::Link { children, .. }
            | Inline::Ref { children, .. } => flatten(children, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_overrides_defaults() {
        let c = Config::parse("[lint]\nno-rationale = \"off\"\ncandidate-without-designation = \"error\"\n")
            .unwrap();
        assert_eq!(c.level("no-rationale"), Level::Off);
        assert_eq!(c.level("candidate-without-designation"), Level::Error);
        assert_eq!(c.level("missing-yes"), Level::Warn);
        assert!(Config::parse("[lint]\nno-rationale = \"loud\"\n").is_err());
    }

    #[test]
    fn rules_fire() {
        let src = "oval 1\nname: T\ndate: 2026-11-03\nauthor: A\n\n# P @p\n\n## Empty @e\n\n## Race @r\ncontest\nnote: Candidate list not yet verified.\n\n+ Al Able\n- Bo Baker\n\n## M @m\nmeasure\nvote: yes\n";
        let loaded = oval_model::load(src);
        let out = lint(&loaded, &Config::default());
        let codes: Vec<_> = out.iter().map(|d| d.code).collect();
        assert_eq!(
            codes,
            [
                "empty-section",
                "no-rationale",
                "avoid-without-rationale",
                "unverified-in-note",
                "missing-yes",
                "no-rationale"
            ]
        );
    }
}
