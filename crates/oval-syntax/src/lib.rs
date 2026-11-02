//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

/// The version of the Oval specification this crate implements.
pub const SPEC_VERSION: &str = "0.1";

#[cfg(test)]
mod tests {
    use super::SPEC_VERSION;

    #[test]
    fn spec_version_is_a_major_minor_pair() {
        let parts: Vec<&str> = SPEC_VERSION.split('.').collect();
        assert_eq!(parts.len(), 2, "expected MAJOR.MINOR, got {SPEC_VERSION:?}");
        for part in parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "non-numeric component in {SPEC_VERSION:?}"
            );
        }
    }
}
