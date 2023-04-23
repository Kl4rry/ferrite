use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
};

use anyhow::Result;
use ropey::{Rope, RopeSlice};
use tree_sitter::{Node, Parser, Point, Query, QueryCursor, TextProvider, Tree};

use super::{get_tree_sitter_language, LanguageConfig};
use crate::tui_app::event_loop::TuiEventLoopProxy;

struct SyntaxProvider {
    pub language: LanguageConfig,
    pub rope_tx: Sender<Rope>,
}

impl SyntaxProvider {
    pub fn new(
        language: LanguageConfig,
        proxy: TuiEventLoopProxy,
        result: Arc<Mutex<Option<(Rope, Tree)>>>,
    ) -> Result<Self> {
        let (rope_tx, rope_rx) = mpsc::channel::<Rope>();

        let mut parser = Parser::new();
        parser.set_language(language.grammar)?;

        thread::spawn(move || {
            parser.reset();

            loop {
                let rope = match rope_rx.recv() {
                    Ok(rope) => rope,
                    Err(err) => {
                        log::error!("Recv error: {err}");
                        break;
                    }
                };

                let tree = parser.parse_with(
                    &mut |byte_idx, _point| match rope.get_chunk_at_byte(byte_idx) {
                        Some((chunk, chunk_byte_idx, _chunk_char_idx, _chunk_line_idx)) => {
                            let diff = byte_idx - chunk_byte_idx;
                            &chunk.as_bytes()[diff..]
                        }
                        None => &[],
                    },
                    None,
                );

                if let Some(tree) = tree {
                    *result.lock().unwrap() = Some((rope, tree));
                    proxy.request_render();
                }
            }

            log::info!("Syntax provider thread exit");
        });

        Ok(Self { language, rope_tx })
    }

    pub fn update_text(&self, rope: Rope) {
        let _ = self.rope_tx.send(rope);
    }
}

pub struct Syntax {
    syntax_provder: Option<SyntaxProvider>,
    result: Arc<Mutex<Option<(Rope, Tree)>>>,
    proxy: TuiEventLoopProxy,
}

impl Syntax {
    pub fn new(proxy: TuiEventLoopProxy) -> Self {
        Self {
            syntax_provder: None,
            result: Arc::new(Mutex::new(None)),
            proxy,
        }
    }

    pub fn set_language(&mut self, language: &str) -> Result<()> {
        self.syntax_provder = None;
        *self.result.lock().unwrap() = None;

        if let Some(lang) = get_tree_sitter_language(language) {
            log::debug!("set lang to '{language}'");
            self.syntax_provder = Some(SyntaxProvider::new(
                lang,
                self.proxy.clone(),
                self.result.clone(),
            )?);
        }

        Ok(())
    }

    pub fn get_language_name(&self) -> Option<String> {
        Some(self.syntax_provder.as_ref()?.language.name.to_string())
    }

    pub fn update_text(&mut self, rope: Rope) {
        if let Some(syntax) = &self.syntax_provder {
            syntax.update_text(rope);
        }
    }

    pub fn query_highlight(&mut self, start_byte: usize, end_byte: usize) -> Vec<NodeSpan> {
        if let (Some(syntax_provider), Some((rope, tree))) =
            (&self.syntax_provder, &*self.result.lock().unwrap())
        {
            let rope = RopeProvider(rope.slice(..));
            let query = syntax_provider.language.highlight_query.clone();
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(start_byte..end_byte);
            let captures = cursor.captures(&query, tree.root_node(), rope);
            let mut spans = Vec::new();
            for (m, index) in captures {
                let capture = m.captures[index];
                spans.push(NodeSpan {
                    start: capture.node.start_position(),
                    end: capture.node.end_position(),
                    start_byte: capture.node.start_byte(),
                    end_byte: capture.node.end_byte(),
                    index: capture.index as usize,
                    query: query.clone(),
                });
            }
            return spans;
        }
        Vec::new()
    }
}

pub struct NodeSpan {
    pub start: Point,
    pub end: Point,
    pub start_byte: usize,
    pub end_byte: usize,
    index: usize,
    query: Arc<Query>,
}

impl NodeSpan {
    pub fn name(&self) -> &str {
        &self.query.capture_names()[self.index]
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
