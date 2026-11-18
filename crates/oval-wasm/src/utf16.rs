use oval_syntax::{TextRange, TextSize};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Range {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

pub(crate) struct Utf16Index {
    wide: Vec<Wide>,
    len: TextSize,
}

#[derive(Clone, Copy)]
struct Wide {
    offset: TextSize,
    index: u32,
    bytes: TextSize,
    units: u32,
}

impl Utf16Index {
    pub(crate) fn new(text: &str) -> Self {
        let mut wide = Vec::new();
        let mut offset = TextSize::from(0);
        let mut index = 0;
        for character in text.chars() {
            let bytes = TextSize::of(character);
            let units = if character.len_utf16() == 1 { 1 } else { 2 };
            if !character.is_ascii() {
                wide.push(Wide {
                    offset,
                    index,
                    bytes,
                    units,
                });
            }
            offset += bytes;
            index += units;
        }
        Self { wide, len: offset }
    }

    pub(crate) fn index(&self, offset: TextSize) -> u32 {
        let offset = offset.min(self.len);
        let before = self.wide.partition_point(|wide| wide.offset < offset);
        let Some(wide) = before.checked_sub(1).map(|last| self.wide[last]) else {
            return offset.into();
        };
        if offset < wide.offset + wide.bytes {
            return wide.index;
        }
        wide.index + wide.units + u32::from(offset - wide.offset - wide.bytes)
    }

    pub(crate) fn range(&self, range: TextRange) -> Range {
        Range {
            start: self.index(range.start()),
            end: self.index(range.end()),
        }
    }
}

#[cfg(test)]
mod tests {
    use oval_syntax::{TextRange, TextSize};

    use super::{Range, Utf16Index};

    fn indices(text: &str) -> Vec<u32> {
        let index = Utf16Index::new(text);
        (0..=text.len())
            .map(|offset| index.index(TextSize::try_from(offset).unwrap()))
            .collect()
    }

    #[test]
    fn an_ascii_offset_is_its_own_index() {
        assert_eq!(indices("ab\r\nc"), [0, 1, 2, 3, 4, 5]);
        assert_eq!(indices(""), [0]);
    }

    #[test]
    fn an_offset_inside_a_character_rounds_down_to_its_start() {
        assert_eq!(indices("é!"), [0, 0, 1, 2]);
        assert_eq!(indices("a€b"), [0, 1, 1, 1, 2, 3]);
    }

    #[test]
    fn a_character_beyond_the_basic_plane_takes_two_units() {
        assert_eq!(indices("😀x😀"), [0, 0, 0, 0, 2, 3, 3, 3, 3, 5]);
    }

    #[test]
    fn an_offset_past_the_end_clamps() {
        assert_eq!(Utf16Index::new("x😀").index(TextSize::from(99)), 3);
    }

    #[test]
    fn a_range_converts_both_ends() {
        let text = "é \"😀\" @b";
        let start = TextSize::try_from(text.find('@').unwrap()).unwrap();
        let range = TextRange::at(start, TextSize::of("@b"));
        assert_eq!(
            Utf16Index::new(text).range(range),
            Range { start: 7, end: 9 }
        );
    }
}
