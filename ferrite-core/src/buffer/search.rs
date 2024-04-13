use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use ropey::Rope;
use utility::{graphemes::RopeGraphemeExt as _, point::Point};

use crate::tui_app::event_loop::TuiEventLoopProxy;

#[derive(Debug, Clone, Copy)]
pub struct SearchMatch {
    pub start: Point<usize>,
    pub end: Point<usize>,
}

enum QueryUpdate {
    Rope(Rope, Option<bool>),
    Query(String, bool),
}

pub struct BufferSearcher {
    matches: Arc<Mutex<Vec<SearchMatch>>>,
    last_rope: Rope,
    match_index: usize,
    tx: mpsc::Sender<QueryUpdate>,
}

impl BufferSearcher {
    pub fn new(
        proxy: TuiEventLoopProxy,
        query: String,
        rope: Rope,
        case_insensitive: bool,
    ) -> Self {
        let matches = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(QueryUpdate::Rope(rope.clone(), Some(case_insensitive)));
        let thread_rope = rope.clone();

        let thread_matches = matches.clone();
        thread::spawn(move || {
            tracing::info!("search thread spawned");
            let matches = thread_matches;
            let mut query = query;
            let mut rope = thread_rope;
            let mut case_insensitive = case_insensitive;

            let mut match_buffer = Vec::new();

            // TODO don't block on every update do batch reciving
            while let Ok(update) = rx.recv() {
                match update {
                    QueryUpdate::Rope(r, case) => {
                        if let Some(case) = case {
                            case_insensitive = case;
                        }
                        rope = r;
                    }
                    QueryUpdate::Query(q, case) => {
                        case_insensitive = case;
                        query = q;
                    }
                }

                let chars: Vec<_> = query.chars().collect();
                let mut query_idx = 0;
                let mut current_char = 1;

                for ch in rope.chars() {
                    if compare_char(&ch, &chars[query_idx], case_insensitive) {
                        query_idx += 1;
                    } else {
                        query_idx = 0;
                        if compare_char(&ch, &chars[query_idx], case_insensitive) {
                            query_idx += 1;
                        }
                    }

                    if query_idx >= chars.len() {
                        match_buffer.push(SearchMatch {
                            start: rope
                                .byte_to_point(rope.char_to_byte(current_char - chars.len())),
                            end: rope.byte_to_point(rope.char_to_byte(current_char)),
                        });
                        query_idx = 0;
                    }
                    current_char += 1;
                }

                {
                    let mut guard = matches.lock().unwrap();
                    guard.clear();
                    guard.extend_from_slice(&match_buffer);
                }

                proxy.request_render();
                match_buffer.clear();
            }
            tracing::info!("search thread exit");
        });

        Self {
            matches,
            tx,
            last_rope: rope,
            match_index: usize::MAX - 1,
        }
    }

    pub fn get_next_match(&mut self) -> Option<SearchMatch> {
        let guard = self.matches.lock().unwrap();
        self.match_index += 1;
        if self.match_index >= guard.len() {
            self.match_index = 0;
        }
        guard.get(self.match_index).copied()
    }

    pub fn get_prev_match(&mut self) -> Option<SearchMatch> {
        let guard = self.matches.lock().unwrap();
        if self.match_index == 0 {
            self.match_index = guard.len().saturating_sub(1);
        }
        self.match_index -= 1;
        guard.get(self.match_index).copied()
    }

    pub fn update_query(&mut self, query: String, case_insensitive: bool) {
        let _ = self.tx.send(QueryUpdate::Query(query, case_insensitive));
    }

    pub fn update_buffer(&mut self, rope: Rope, case_insensitive: Option<bool>) {
        if !self.last_rope.is_instance(&rope) {
            let _ = self.tx.send(QueryUpdate::Rope(rope, case_insensitive));
        }
    }

    pub fn get_matches(&self) -> Arc<Mutex<Vec<SearchMatch>>> {
        self.matches.clone()
    }
}

#[inline(always)]
pub fn compare_char(lhs: &char, rhs: &char, case_insensitive: bool) -> bool {
    if case_insensitive {
        lhs.to_ascii_lowercase() == rhs.to_ascii_lowercase()
    } else {
        lhs == rhs
    }
}
