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
}

impl ValidationRule {
    /// Returns this rule's identifier in the specification, such as `"missing-guide-field"`.
    pub fn spec_id(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported-version",
            Self::MissingGuideField => "missing-guide-field",
            Self::EmptyDocument => "empty-document",
        }
    }

    /// Returns how serious breaking this rule is.
    pub fn severity(self) -> Severity {
        match self {
            Self::UnsupportedVersion | Self::MissingGuideField | Self::EmptyDocument => {
                Severity::Error
            }
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
        ] {
            assert_eq!(rule.spec_id(), id);
            assert_eq!(rule.severity(), severity);
        }
    }
}
