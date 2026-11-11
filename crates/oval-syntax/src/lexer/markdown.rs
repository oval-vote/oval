use super::{Lexer, is_line_terminator};
use crate::TokenKind;

impl Lexer<'_> {
    pub(super) fn markdown_block(&mut self, start: usize) -> bool {
        let Some(opener) = self.body_opener else {
            return false;
        };
        let first = self.measure_line(start);
        if first.is_blank {
            return false;
        }
        self.body_opener = None;
        if first.indentation <= opener {
            return false;
        }
        let mut end = first.end;
        let mut next = first.end;
        while next < self.source.len() {
            let line = self.measure_line(next);
            if !line.is_blank && line.indentation <= opener {
                break;
            }
            next = line.end;
            if !line.is_blank {
                end = line.end;
            }
        }
        self.pos = end;
        self.push(TokenKind::MarkdownBlock, start);
        self.line_has_tokens = false;
        self.line_start = self.pos;
        true
    }

    fn measure_line(&self, start: usize) -> Line {
        let text = &self.source[start..];
        let indentation = text.bytes().take_while(|&b| b == b' ').count();
        let content_len = text.find(is_line_terminator).unwrap_or(text.len());
        let terminator_len = match &text.as_bytes()[content_len..] {
            [] => 0,
            [b'\r', b'\n', ..] => 2,
            _ => 1,
        };
        Line {
            indentation,
            is_blank: indentation == content_len,
            end: start + content_len + terminator_len,
        }
    }
}

struct Line {
    indentation: usize,
    is_blank: bool,
    end: usize,
}
