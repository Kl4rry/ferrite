// "Borrowed" from helix
//! Utility functions to traverse the unicode graphemes of a `Rope`'s text contents.
//!
//! Based on <https://github.com/cessen/led/blob/c4fa72405f510b7fd16052f90a598c429b3104a6/src/graphemes.rs>
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    slice, str,
};

use ferrite_ctx::{Arena, ArenaString};
use ropey::{Rope, RopeSlice, iter::Chunks, str_utils::byte_to_char_idx};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::UnicodeWidthStr;

use crate::{
    line_ending::{self, LineEnding, get_line_ending, line_without_line_ending},
    point::Point,
};

pub const TAB_WIDTH: u16 = 4;

#[inline]
pub fn tab_width_at(visual_x: usize, tab_width: u16) -> usize {
    tab_width as usize - (visual_x % tab_width as usize)
}

#[must_use]
pub fn grapheme_width(g: &str, current_col: usize) -> usize {
    if g.as_bytes()[0] <= 127 {
        // Fast-path ascii.
        // Point 1: theoretically, ascii control characters should have zero
        // width, but in our case we actually want them to have width: if they
        // show up in text, we want to treat them as textual elements that can
        // be edited.  So we can get away with making all ascii single width
        // here.
        // Point 2: we're only examining the first codepoint here, which means
        // we're ignoring graphemes formed with combining characters.  However,
        // if it starts with ascii, it's going to be a single-width grapeheme
        // regardless, so, again, we can get away with that here.
        // Point 3: we're only examining the first _byte_.  But for utf8, when
        // checking for ascii range values only, that works.
        if g.as_bytes()[0] == b'\t' {
            tab_width_at(current_col, TAB_WIDTH)
        } else {
            1
        }
    } else {
        // We use max(1) here because all grapeheme clusters--even illformed
        // ones--should have at least some width so they can be edited
        // properly.
        // TODO properly handle unicode width for all codepoints
        // example of where unicode width is currently wrong: 🤦🏼‍♂️ (taken from https://hsivonen.fi/string-length/)
        UnicodeWidthStr::width(g).max(1)
    }
}

#[must_use]
pub fn nth_prev_grapheme_boundary(slice: RopeSlice, char_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let mut byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the previous grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.prev_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return 0,
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    let (a, b, c, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                    chunk = a;
                    chunk_byte_idx = b;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

/// Finds the previous grapheme boundary before the given char position.
#[must_use]
#[inline(always)]
pub fn prev_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> usize {
    nth_prev_grapheme_boundary(slice, char_idx, 1)
}

#[must_use]
pub fn nth_next_grapheme_boundary(slice: RopeSlice, char_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let mut byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the nth next grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.next_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return slice.len_chars(),
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte_idx += chunk.len();
                    let (a, _, c, _) = slice.chunk_at_byte(chunk_byte_idx);
                    chunk = a;
                    chunk_char_idx = c;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    let tmp = byte_to_char_idx(chunk, byte_idx - chunk_byte_idx);
    chunk_char_idx + tmp
}

#[must_use]
fn nth_next_grapheme_boundary_byte(slice: RopeSlice, mut byte_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(byte_idx <= slice.len_bytes());

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, mut _chunk_char_idx, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the nth next grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.next_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return slice.len_bytes(),
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte_idx += chunk.len();
                    let (a, _, _, _) = slice.chunk_at_byte(chunk_byte_idx);
                    chunk = a;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    byte_idx
}

#[must_use]
fn nth_prev_grapheme_boundary_byte(slice: RopeSlice, mut byte_idx: usize, n: usize) -> usize {
    // Bounds check
    debug_assert!(byte_idx <= slice.len_bytes());

    // Get the chunk with our byte index in it.
    let (mut chunk, mut chunk_byte_idx, _, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Find the previous grapheme cluster boundary.
    for _ in 0..n {
        loop {
            match gc.prev_boundary(chunk, chunk_byte_idx) {
                Ok(None) => return 0,
                Ok(Some(n)) => {
                    byte_idx = n;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    let (a, b, _, _) = slice.chunk_at_byte(chunk_byte_idx - 1);
                    chunk = a;
                    chunk_byte_idx = b;
                }
                Err(GraphemeIncomplete::PreContext(n)) => {
                    let ctx_chunk = slice.chunk_at_byte(n - 1).0;
                    gc.provide_context(ctx_chunk, n - ctx_chunk.len());
                }
                _ => unreachable!(),
            }
        }
    }
    byte_idx
}

/// Finds the next grapheme boundary after the given byte position.
#[must_use]
#[inline(always)]
fn prev_grapheme_boundary_byte(slice: RopeSlice, byte_idx: usize) -> usize {
    nth_prev_grapheme_boundary_byte(slice, byte_idx, 1)
}

/// Finds the next grapheme boundary after the given char position.
#[must_use]
#[inline(always)]
fn next_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> usize {
    nth_next_grapheme_boundary(slice, char_idx, 1)
}

/// Finds the next grapheme boundary after the given byte position.
#[must_use]
#[inline(always)]
fn next_grapheme_boundary_byte(slice: RopeSlice, byte_idx: usize) -> usize {
    nth_next_grapheme_boundary_byte(slice, byte_idx, 1)
}

/// Returns the passed char index if it's already a grapheme boundary,
/// or the next grapheme boundary char index if not.
#[must_use]
#[inline]
pub fn ensure_grapheme_boundary_next(slice: RopeSlice, char_idx: usize) -> usize {
    if char_idx == 0 {
        char_idx
    } else {
        next_grapheme_boundary(slice, char_idx - 1)
    }
}

/// Returns the passed char index if it's already a grapheme boundary,
/// or the prev grapheme boundary char index if not.
#[must_use]
#[inline]
pub fn ensure_grapheme_boundary_prev(slice: RopeSlice, char_idx: usize) -> usize {
    if char_idx == slice.len_chars() {
        char_idx
    } else {
        prev_grapheme_boundary(slice, char_idx + 1)
    }
}

/// Returns the passed byte index if it's already a grapheme boundary,
/// or the next grapheme boundary byte index if not.
#[must_use]
#[inline]
fn ensure_grapheme_boundary_next_byte(slice: RopeSlice, byte_idx: usize) -> usize {
    if byte_idx == 0 {
        byte_idx
    } else {
        // TODO: optimize so we're not constructing grapheme cursor twice
        if is_grapheme_boundary_byte(slice, byte_idx) {
            byte_idx
        } else {
            next_grapheme_boundary_byte(slice, byte_idx)
        }
    }
}

/// Returns whether the given char position is a grapheme boundary.
#[must_use]
pub fn is_grapheme_boundary(slice: RopeSlice, char_idx: usize) -> bool {
    // Bounds check
    debug_assert!(char_idx <= slice.len_chars());

    // We work with bytes for this, so convert.
    let byte_idx = slice.char_to_byte(char_idx);

    // Get the chunk with our byte index in it.
    let (chunk, chunk_byte_idx, _, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Determine if the given position is a grapheme cluster boundary.
    loop {
        match gc.is_boundary(chunk, chunk_byte_idx) {
            Ok(n) => return n,
            Err(GraphemeIncomplete::PreContext(n)) => {
                let (ctx_chunk, ctx_byte_start, _, _) = slice.chunk_at_byte(n - 1);
                gc.provide_context(ctx_chunk, ctx_byte_start);
            }
            Err(_) => unreachable!(),
        }
    }
}

/// Returns whether the given byte position is a grapheme boundary.
#[must_use]
pub fn is_grapheme_boundary_byte(slice: RopeSlice, byte_idx: usize) -> bool {
    // Bounds check
    debug_assert!(byte_idx <= slice.len_bytes());

    // Get the chunk with our byte index in it.
    let (chunk, chunk_byte_idx, _, _) = slice.chunk_at_byte(byte_idx);

    // Set up the grapheme cursor.
    let mut gc = GraphemeCursor::new(byte_idx, slice.len_bytes(), true);

    // Determine if the given position is a grapheme cluster boundary.
    loop {
        match gc.is_boundary(chunk, chunk_byte_idx) {
            Ok(n) => return n,
            Err(GraphemeIncomplete::PreContext(n)) => {
                let (ctx_chunk, ctx_byte_start, _, _) = slice.chunk_at_byte(n - 1);
                gc.provide_context(ctx_chunk, ctx_byte_start);
            }
            Err(_) => unreachable!(),
        }
    }
}

/// An iterator over the graphemes of a `RopeSlice`.
#[derive(Clone)]
pub struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl fmt::Debug for RopeGraphemes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RopeGraphemes")
            .field("text", &self.text)
            .field("chunks", &self.chunks)
            .field("cur_chunk", &self.cur_chunk)
            .field("cur_chunk_start", &self.cur_chunk_start)
            // .field("cursor", &self.cursor)
            .finish()
    }
}

impl RopeGraphemes<'_> {
    #[must_use]
    pub fn new(slice: RopeSlice) -> RopeGraphemes {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: slice,
            chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            Some(self.text.byte_slice(a..b))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

/// A highly compressed Cow<'a, str> that holds
/// atmost u31::MAX bytes and is readonly
pub struct GraphemeStr<'a> {
    ptr: NonNull<u8>,
    len: u32,
    phantom: PhantomData<&'a str>,
}

impl GraphemeStr<'_> {
    const MASK_OWNED: u32 = 1 << 31;

    fn compute_len(&self) -> usize {
        (self.len & !Self::MASK_OWNED) as usize
    }
}

impl Deref for GraphemeStr<'_> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let bytes = slice::from_raw_parts(self.ptr.as_ptr(), self.compute_len());
            str::from_utf8_unchecked(bytes)
        }
    }
}

impl Drop for GraphemeStr<'_> {
    fn drop(&mut self) {
        if self.len & Self::MASK_OWNED != 0 {
            // free allocation
            unsafe {
                drop(Box::from_raw(slice::from_raw_parts_mut(
                    self.ptr.as_ptr(),
                    self.compute_len(),
                )));
            }
        }
    }
}

impl<'a> From<&'a str> for GraphemeStr<'a> {
    fn from(g: &'a str) -> Self {
        GraphemeStr {
            ptr: unsafe { NonNull::new_unchecked(g.as_bytes().as_ptr() as *mut u8) },
            len: i32::try_from(g.len()).unwrap() as u32,
            phantom: PhantomData,
        }
    }
}

impl From<String> for GraphemeStr<'_> {
    fn from(g: String) -> Self {
        let len = g.len();
        let ptr = Box::into_raw(g.into_bytes().into_boxed_slice()) as *mut u8;
        GraphemeStr {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            len: i32::try_from(len).unwrap() as u32,
            phantom: PhantomData,
        }
    }
}

impl<'a> From<Cow<'a, str>> for GraphemeStr<'a> {
    fn from(g: Cow<'a, str>) -> Self {
        match g {
            Cow::Borrowed(g) => g.into(),
            Cow::Owned(g) => g.into(),
        }
    }
}

impl<T: Deref<Target = str>> PartialEq<T> for GraphemeStr<'_> {
    fn eq(&self, other: &T) -> bool {
        self.deref() == other.deref()
    }
}
impl PartialEq<str> for GraphemeStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.deref() == other
    }
}
impl Eq for GraphemeStr<'_> {}
impl Debug for GraphemeStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}
impl Display for GraphemeStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.deref(), f)
    }
}
impl Clone for GraphemeStr<'_> {
    fn clone(&self) -> Self {
        self.deref().to_owned().into()
    }
}

pub trait RopeGraphemeExt {
    fn width(&self, current_col: usize) -> usize;
    fn line_without_line_ending(&self, line_idx: usize) -> RopeSlice;
    fn prev_grapheme_boundary_byte(&self, byte_idx: usize) -> usize;
    fn nth_prev_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize;
    fn next_grapheme_boundary_byte(&self, byte_idx: usize) -> usize;
    fn nth_next_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize;
    fn grapehemes(&self) -> RopeGraphemes;
    fn get_line_ending(&self) -> Option<LineEnding>;
    fn last_n_columns(&self, n: usize) -> RopeSlice;

    fn is_whitespace(&self) -> bool;
    fn is_word_char(&self) -> bool;

    fn end_of_line_byte(&self, line_idx: usize) -> usize;
    fn end_of_line_char(&self, line_idx: usize) -> usize;

    fn starts_width_char(&self, ch: char) -> bool;

    fn get_text_start_col(&self, line_idx: usize) -> usize;
    fn get_text_start_byte(&self, line_idx: usize) -> usize;

    fn get_text_end_col(&self, line_idx: usize) -> usize;

    fn byte_to_col(&self, byte_idx: usize) -> usize;
    fn byte_to_point(&self, byte_idx: usize) -> Point<usize>;

    fn ensure_grapheme_boundary_next_byte(&self, byte_idx: usize) -> usize;

    fn trim_start_whitespace(&self) -> RopeSlice;

    fn to_arena_string<'a>(&self, arena: &'a Arena) -> ArenaString<'a>;
}

impl RopeGraphemeExt for RopeSlice<'_> {
    fn width(&self, current_col: usize) -> usize {
        let mut width = 0;
        for grapheme in RopeGraphemes::new(*self) {
            for chunk in grapheme.chunks() {
                width += grapheme_width(chunk, current_col + width);
            }
        }
        width
    }

    fn line_without_line_ending(&self, line_idx: usize) -> RopeSlice {
        line_without_line_ending(self, line_idx)
    }

    fn prev_grapheme_boundary_byte(&self, byte_idx: usize) -> usize {
        prev_grapheme_boundary_byte(*self, byte_idx)
    }

    fn nth_prev_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize {
        nth_prev_grapheme_boundary_byte(*self, byte_idx, n)
    }

    fn next_grapheme_boundary_byte(&self, byte_idx: usize) -> usize {
        next_grapheme_boundary_byte(*self, byte_idx)
    }

    fn nth_next_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize {
        nth_next_grapheme_boundary_byte(*self, byte_idx, n)
    }

    fn grapehemes(&self) -> RopeGraphemes {
        RopeGraphemes::new(*self)
    }

    fn get_line_ending(&self) -> Option<LineEnding> {
        get_line_ending(self)
    }

    fn last_n_columns(&self, n: usize) -> RopeSlice {
        let left = self.width(0).saturating_sub(n);
        let mut width = 0;
        let mut byte_idx = 0;
        for grapheme in self.grapehemes() {
            if width >= left {
                break;
            }

            width += grapheme.width(width);
            byte_idx += grapheme.len_bytes();
        }

        self.byte_slice(byte_idx..)
    }

    fn is_whitespace(&self) -> bool {
        self.chars().all(char::is_whitespace)
    }

    fn is_word_char(&self) -> bool {
        self.chars().all(|ch| {
            use unicode_general_category::GeneralCategory;
            matches!(
                unicode_general_category::get_general_category(ch),
                GeneralCategory::ConnectorPunctuation
                    | GeneralCategory::UppercaseLetter
                    | GeneralCategory::TitlecaseLetter
                    | GeneralCategory::LowercaseLetter
                    | GeneralCategory::NonspacingMark
                    | GeneralCategory::DecimalNumber
                    | GeneralCategory::LetterNumber
                    | GeneralCategory::OtherLetter
            )
        })
    }

    fn end_of_line_byte(&self, line_idx: usize) -> usize {
        let line_len = self.line(line_idx).len_bytes();
        let line_start = self.line_to_byte(line_idx);
        line_start + line_len
    }

    fn end_of_line_char(&self, line_idx: usize) -> usize {
        let line_len = self.line(line_idx).len_chars();
        let line_start = self.line_to_char(line_idx);
        line_start + line_len
    }

    fn starts_width_char(&self, ch: char) -> bool {
        self.chars()
            .next()
            .map(|first| first == ch)
            .unwrap_or(false)
    }

    fn get_text_start_col(&self, line_idx: usize) -> usize {
        let line = self.line_without_line_ending(line_idx);
        let mut width = 0;
        for grapheme in line.grapehemes() {
            if !grapheme.is_whitespace() {
                break;
            }
            width += grapheme.width(width);
        }

        width
    }

    fn get_text_start_byte(&self, line_idx: usize) -> usize {
        let line = self.line_without_line_ending(line_idx);
        let mut len = 0;
        for grapheme in line.grapehemes() {
            if !grapheme.is_whitespace() {
                break;
            }
            len += grapheme.len_bytes();
        }

        len
    }

    fn get_text_end_col(&self, line_idx: usize) -> usize {
        let line = self.line_without_line_ending(line_idx);
        let mut width = 0;
        let mut text_width = 0;
        let mut has_text = false;
        for grapheme in line.grapehemes() {
            if !grapheme.is_whitespace() {
                has_text = true;
            }
            width += grapheme.width(width);
            if has_text && !grapheme.is_whitespace() {
                text_width = width;
            }
        }

        text_width
    }

    fn byte_to_col(&self, byte_idx: usize) -> usize {
        let mut bytes = 0;
        let mut width = 0;
        for grapheme in self.grapehemes() {
            if bytes >= byte_idx {
                break;
            }

            width += grapheme.width(width);
            bytes += grapheme.len_bytes();
        }
        width
    }

    fn byte_to_point(&self, byte_idx: usize) -> Point<usize> {
        let line = self.byte_to_line(byte_idx);
        let line_start = self.line_to_byte(line);
        let line_byte_idx = byte_idx - line_start;
        let column = self.line(line).byte_to_col(line_byte_idx);
        Point { line, column }
    }

    fn ensure_grapheme_boundary_next_byte(&self, byte_idx: usize) -> usize {
        ensure_grapheme_boundary_next_byte(*self, byte_idx)
    }

    fn trim_start_whitespace(&self) -> RopeSlice {
        let mut start = 0;
        for grapheme in self.grapehemes() {
            if grapheme.is_whitespace() {
                start += grapheme.len_bytes();
            } else {
                break;
            }
        }

        self.byte_slice(start..)
    }

    fn to_arena_string<'a>(&self, arena: &'a Arena) -> ArenaString<'a> {
        let mut string = ArenaString::with_capacity_in(self.len_bytes(), arena);
        for chunk in self.chunks() {
            string.push_str(chunk);
        }
        string
    }
}

impl RopeGraphemeExt for Rope {
    fn width(&self, current_col: usize) -> usize {
        self.byte_slice(..).width(current_col)
    }

    fn line_without_line_ending(&self, line_idx: usize) -> RopeSlice {
        let start = self.line_to_char(line_idx);
        let end = line_ending::line_end_char_index(&self.byte_slice(..), line_idx);
        self.slice(start..end)
    }

    fn prev_grapheme_boundary_byte(&self, byte_idx: usize) -> usize {
        self.byte_slice(..).prev_grapheme_boundary_byte(byte_idx)
    }

    fn nth_prev_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize {
        self.byte_slice(..)
            .nth_prev_grapheme_boundary_byte(byte_idx, n)
    }

    fn next_grapheme_boundary_byte(&self, byte_idx: usize) -> usize {
        self.byte_slice(..).next_grapheme_boundary_byte(byte_idx)
    }

    fn nth_next_grapheme_boundary_byte(&self, byte_idx: usize, n: usize) -> usize {
        self.byte_slice(..)
            .nth_next_grapheme_boundary_byte(byte_idx, n)
    }

    fn grapehemes(&self) -> RopeGraphemes {
        RopeGraphemes::new(self.byte_slice(..))
    }

    fn get_line_ending(&self) -> Option<LineEnding> {
        self.byte_slice(..).get_line_ending()
    }

    fn last_n_columns(&self, n: usize) -> RopeSlice {
        let left = self.width(0).saturating_sub(n);
        let mut width = 0;
        let mut byte_idx = 0;
        for grapheme in self.grapehemes() {
            if width >= left {
                break;
            }

            width += grapheme.width(width);
            byte_idx += grapheme.len_bytes();
        }

        self.byte_slice(byte_idx..)
    }

    fn is_whitespace(&self) -> bool {
        self.byte_slice(..).is_whitespace()
    }

    fn is_word_char(&self) -> bool {
        self.byte_slice(..).is_word_char()
    }

    fn end_of_line_byte(&self, byte_idx: usize) -> usize {
        self.byte_slice(..).end_of_line_byte(byte_idx)
    }

    fn end_of_line_char(&self, char_idx: usize) -> usize {
        self.byte_slice(..).end_of_line_char(char_idx)
    }

    fn starts_width_char(&self, ch: char) -> bool {
        self.slice(..).starts_width_char(ch)
    }

    fn get_text_start_col(&self, line_idx: usize) -> usize {
        self.slice(..).get_text_start_col(line_idx)
    }

    fn get_text_start_byte(&self, line_idx: usize) -> usize {
        self.slice(..).get_text_start_byte(line_idx)
    }

    fn get_text_end_col(&self, line_idx: usize) -> usize {
        self.slice(..).get_text_end_col(line_idx)
    }

    fn byte_to_col(&self, byte_idx: usize) -> usize {
        self.slice(..).byte_to_col(byte_idx)
    }

    fn byte_to_point(&self, byte_idx: usize) -> Point<usize> {
        self.slice(..).byte_to_point(byte_idx)
    }

    fn ensure_grapheme_boundary_next_byte(&self, byte_idx: usize) -> usize {
        self.slice(..).ensure_grapheme_boundary_next_byte(byte_idx)
    }

    fn trim_start_whitespace(&self) -> RopeSlice {
        let mut start = 0;
        for grapheme in self.grapehemes() {
            if grapheme.is_whitespace() {
                start += grapheme.len_bytes();
            } else {
                break;
            }
        }

        self.byte_slice(start..)
    }

    fn to_arena_string<'a>(&self, arena: &'a Arena) -> ArenaString<'a> {
        self.slice(..).to_arena_string(arena)
    }
}
