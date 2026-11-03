use oval_syntax::TextSize;

pub(crate) struct LineIndex {
    line_starts: Vec<usize>,
    source_len: usize,
    source: String,
}

impl LineIndex {
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => line_starts.push(i + 1),
                b'\r' => {
                    if bytes.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    line_starts.push(i + 1);
                }
                _ => {}
            }
            i += 1;
        }
        Self {
            line_starts,
            source_len: source.len(),
            source: source.to_owned(),
        }
    }

    pub(crate) fn line_column(&self, offset: TextSize) -> (usize, usize) {
        let offset = usize::from(offset).min(self.source_len);
        let line = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let column = self.source[self.line_starts[line]..offset].chars().count() + 1;
        (line + 1, column)
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    fn at(source: &str, offset: u32) -> (usize, usize) {
        LineIndex::new(source).line_column(offset.into())
    }

    #[test]
    fn lines_and_columns_are_one_based() {
        let source = "guide \"x\"\nparty @d \"D\"\n";
        assert_eq!(at(source, 0), (1, 1));
        assert_eq!(at(source, 6), (1, 7));
        assert_eq!(at(source, 10), (2, 1));
        assert_eq!(at(source, 16), (2, 7));
    }

    #[test]
    fn every_terminator_form_starts_a_new_line() {
        assert_eq!(at("a\nb", 2), (2, 1));
        assert_eq!(at("a\r\nb", 3), (2, 1));
        assert_eq!(at("a\rb", 2), (2, 1));
        assert_eq!(at("a\r\nb", 1), (1, 2));
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        let source = "José Cisneros";
        assert_eq!(at(source, 5), (1, 5));
        assert_eq!(at(source, 14), (1, 14));
    }

    #[test]
    fn end_of_input_is_on_the_last_line() {
        assert_eq!(at("a\nbc", 4), (2, 3));
        assert_eq!(at("a\n", 2), (2, 1));
        assert_eq!(at("", 0), (1, 1));
    }
}
