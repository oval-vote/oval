//! Parses every document under `tests/fixtures`.

use std::fs;
use std::path::Path;

use oval_syntax::parser::parse;

#[test]
fn every_fixture_parses_without_errors_and_round_trips() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut count = 0;
    for entry in fs::read_dir(&dir).expect("fixtures directory") {
        let path = entry.expect("directory entry").path();
        let source = fs::read_to_string(&path).expect("fixture is UTF-8");
        let result = parse(&source).expect("accepted");
        assert_eq!(
            result.syntax().text().to_string(),
            source,
            "{}",
            path.display()
        );
        assert_eq!(result.errors(), [], "{}", path.display());
        count += 1;
    }
    assert_eq!(count, 1, "expected the fixture in {}", dir.display());
}
