use crate::ast::{AstNode, IntegerLiteral, ast_node, support};

ast_node! {
    /// The version marker, such as `oval 1`, which names the edition of the specification a
    /// document is written for.
    ///
    /// ```
    /// let result = oval_syntax::parse("oval 1\n")?;
    /// let marker = result.document().version_marker().expect("the document has a marker");
    /// let version = marker.version().expect("the marker has a version");
    /// assert_eq!(version.value(), Some(1));
    /// # Ok::<(), oval_syntax::InputRejection>(())
    /// ```
    VersionMarker
}

impl VersionMarker {
    /// Returns the marker's version, or `None` if the line ends without one or has another kind of
    /// token in its place.
    pub fn version(&self) -> Option<IntegerLiteral> {
        support::token(self.syntax())
    }
}

#[cfg(test)]
mod tests {
    use super::VersionMarker;
    use crate::SyntaxKind;
    use crate::ast::AstNode;

    fn version(text: &str) -> Option<u64> {
        crate::parse(text)
            .unwrap()
            .syntax()
            .descendants()
            .find_map(VersionMarker::cast)
            .expect("the document has a marker")
            .version()
            .and_then(|version| version.value())
    }

    #[test]
    fn a_marker_has_its_version() {
        assert_eq!(version("oval 1\n"), Some(1));
        assert_eq!(version("oval 007\n"), Some(7));
    }

    #[test]
    fn a_marker_without_a_version_has_none() {
        assert_eq!(version("oval\n"), None);
        assert_eq!(version("oval x\n"), None);
    }

    #[test]
    fn only_a_version_marker_node_is_a_version_marker() {
        assert!(VersionMarker::can_cast(SyntaxKind::VersionMarker));
        assert!(!VersionMarker::can_cast(SyntaxKind::Document));
    }
}
