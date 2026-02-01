use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use ferrite_geom::point::Point;
use ferrite_utility::{graphemes::RopeGraphemeExt as _, rope_reader::RopeReader};
use grep_matcher::Matcher;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, sinks::UTF8};
use ropey::{Rope, RopeSlice};

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

#[derive(Debug, Clone, Copy)]
pub struct SearchMatch {
    pub start: Point<usize>,
    pub end: Point<usize>,
    pub start_byte: usize,
    pub end_byte: usize,
}

enum QueryUpdate {
    Rope(Rope, Option<bool>),
    Query(String, bool, usize),
}

pub struct BufferSearcher {
    matches: Arc<Mutex<(Vec<SearchMatch>, Option<usize>)>>,
    last_rope: Rope,
    last_query: String,
    match_index: usize,
    tx: mpsc::Sender<QueryUpdate>,
}

impl BufferSearcher {
    pub fn new(
        proxy: Box<dyn EventLoopProxy<UserEvent>>,
        query: String,
        rope: Rope,
        case_insensitive: bool,
        cursor_pos: usize,
    ) -> Self {
        let matches = Arc::new(Mutex::new((Vec::new(), None)));
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(QueryUpdate::Rope(rope.clone(), Some(case_insensitive)));
        let thread_rope = rope.clone();
        let last_query = query.clone();

        let thread_matches = matches.clone();
        thread::spawn(move || {
            tracing::info!("search thread spawned");
            let matches = thread_matches;
            let mut query = query;
            let mut rope = thread_rope;
            let mut case_insensitive = case_insensitive;
            let mut cursor_pos = Some(cursor_pos);

            // TODO don't block on every update do batch reciving
            while let Ok(update) = rx.recv() {
                match update {
                    QueryUpdate::Rope(r, case) => {
                        if let Some(case) = case {
                            case_insensitive = case;
                        }
                        rope = r;
                    }
                    QueryUpdate::Query(q, case, cursor) => {
                        case_insensitive = case;
                        query = q;
                        cursor_pos = Some(cursor);
                    }
                }

                let match_buffer =
                    search_rope(rope.slice(..), query.clone(), case_insensitive, false);

                let mut index = match cursor_pos.take() {
                    Some(cursor_pos) => {
                        let mut index = 0;
                        for (i, m) in match_buffer.iter().enumerate() {
                            if m.end_byte > cursor_pos {
                                index = i;
                                break;
                            }
                        }
                        Some(index)
                    }
                    None => None,
                };

                {
                    let mut guard = matches.lock().unwrap();
                    guard.0.clear();
                    guard.0.extend_from_slice(&match_buffer);
                    if index.is_some() {
                        guard.1 = index.take();
                    }
                }

                proxy.request_render("search results ready");
            }
            tracing::info!("search thread exit");
        });

        Self {
            matches,
            tx,
            last_query,
            last_rope: rope,
            match_index: usize::MAX - 1,
        }
    }

    pub fn get_next_match(&mut self) -> Option<SearchMatch> {
        let mut guard = self.matches.lock().unwrap();
        if let Some(index) = guard.1.take() {
            self.match_index = index.min(guard.0.len().saturating_sub(1));
        } else {
            self.match_index += 1;
            if self.match_index >= guard.0.len() {
                self.match_index = 0;
            }
        }
        guard.0.get(self.match_index).copied()
    }

    pub fn get_prev_match(&mut self) -> Option<SearchMatch> {
        let mut guard = self.matches.lock().unwrap();
        if let Some(index) = guard.1.take() {
            self.match_index = index.min(guard.0.len().saturating_sub(1));
        } else if self.match_index == 0 {
            self.match_index = guard.0.len().saturating_sub(1);
        } else {
            self.match_index = self.match_index.saturating_sub(1);
        }
        guard.0.get(self.match_index).copied()
    }

    pub fn get_current_match(&mut self) -> Option<SearchMatch> {
        self.matches
            .lock()
            .unwrap()
            .0
            .get(self.match_index)
            .copied()
    }

    pub fn update_query(&mut self, query: String, case_insensitive: bool, cursor_pos: usize) {
        self.last_query = query.clone();
        let _ = self
            .tx
            .send(QueryUpdate::Query(query, case_insensitive, cursor_pos));
    }

    pub fn update_buffer(&mut self, rope: Rope, case_insensitive: Option<bool>) {
        if !self.last_rope.is_instance(&rope) {
            let _ = self.tx.send(QueryUpdate::Rope(rope, case_insensitive));
        }
    }

    pub fn get_matches(&self) -> Arc<Mutex<(Vec<SearchMatch>, Option<usize>)>> {
        self.matches.clone()
    }

    pub fn get_last_query(&self) -> String {
        self.last_query.clone()
    }

    pub fn get_match_index_and_match_count(&self) -> (usize, usize) {
        let guard = self.matches.lock().unwrap();
        let count = guard.0.len();
        let index = self.match_index.min(count.saturating_sub(1));
        (index, guard.0.len())
    }
}

pub fn search_rope(
    rope: RopeSlice,
    query: String,
    case_insensitive: bool,
    stop_at_first: bool,
) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    let matcher = RegexMatcherBuilder::new()
        .fixed_strings(true)
        .multi_line(false)
        .case_insensitive(case_insensitive)
        .build(&query)
        .unwrap();

    let max_matches: Option<u64> = if stop_at_first { Some(1) } else { None };

    if let Err(err) = SearcherBuilder::new()
        .max_matches(max_matches)
        .build()
        .search_reader(
            &matcher,
            RopeReader::new(rope),
            UTF8(|line_number, line| {
                if let Some(mymatch) = matcher.find(line.as_bytes())? {
                    let line_number = line_number as usize - 1;
                    let rope_line = rope.line(line_number);
                    let line_start_byte = rope.line_to_byte(line_number);

                    let start_byte = mymatch.start();
                    let end_byte = mymatch.end();
                    let start_col = rope_line.byte_to_col(start_byte);
                    let end_col = rope_line.byte_to_col(end_byte);
                    matches.push(SearchMatch {
                        start_byte: start_byte + line_start_byte,
                        end_byte: end_byte + line_start_byte,
                        start: Point::new(start_col, line_number),
                        end: Point::new(end_col, line_number),
                    });
                }
                Ok(true)
            }),
        )
    {
        tracing::error!("Search error: {err}");
    }

    matches
}
