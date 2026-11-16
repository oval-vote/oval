pub(crate) struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        for (offset, byte) in bytes.iter().enumerate() {
            let ends_line = match byte {
                b'\n' => true,
                b'\r' => bytes.get(offset + 1) != Some(&b'\n'),
                _ => false,
            };
            if ends_line {
                starts.push(offset + 1);
            }
        }
        Self { text, starts }
    }

    pub(crate) fn position(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line = self.starts.partition_point(|&start| start <= offset);
        let start = self.starts[line - 1];
        let column = self.text[start..offset].chars().count() + 1;
        (line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    #[test]
    fn a_position_is_a_one_based_line_and_column() {
        let index = LineIndex::new("ab\ncd\n");
        assert_eq!(index.position(0), (1, 1));
        assert_eq!(index.position(1), (1, 2));
        assert_eq!(index.position(3), (2, 1));
        assert_eq!(index.position(6), (3, 1));
    }

    #[test]
    fn every_line_terminator_ends_a_line() {
        let index = LineIndex::new("a\r\nb\rc\nd");
        assert_eq!(index.position(3), (2, 1));
        assert_eq!(index.position(5), (3, 1));
        assert_eq!(index.position(7), (4, 1));
    }

    #[test]
    fn a_column_counts_characters_rather_than_bytes() {
        let index = LineIndex::new("é😀x");
        assert_eq!(index.position("é😀".len()), (1, 3));
    }

    #[test]
    fn an_offset_past_the_end_is_the_end() {
        assert_eq!(LineIndex::new("ab").position(10), (1, 3));
    }
}
