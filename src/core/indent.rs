use std::num::NonZeroUsize;

use detect_indent::IndentKind;
use ropey::RopeSlice;

#[derive(Debug, Clone, Copy)]
pub enum Indentation {
    Tabs(NonZeroUsize),
    Spaces(NonZeroUsize),
}

impl Default for Indentation {
    fn default() -> Self {
        Self::Tabs(NonZeroUsize::new(1).unwrap())
    }
}

impl Indentation {
    pub fn detect_indent_rope(rope: RopeSlice) -> Indentation {
        let mut buffer = String::with_capacity(10240);
        for chunk in rope.chunks() {
            if chunk.len() + buffer.len() > buffer.capacity() {
                buffer.push_str(chunk);
                break;
            }
        }
        Self::detect_indent(&buffer)
    }

    pub fn detect_indent(text: &str) -> Indentation {
        let indent = detect_indent::detect_indent(text);
        if indent.amount() == 0 {
            return Default::default();
        }
        match indent.kind() {
            Some(IndentKind::Space) => {
                Indentation::Spaces(NonZeroUsize::new(indent.amount()).unwrap())
            }
            Some(IndentKind::Tab) => Indentation::Tabs(NonZeroUsize::new(indent.amount()).unwrap()),
            None => Default::default(),
        }
    }

    pub fn to_next_ident(self, col: usize) -> String {
        match self {
            // TODO fix this function for tabs
            Indentation::Tabs(_) => "\t".into(),
            Indentation::Spaces(amount) => {
                let amount = amount.get();
                let rest = col % amount;
                let len = if rest == 0 { col } else { rest };

                " ".repeat(len)
            }
        }
    }
}
