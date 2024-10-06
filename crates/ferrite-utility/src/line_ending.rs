// "Borrowed" from helix
use ropey::{Rope, RopeSlice};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::Crlf;
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::LF;

/// Represents one of the valid Unicode line endings.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum LineEnding {
    Crlf, // CarriageReturn followed by LineFeed
    LF,   // U+000A -- LineFeed
    VT,   // U+000B -- VerticalTab
    FF,   // U+000C -- FormFeed
    CR,   // U+000D -- CarriageReturn
    Nel,  // U+0085 -- NextLine
    LS,   // U+2028 -- Line Separator
    PS,   // U+2029 -- ParagraphSeparator
}

impl LineEnding {
    #[inline]
    pub const fn len_chars(&self) -> usize {
        match self {
            Self::Crlf => 2,
            _ => 1,
        }
    }

    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Crlf => "\u{000D}\u{000A}",
            Self::LF => "\u{000A}",
            Self::VT => "\u{000B}",
            Self::FF => "\u{000C}",
            Self::CR => "\u{000D}",
            Self::Nel => "\u{0085}",
            Self::LS => "\u{2028}",
            Self::PS => "\u{2029}",
        }
    }

    #[inline]
    pub const fn from_char(ch: char) -> Option<LineEnding> {
        match ch {
            '\u{000A}' => Some(LineEnding::LF),
            '\u{000B}' => Some(LineEnding::VT),
            '\u{000C}' => Some(LineEnding::FF),
            '\u{000D}' => Some(LineEnding::CR),
            '\u{0085}' => Some(LineEnding::Nel),
            '\u{2028}' => Some(LineEnding::LS),
            '\u{2029}' => Some(LineEnding::PS),
            // Not a line ending
            _ => None,
        }
    }

    // Normally we'd want to implement the FromStr trait, but in this case
    // that would force us into a different return type than from_char or
    // or from_rope_slice, which would be weird.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(g: &str) -> Option<LineEnding> {
        match g {
            "\u{000D}\u{000A}" => Some(LineEnding::Crlf),
            "\u{000A}" => Some(LineEnding::LF),
            "\u{000B}" => Some(LineEnding::VT),
            "\u{000C}" => Some(LineEnding::FF),
            "\u{000D}" => Some(LineEnding::CR),
            "\u{0085}" => Some(LineEnding::Nel),
            "\u{2028}" => Some(LineEnding::LS),
            "\u{2029}" => Some(LineEnding::PS),
            // Not a line ending
            _ => None,
        }
    }

    #[inline]
    pub fn from_rope_slice(g: &RopeSlice) -> Option<LineEnding> {
        if let Some(text) = g.as_str() {
            LineEnding::from_str(text)
        } else {
            // Non-contiguous, so it can't be a line ending.
            // Specifically, Ropey guarantees that CRLF is always
            // contiguous.  And the remaining line endings are all
            // single `char`s, and therefore trivially contiguous.
            None
        }
    }
}

#[inline]
pub fn str_is_line_ending(s: &str) -> bool {
    LineEnding::from_str(s).is_some()
}

#[inline]
pub fn rope_is_line_ending(r: RopeSlice) -> bool {
    r.chunks().all(str_is_line_ending)
}

/// Attempts to detect what line ending the passed document uses.
pub fn auto_detect_line_ending(doc: &Rope) -> Option<LineEnding> {
    // Return first matched line ending. Not all possible line endings
    // are being matched, as they might be special-use only
    for line in doc.lines().take(100) {
        match get_line_ending(&line) {
            None => {}
            Some(LineEnding::VT) | Some(LineEnding::FF) | Some(LineEnding::PS) => {}
            ending => return ending,
        }
    }
    None
}

/// Returns the passed line's line ending, if any.
pub fn get_line_ending(line: &RopeSlice) -> Option<LineEnding> {
    // Last character as str.
    let g1 = line
        .slice(line.len_chars().saturating_sub(1)..)
        .as_str()
        .unwrap();

    // Last two characters as str, or empty str if they're not contiguous.
    // It's fine to punt on the non-contiguous case, because Ropey guarantees
    // that CRLF is always contiguous.
    let g2 = line
        .slice(line.len_chars().saturating_sub(2)..)
        .as_str()
        .unwrap_or("");

    // First check the two-character case for CRLF, then check the single-character case.
    LineEnding::from_str(g2).or_else(|| LineEnding::from_str(g1))
}

/// Returns the passed line's line ending, if any.
pub fn get_line_ending_of_str(line: &str) -> Option<LineEnding> {
    if line.ends_with("\u{000D}\u{000A}") {
        Some(LineEnding::Crlf)
    } else if line.ends_with('\u{000A}') {
        Some(LineEnding::LF)
    } else if line.ends_with('\u{000B}') {
        Some(LineEnding::VT)
    } else if line.ends_with('\u{000C}') {
        Some(LineEnding::FF)
    } else if line.ends_with('\u{000D}') {
        Some(LineEnding::CR)
    } else if line.ends_with('\u{0085}') {
        Some(LineEnding::Nel)
    } else if line.ends_with('\u{2028}') {
        Some(LineEnding::LS)
    } else if line.ends_with('\u{2029}') {
        Some(LineEnding::PS)
    } else {
        None
    }
}

/// Returns the char index of the end of the given line, not including its line ending.
pub fn line_end_char_index(slice: &RopeSlice, line: usize) -> usize {
    slice.line_to_char(line + 1)
        - get_line_ending(&slice.line(line))
            .map(|le| le.len_chars())
            .unwrap_or(0)
}

/// Fetches line `line_idx` from the passed rope slice, sans any line ending.
pub(crate) fn line_without_line_ending<'a>(slice: &'a RopeSlice, line_idx: usize) -> RopeSlice<'a> {
    let start = slice.line_to_char(line_idx);
    let end = line_end_char_index(slice, line_idx);
    slice.slice(start..end)
}

/// Returns the char index of the end of the given RopeSlice, not including
/// any final line ending.
pub fn rope_end_without_line_ending(slice: &RopeSlice) -> usize {
    slice.len_chars() - get_line_ending(slice).map(|le| le.len_chars()).unwrap_or(0)
}
