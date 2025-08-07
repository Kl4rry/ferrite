use std::{
    borrow::Cow,
    fmt, iter, mem, ops,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Instant,
};

use anyhow::{Result, bail};
use cb::Sender;
use ropey::{Rope, RopeSlice};
use tree_sitter::{
    Language, Node, Parser, Point, Query, QueryCaptures, QueryCursor, QueryError, QueryMatch,
    Range, TextProvider, Tree,
};

use super::{TreeSitterConfig, get_tree_sitter_language};
use crate::event_loop_proxy::EventLoopProxy;

type HighlightResult = Arc<Mutex<Option<(Rope, Vec<HighlightEvent>)>>>;

struct SyntaxProvider {
    pub language: &'static TreeSitterConfig,
    pub rope_tx: Sender<Rope>,
}

impl SyntaxProvider {
    pub fn new(
        language: &'static TreeSitterConfig,
        proxy: Box<dyn EventLoopProxy>,
        result: HighlightResult,
    ) -> Result<Self> {
        let (rope_tx, rope_rx) = cb::unbounded::<Rope>();

        let highlight_config = language.highlight_config.clone();
        let name = language.name.clone();
        thread::spawn(move || {
            tracing::info!("Highlight thread started for `{name}`");
            let mut highlighter = Highlighter::default();
            let mut rope;

            loop {
                rope = match rope_rx.recv() {
                    Ok(rope) => rope,
                    Err(err) => {
                        tracing::info!("Exiting highlight thread: {err}");
                        break;
                    }
                };

                if !rope_rx.is_empty() {
                    continue;
                }

                let time = Instant::now();
                if let Ok(iterator) =
                    highlighter.highlight(&highlight_config.clone(), rope.slice(..), |name| {
                        get_tree_sitter_language(name).map(|language| &*language.highlight_config)
                    })
                {
                    *result.lock().unwrap() = Some((
                        rope.clone(),
                        iterator.filter_map(|event| event.ok()).collect(),
                    ));
                    proxy.request_render();
                }
                tracing::trace!(
                    "highlight took: {}us or {}ms",
                    time.elapsed().as_micros(),
                    time.elapsed().as_millis()
                );
            }

            tracing::info!("Syntax provider thread exit");
        });

        Ok(Self { language, rope_tx })
    }

    pub fn update_text(&self, rope: Rope) {
        let _ = self.rope_tx.send(rope);
    }
}

pub struct Syntax {
    syntax_provder: Option<SyntaxProvider>,
    result: HighlightResult,
    proxy: Box<dyn EventLoopProxy>,
}

impl Syntax {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Self {
        Self {
            syntax_provder: None,
            result: Arc::new(Mutex::new(None)),
            proxy,
        }
    }

    pub fn set_language(&mut self, language: &str) -> Result<()> {
        if language == "text" {
            return Ok(());
        }
        match get_tree_sitter_language(language) {
            Some(lang) => {
                tracing::info!("set lang to `{language}`");
                self.syntax_provder = Some(SyntaxProvider::new(
                    lang,
                    self.proxy.dup(),
                    self.result.clone(),
                )?);
                *self.result.lock().unwrap() = None;
                Ok(())
            }
            None => bail!("Unknown language: `{language}`"),
        }
    }

    pub fn get_language_name(&self) -> Option<&str> {
        Some(&self.syntax_provder.as_ref()?.language.name)
    }

    pub fn update_text(&mut self, rope: Rope) {
        if let Some(syntax) = &self.syntax_provder {
            syntax.update_text(rope);
        }
    }

    pub fn get_highlight_events(&self) -> MutexGuard<Option<(Rope, Vec<HighlightEvent>)>> {
        self.result.lock().unwrap()
    }
}

pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}
impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

pub struct RopeProvider<'a>(pub RopeSlice<'a>);
impl<'a> TextProvider<'a> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

// Stolen from tree-sitter-highlight

/// Indicates which highlight should be applied to a region of source code.
#[derive(Copy, Clone, Debug)]
pub struct Highlight {
    pub capture_index: usize,
    pub query: &'static Query,
}

/// Represents the reason why syntax highlighting failed.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    InvalidLanguage,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => "Cancelled".fmt(f),
            Self::InvalidLanguage => "InvalidLanguage".fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// Represents a single step in rendering a syntax-highlighted document.
#[derive(Copy, Clone, Debug)]
pub enum HighlightEvent {
    Source { start: usize, end: usize },
    HighlightStart(Highlight),
    HighlightEnd,
}

/// Contains the data needed to highlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
pub struct HighlightConfiguration {
    pub language: Language,
    pub query: &'static Query,
    combined_injections_query: &'static Option<Query>,
    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,
}

/// Performs syntax highlighting, recognizing a given list of highlight names.
///
/// For the best performance `Highlighter` values should be reused between
/// syntax highlighting calls. A separate highlighter is needed for each thread that
/// is performing highlighting.
pub struct Highlighter {
    parser: Parser,
    cursors: Vec<QueryCursor>,
}

#[derive(Debug)]
struct LocalDef<'a> {
    name: RopeSlice<'a>,
    value_range: ops::Range<usize>,
    highlight: Option<Highlight>,
}

#[derive(Debug)]
struct LocalScope<'a> {
    inherits: bool,
    range: ops::Range<usize>,
    local_defs: Vec<LocalDef<'a>>,
}

struct HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    source: RopeSlice<'a>,
    byte_offset: usize,
    highlighter: &'a mut Highlighter,
    injection_callback: F,
    layers: Vec<HighlightIterLayer<'a>>,
    next_event: Option<HighlightEvent>,
    last_highlight_range: Option<(usize, usize, usize)>,
}

struct HighlightIterLayer<'a> {
    _tree: Tree,
    cursor: QueryCursor,
    captures: iter::Peekable<QueryCaptures<'a, 'a, RopeProvider<'a>>>,
    config: &'a HighlightConfiguration,
    highlight_end_stack: Vec<usize>,
    scope_stack: Vec<LocalScope<'a>>,
    ranges: Vec<Range>,
    depth: usize,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self {
            parser: Parser::new(),
            cursors: Vec::new(),
        }
    }
}

impl Highlighter {
    #[allow(dead_code)]
    pub fn parser(&mut self) -> &mut Parser {
        &mut self.parser
    }

    /// Iterate over the highlighted regions for a given slice of source code.
    pub fn highlight<'a>(
        &'a mut self,
        config: &'a HighlightConfiguration,
        source: RopeSlice<'a>,
        mut injection_callback: impl FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
    ) -> Result<impl Iterator<Item = Result<HighlightEvent, Error>> + 'a, Error> {
        let layers = HighlightIterLayer::new(
            source,
            self,
            &mut injection_callback,
            config,
            0,
            vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
        )?;
        assert_ne!(layers.len(), 0);
        let mut result = HighlightIter {
            source,
            byte_offset: 0,
            injection_callback,
            highlighter: self,
            layers,
            next_event: None,
            last_highlight_range: None,
        };
        result.sort_layers();
        Ok(result)
    }
}

impl HighlightConfiguration {
    /// Creates a `HighlightConfiguration` for a given `Language` and set of highlighting
    /// queries.
    ///
    /// # Parameters
    ///
    /// * `language`  - The Tree-sitter `Language` that should be used for parsing.
    /// * `highlights_query` - A string containing tree patterns for syntax highlighting. This
    ///   should be non-empty, otherwise no syntax highlights will be added.
    /// * `injections_query` -  A string containing tree patterns for injecting other languages
    ///   into the document. This can be empty if no injections are desired.
    /// * `locals_query` - A string containing tree patterns for tracking local variable
    ///   definitions and references. This can be empty if local variable tracking is not needed.
    ///
    /// Returns a `HighlightConfiguration` that can then be used with the `highlight` method.
    pub fn new(
        language: Language,
        highlights_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Result<Self, QueryError> {
        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(injection_query);
        let locals_query_offset = query_source.len();
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let mut query = Query::new(language, &query_source)?;
        let mut locals_pattern_index = 0;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                if pattern_offset < highlights_query_offset {
                    highlights_pattern_index += 1;
                }
                if pattern_offset < locals_query_offset {
                    locals_pattern_index += 1;
                }
            }
        }

        // Construct a separate query just for dealing with the 'combined injections'.
        // Disable the combined injection patterns in the main query.
        let mut combined_injections_query = Query::new(language, injection_query)?;
        let mut has_combined_queries = false;
        for pattern_index in 0..locals_pattern_index {
            let settings = query.property_settings(pattern_index);
            if settings.iter().any(|s| &*s.key == "injection.combined") {
                has_combined_queries = true;
                query.disable_pattern(pattern_index);
            } else {
                combined_injections_query.disable_pattern(pattern_index);
            }
        }
        let combined_injections_query = if has_combined_queries {
            Some(combined_injections_query)
        } else {
            None
        };

        // Find all of the highlighting patterns that are disabled for nodes that
        // have been identified as local variables.
        let non_local_variable_patterns = (0..query.pattern_count())
            .map(|i| {
                query
                    .property_predicates(i)
                    .iter()
                    .any(|(prop, positive)| !*positive && prop.key.as_ref() == "local")
            })
            .collect();

        // Store the numeric ids for all of the special captures.
        let mut injection_content_capture_index = None;
        let mut injection_language_capture_index = None;
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match name.as_str() {
                "injection.content" => injection_content_capture_index = i,
                "injection.language" => injection_language_capture_index = i,
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        Ok(HighlightConfiguration {
            language,
            query: Box::leak(Box::new(query)),
            combined_injections_query: Box::leak(Box::new(combined_injections_query)),
            locals_pattern_index,
            highlights_pattern_index,
            non_local_variable_patterns,
            injection_content_capture_index,
            injection_language_capture_index,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
            local_scope_capture_index,
        })
    }

    #[allow(dead_code)]
    /// Get a slice containing all of the highlight names used in the configuration.
    pub fn names(&self) -> &[String] {
        self.query.capture_names()
    }
}

impl<'a> HighlightIterLayer<'a> {
    /// Create a new 'layer' of highlighting for this document.
    ///
    /// In the even that the new layer contains "combined injections" (injections where multiple
    /// disjoint ranges are parsed as one syntax tree), these will be eagerly processed and
    /// added to the returned vector.
    fn new<F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a>(
        source: RopeSlice<'a>,
        highlighter: &mut Highlighter,
        injection_callback: &mut F,
        mut config: &'a HighlightConfiguration,
        mut depth: usize,
        mut ranges: Vec<Range>,
    ) -> Result<Vec<Self>, Error> {
        let mut result = Vec::with_capacity(1);
        let mut queue = Vec::new();
        loop {
            if highlighter.parser.set_included_ranges(&ranges).is_ok() {
                highlighter
                    .parser
                    .set_language(config.language)
                    .map_err(|_| Error::InvalidLanguage)?;

                let time = Instant::now();
                let tree = highlighter
                    .parser
                    .parse_with(
                        &mut |byte, _| {
                            if byte <= source.len_bytes() {
                                let (chunk, start_byte, _, _) = source.chunk_at_byte(byte);
                                &chunk.as_bytes()[byte - start_byte..]
                            } else {
                                // out of range
                                &[]
                            }
                        },
                        None,
                    )
                    .ok_or(Error::Cancelled)?;
                tracing::trace!(
                    "parsing took: {}us or {}ms",
                    time.elapsed().as_micros(),
                    time.elapsed().as_millis()
                );

                let mut cursor = highlighter.cursors.pop().unwrap_or(QueryCursor::new());

                // Process combined injections.
                if let Some(combined_injections_query) = &config.combined_injections_query {
                    let mut injections_by_pattern_index =
                        vec![(None, Vec::new(), false); combined_injections_query.pattern_count()];
                    let matches = cursor.matches(
                        combined_injections_query,
                        tree.root_node(),
                        RopeProvider(source.slice(..)),
                    );
                    for mat in matches {
                        let entry = &mut injections_by_pattern_index[mat.pattern_index];
                        let (language_name, content_node, include_children) = injection_for_match(
                            config,
                            combined_injections_query,
                            &mat,
                            source.slice(..),
                        );
                        if language_name.is_some() {
                            entry.0 = language_name;
                        }
                        if let Some(content_node) = content_node {
                            entry.1.push(content_node);
                        }
                        entry.2 = include_children;
                    }
                    for (lang_name, content_nodes, includes_children) in injections_by_pattern_index
                    {
                        if let (Some(lang_name), false) = (lang_name, content_nodes.is_empty())
                            && let Some(next_config) = (injection_callback)(&lang_name)
                        {
                            let ranges =
                                Self::intersect_ranges(&ranges, &content_nodes, includes_children);
                            if !ranges.is_empty() {
                                queue.push((next_config, depth + 1, ranges));
                            }
                        }
                    }
                }

                // The `captures` iterator borrows the `Tree` and the `QueryCursor`, which
                // prevents them from being moved. But both of these values are really just
                // pointers, so it's actually ok to move them.
                let tree_ref = unsafe { mem::transmute::<&Tree, &'static Tree>(&tree) };
                let cursor_ref = unsafe {
                    mem::transmute::<&mut QueryCursor, &'static mut QueryCursor>(&mut cursor)
                };

                cursor_ref.set_match_limit(300);

                let captures = cursor_ref
                    .captures(
                        config.query,
                        tree_ref.root_node(),
                        RopeProvider(source.clone().slice(..)),
                    )
                    .peekable();

                result.push(HighlightIterLayer {
                    highlight_end_stack: Vec::new(),
                    scope_stack: vec![LocalScope {
                        inherits: false,
                        range: 0..usize::MAX,
                        local_defs: Vec::new(),
                    }],
                    cursor,
                    depth,
                    _tree: tree,
                    captures,
                    config,
                    ranges,
                });
            }

            if queue.is_empty() {
                break;
            } else {
                let (next_config, next_depth, next_ranges) = queue.remove(0);
                config = next_config;
                depth = next_depth;
                ranges = next_ranges;
            }
        }

        Ok(result)
    }

    // Compute the ranges that should be included when parsing an injection.
    // This takes into account three things:
    // * `parent_ranges` - The ranges must all fall within the *current* layer's ranges.
    // * `nodes` - Every injection takes place within a set of nodes. The injection ranges
    //   are the ranges of those nodes.
    // * `includes_children` - For some injections, the content nodes' children should be
    //   excluded from the nested document, so that only the content nodes' *own* content
    //   is reparsed. For other injections, the content nodes' entire ranges should be
    //   reparsed, including the ranges of their children.
    fn intersect_ranges(
        parent_ranges: &[Range],
        nodes: &[Node],
        includes_children: bool,
    ) -> Vec<Range> {
        let mut cursor = nodes[0].walk();
        let mut result = Vec::new();
        let mut parent_range_iter = parent_ranges.iter();
        let mut parent_range = parent_range_iter
            .next()
            .expect("Layers should only be constructed with non-empty ranges vectors");
        for node in nodes.iter() {
            let mut preceding_range = Range {
                start_byte: 0,
                start_point: Point::new(0, 0),
                end_byte: node.start_byte(),
                end_point: node.start_position(),
            };
            let following_range = Range {
                start_byte: node.end_byte(),
                start_point: node.end_position(),
                end_byte: usize::MAX,
                end_point: Point::new(usize::MAX, usize::MAX),
            };

            for excluded_range in node
                .children(&mut cursor)
                .filter_map(|child| {
                    if includes_children {
                        None
                    } else {
                        Some(child.range())
                    }
                })
                .chain([following_range].iter().cloned())
            {
                let mut range = Range {
                    start_byte: preceding_range.end_byte,
                    start_point: preceding_range.end_point,
                    end_byte: excluded_range.start_byte,
                    end_point: excluded_range.start_point,
                };
                preceding_range = excluded_range;

                if range.end_byte < parent_range.start_byte {
                    continue;
                }

                while parent_range.start_byte <= range.end_byte {
                    if parent_range.end_byte > range.start_byte {
                        if range.start_byte < parent_range.start_byte {
                            range.start_byte = parent_range.start_byte;
                            range.start_point = parent_range.start_point;
                        }

                        if parent_range.end_byte < range.end_byte {
                            if range.start_byte < parent_range.end_byte {
                                result.push(Range {
                                    start_byte: range.start_byte,
                                    start_point: range.start_point,
                                    end_byte: parent_range.end_byte,
                                    end_point: parent_range.end_point,
                                });
                            }
                            range.start_byte = parent_range.end_byte;
                            range.start_point = parent_range.end_point;
                        } else {
                            if range.start_byte < range.end_byte {
                                result.push(range);
                            }
                            break;
                        }
                    }

                    if let Some(next_range) = parent_range_iter.next() {
                        parent_range = next_range;
                    } else {
                        return result;
                    }
                }
            }
        }
        result
    }

    // First, sort scope boundaries by their byte offset in the document. At a
    // given position, emit scope endings before scope beginnings. Finally, emit
    // scope boundaries from deeper layers first.
    fn sort_key(&mut self) -> Option<(usize, bool, isize)> {
        let depth = -(self.depth as isize);
        let next_start = self
            .captures
            .peek()
            .map(|(m, i)| m.captures[*i].node.start_byte());
        let next_end = self.highlight_end_stack.last().cloned();
        match (next_start, next_end) {
            (Some(start), Some(end)) => {
                if start < end {
                    Some((start, true, depth))
                } else {
                    Some((end, false, depth))
                }
            }
            (Some(i), None) => Some((i, true, depth)),
            (None, Some(j)) => Some((j, false, depth)),
            _ => None,
        }
    }
}

impl<'a, F> HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    fn emit_event(
        &mut self,
        offset: usize,
        event: Option<HighlightEvent>,
    ) -> Option<Result<HighlightEvent, Error>> {
        let result;
        if self.byte_offset < offset {
            result = Some(Ok(HighlightEvent::Source {
                start: self.byte_offset,
                end: offset,
            }));
            self.byte_offset = offset;
            self.next_event = event;
        } else {
            result = event.map(Ok);
        }
        self.sort_layers();
        result
    }

    fn sort_layers(&mut self) {
        while !self.layers.is_empty() {
            if let Some(sort_key) = self.layers[0].sort_key() {
                let mut i = 0;
                while i + 1 < self.layers.len() {
                    if let Some(next_offset) = self.layers[i + 1].sort_key()
                        && next_offset < sort_key
                    {
                        i += 1;
                        continue;
                    }
                    break;
                }
                if i > 0 {
                    self.layers[0..(i + 1)].rotate_left(1);
                }
                break;
            } else {
                let layer = self.layers.remove(0);
                self.highlighter.cursors.push(layer.cursor);
            }
        }
    }

    fn insert_layer(&mut self, mut layer: HighlightIterLayer<'a>) {
        if let Some(sort_key) = layer.sort_key() {
            let mut i = 1;
            while i < self.layers.len() {
                if let Some(sort_key_i) = self.layers[i].sort_key() {
                    if sort_key_i > sort_key {
                        self.layers.insert(i, layer);
                        return;
                    }
                    i += 1;
                } else {
                    self.layers.remove(i);
                }
            }
            self.layers.push(layer);
        }
    }
}

impl<'a, F> Iterator for HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    type Item = Result<HighlightEvent, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        'main: loop {
            // If we've already determined the next highlight boundary, just return it.
            if let Some(e) = self.next_event.take() {
                return Some(Ok(e));
            }

            // If none of the layers have any more highlight boundaries, terminate.
            if self.layers.is_empty() {
                return if self.byte_offset < self.source.len_bytes() {
                    let result = Some(Ok(HighlightEvent::Source {
                        start: self.byte_offset,
                        end: self.source.len_bytes(),
                    }));
                    self.byte_offset = self.source.len_bytes();
                    result
                } else {
                    None
                };
            }

            // Get the next capture from whichever layer has the earliest highlight boundary.
            let range;
            let layer = &mut self.layers[0];
            if let Some((next_match, capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*capture_index];
                range = next_capture.node.byte_range();

                // If any previous highlight ends before this node starts, then before
                // processing this capture, emit the source code up until the end of the
                // previous highlight, and an end event for that highlight.
                if let Some(end_byte) = layer.highlight_end_stack.last().cloned()
                    && end_byte <= range.start
                {
                    layer.highlight_end_stack.pop();
                    return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
                }
            }
            // If there are no more captures, then emit any remaining highlight end events.
            // And if there are none of those, then just advance to the end of the document.
            else if let Some(end_byte) = layer.highlight_end_stack.last().cloned() {
                layer.highlight_end_stack.pop();
                return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
            } else {
                return self.emit_event(self.source.len_bytes(), None);
            };

            let (mut match_, capture_index) = layer.captures.next().unwrap();
            let mut capture = match_.captures[capture_index];

            // If this capture represents an injection, then process the injection.
            if match_.pattern_index < layer.config.locals_pattern_index {
                let (language_name, content_node, include_children) = injection_for_match(
                    layer.config,
                    layer.config.query,
                    &match_,
                    self.source.slice(..),
                );

                // Explicitly remove this match so that none of its other captures will remain
                // in the stream of captures.
                match_.remove();

                // If a language is found with the given name, then add a new language layer
                // to the highlighted document.
                if let (Some(language_name), Some(content_node)) = (language_name, content_node)
                    && let Some(config) = (self.injection_callback)(&language_name)
                {
                    let ranges = HighlightIterLayer::intersect_ranges(
                        &self.layers[0].ranges,
                        &[content_node],
                        include_children,
                    );
                    if !ranges.is_empty() {
                        match HighlightIterLayer::new(
                            self.source,
                            self.highlighter,
                            &mut self.injection_callback,
                            config,
                            self.layers[0].depth + 1,
                            ranges,
                        ) {
                            Ok(layers) => {
                                for layer in layers {
                                    self.insert_layer(layer);
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                }

                self.sort_layers();
                continue 'main;
            }

            // Remove from the local scope stack any local scopes that have already ended.
            while range.start > layer.scope_stack.last().unwrap().range.end {
                layer.scope_stack.pop();
            }

            // If this capture is for tracking local variables, then process the
            // local variable info.
            let mut reference_highlight = None;
            let mut definition_highlight = None;
            while match_.pattern_index < layer.config.highlights_pattern_index {
                // If the node represents a local scope, push a new local scope onto
                // the scope stack.
                if Some(capture.index) == layer.config.local_scope_capture_index {
                    definition_highlight = None;
                    let mut scope = LocalScope {
                        inherits: true,
                        range: range.clone(),
                        local_defs: Vec::new(),
                    };
                    for prop in layer.config.query.property_settings(match_.pattern_index) {
                        if prop.key.as_ref() == "local.scope-inherits" {
                            scope.inherits =
                                prop.value.as_ref().is_none_or(|r| r.as_ref() == "true");
                        }
                    }
                    layer.scope_stack.push(scope);
                }
                // If the node represents a definition, add a new definition to the
                // local scope at the top of the scope stack.
                else if Some(capture.index) == layer.config.local_def_capture_index {
                    reference_highlight = None;
                    definition_highlight = None;
                    let scope = layer.scope_stack.last_mut().unwrap();

                    let mut value_range = 0..0;
                    for capture in match_.captures {
                        if Some(capture.index) == layer.config.local_def_value_capture_index {
                            value_range = capture.node.byte_range();
                        }
                    }

                    if let Some(name) = self.source.get_byte_slice(range.clone()) {
                        scope.local_defs.push(LocalDef {
                            name,
                            value_range,
                            highlight: None,
                        });
                        definition_highlight =
                            scope.local_defs.last_mut().map(|s| &mut s.highlight);
                    }
                }
                // If the node represents a reference, then try to find the corresponding
                // definition in the scope stack.
                else if Some(capture.index) == layer.config.local_ref_capture_index
                    && definition_highlight.is_none()
                {
                    definition_highlight = None;
                    if let Some(name) = self.source.get_byte_slice(range.clone()) {
                        for scope in layer.scope_stack.iter().rev() {
                            if let Some(highlight) = scope.local_defs.iter().rev().find_map(|def| {
                                if def.name == name && range.start >= def.value_range.end {
                                    Some(def.highlight)
                                } else {
                                    None
                                }
                            }) {
                                reference_highlight = highlight;
                                break;
                            }
                            if !scope.inherits {
                                break;
                            }
                        }
                    }
                }

                // Continue processing any additional matches for the same node.
                if let Some((next_match, next_capture_index)) = layer.captures.peek() {
                    let next_capture = next_match.captures[*next_capture_index];
                    if next_capture.node == capture.node {
                        capture = next_capture;
                        match_ = layer.captures.next().unwrap().0;
                        continue;
                    }
                }

                self.sort_layers();
                continue 'main;
            }

            // Otherwise, this capture must represent a highlight.
            // If this exact range has already been highlighted by an earlier pattern, or by
            // a different layer, then skip over this one.
            if let Some((last_start, last_end, last_depth)) = self.last_highlight_range
                && range.start == last_start
                && range.end == last_end
                && layer.depth < last_depth
            {
                self.sort_layers();
                continue 'main;
            }

            // If the current node was found to be a local variable, then skip over any
            // highlighting patterns that are disabled for local variables.
            if definition_highlight.is_some() || reference_highlight.is_some() {
                while layer.config.non_local_variable_patterns[match_.pattern_index] {
                    match_.remove();
                    if let Some((next_match, next_capture_index)) = layer.captures.peek() {
                        let next_capture = next_match.captures[*next_capture_index];
                        if next_capture.node == capture.node {
                            capture = next_capture;
                            match_ = layer.captures.next().unwrap().0;
                            continue;
                        }
                    }

                    self.sort_layers();
                    continue 'main;
                }
            }

            // Once a highlighting pattern is found for the current node, skip over
            // any later highlighting patterns that also match this node. Captures
            // for a given node are ordered by pattern index, so these subsequent
            // captures are guaranteed to be for highlighting, not injections or
            // local variables.
            while let Some((next_match, next_capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*next_capture_index];
                if next_capture.node == capture.node {
                    layer.captures.next();
                } else {
                    break;
                }
            }

            let current_highlight = Some(Highlight {
                capture_index: capture.index as usize,
                query: layer.config.query,
            });

            // If this node represents a local definition, then store the current
            // highlight value on the local scope entry representing this node.
            if let Some(definition_highlight) = definition_highlight {
                *definition_highlight = current_highlight;
            }

            // Emit a scope start event and push the node's end position to the stack.
            if let Some(highlight) = reference_highlight.or(current_highlight) {
                self.last_highlight_range = Some((range.start, range.end, layer.depth));
                layer.highlight_end_stack.push(range.end);
                return self
                    .emit_event(range.start, Some(HighlightEvent::HighlightStart(highlight)));
            }

            self.sort_layers();
        }
    }
}

fn injection_for_match<'a>(
    config: &HighlightConfiguration,
    query: &'a Query,
    query_match: &QueryMatch<'a, 'a>,
    source: RopeSlice<'a>,
) -> (Option<Cow<'a, str>>, Option<Node<'a>>, bool) {
    let content_capture_index = config.injection_content_capture_index;
    let language_capture_index = config.injection_language_capture_index;

    let mut language_name = None;
    let mut content_node = None;
    for capture in query_match.captures {
        let index = Some(capture.index);
        if index == language_capture_index {
            let name = byte_range_to_str(capture.node.byte_range(), source);
            language_name = Some(name);
        } else if index == content_capture_index {
            content_node = Some(capture.node);
        }
    }

    let mut include_children = false;
    for prop in query.property_settings(query_match.pattern_index) {
        match prop.key.as_ref() {
            // In addition to specifying the language name via the text of a
            // captured node, it can also be hard-coded via a `#set!` predicate
            // that sets the injection.language key.
            "injection.language" => {
                if language_name.is_none() {
                    language_name = prop.value.as_ref().map(|s| s.as_ref().into())
                }
            }

            // By default, injections do not include the *children* of an
            // `injection.content` node - only the ranges that belong to the
            // node itself. This can be changed using a `#set!` predicate that
            // sets the `injection.include-children` key.
            "injection.include-children" => include_children = true,
            _ => {}
        }
    }

    (language_name, content_node, include_children)
}

fn byte_range_to_str(range: std::ops::Range<usize>, source: RopeSlice) -> Cow<str> {
    Cow::from(source.byte_slice(range))
}
