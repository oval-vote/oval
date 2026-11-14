use crate::Severity;

/// A rule a well-formed document can break, one for each validation error and warning in the
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationRule {
    /// The document has no version marker, or its marker names no major version or one this
    /// implementation does not support.
    ///
    /// A document that breaks this rule is reported with this diagnostic alone, since the rest of
    /// it may not mean what this version of the language says.
    UnsupportedVersion,
    /// `guide`, `election`, or `date` is absent at the document level. Each absent field is its own
    /// diagnostic.
    MissingGuideField,
    /// The document has no root ballot.
    EmptyDocument,
    /// A date does not exist on the calendar, such as `2026-02-30`.
    InvalidDate,
    /// A reference names an identifier that nothing in the document declares.
    UndeclaredReference,
    /// An identifier is declared more than once, whatever the kinds of the declarations. Each
    /// declaration after the first is its own diagnostic, and references resolve to the first.
    DuplicateDeclaration,
    /// A reference resolves to a declaration of a kind its position does not accept, such as a
    /// candidate's `party` field naming another candidate.
    MismatchedReference,
    /// A URL uses `http` rather than `https`, so a reader following it is not protected from
    /// tampering. A warning.
    InsecureUrl,
    /// A string has no characters, or only spaces and tabs, so it shows the reader a blank. A
    /// warning.
    EmptyString,
    /// A top-level party or candidate is never used, because no `party` field and no campaign
    /// member resolves to it. A warning.
    ///
    /// A duplicate declaration is never used, since references resolve to the first declaration
    /// of an identifier. A party or candidate declared inline is used where it is declared.
    UnusedDeclaration,
}

impl ValidationRule {
    /// Returns this rule's identifier in the specification, such as `"missing-guide-field"`.
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported-version",
            Self::MissingGuideField => "missing-guide-field",
            Self::EmptyDocument => "empty-document",
            Self::InvalidDate => "invalid-date",
            Self::UndeclaredReference => "undeclared-reference",
            Self::DuplicateDeclaration => "duplicate-declaration",
            Self::MismatchedReference => "mismatched-reference",
            Self::InsecureUrl => "insecure-url",
            Self::EmptyString => "empty-string",
            Self::UnusedDeclaration => "unused-declaration",
        }
    }

    /// Returns how serious breaking this rule is.
    pub fn severity(self) -> Severity {
        match self {
            Self::UnsupportedVersion
            | Self::MissingGuideField
            | Self::EmptyDocument
            | Self::InvalidDate
            | Self::UndeclaredReference
            | Self::DuplicateDeclaration
            | Self::MismatchedReference => Severity::Error,
            Self::InsecureUrl | Self::EmptyString | Self::UnusedDeclaration => Severity::Warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationRule;
    use crate::Severity;

    #[test]
    fn each_rule_has_its_identifier_and_severity() {
        for (rule, id, severity) in [
            (
                ValidationRule::UnsupportedVersion,
                "unsupported-version",
                Severity::Error,
            ),
            (
                ValidationRule::MissingGuideField,
                "missing-guide-field",
                Severity::Error,
            ),
            (
                ValidationRule::EmptyDocument,
                "empty-document",
                Severity::Error,
            ),
            (ValidationRule::InvalidDate, "invalid-date", Severity::Error),
            (
                ValidationRule::UndeclaredReference,
                "undeclared-reference",
                Severity::Error,
            ),
            (
                ValidationRule::DuplicateDeclaration,
                "duplicate-declaration",
                Severity::Error,
            ),
            (
                ValidationRule::MismatchedReference,
                "mismatched-reference",
                Severity::Error,
            ),
            (
                ValidationRule::InsecureUrl,
                "insecure-url",
                Severity::Warning,
            ),
            (
                ValidationRule::EmptyString,
                "empty-string",
                Severity::Warning,
            ),
            (
                ValidationRule::UnusedDeclaration,
                "unused-declaration",
                Severity::Warning,
            ),
        ] {
            assert_eq!(rule.spec_id(), id);
            assert_eq!(rule.severity(), severity);
        }
    }
}
