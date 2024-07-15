use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use ferrite_utility::{graphemes::RopeGraphemeExt as _, point::Point};
use ropey::Rope;

use crate::event_loop_proxy::EventLoopProxy;

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
    match_index: usize,
    tx: mpsc::Sender<QueryUpdate>,
}

impl BufferSearcher {
    pub fn new(
        proxy: Box<dyn EventLoopProxy>,
        query: String,
        rope: Rope,
        case_insensitive: bool,
        cursor_pos: usize,
    ) -> Self {
        let matches = Arc::new(Mutex::new((Vec::new(), None)));
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
            let mut cursor_pos = Some(cursor_pos);

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
                    QueryUpdate::Query(q, case, cursor) => {
                        case_insensitive = case;
                        query = q;
                        cursor_pos = Some(cursor);
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
                        let start_byte = rope.char_to_byte(current_char - chars.len());
                        let end_byte = rope.char_to_byte(current_char);
                        match_buffer.push(SearchMatch {
                            start: rope.byte_to_point(start_byte),
                            end: rope.byte_to_point(end_byte),
                            start_byte,
                            end_byte,
                        });
                        query_idx = 0;
                    }
                    current_char += 1;
                }

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
        } else {
            if self.match_index == 0 {
                self.match_index = guard.0.len().saturating_sub(1);
            } else {
                self.match_index = self.match_index.saturating_sub(1);
            }
        }
        guard.0.get(self.match_index).copied()
    }

    pub fn update_query(&mut self, query: String, case_insensitive: bool, cursor_pos: usize) {
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
}

#[inline(always)]
pub fn compare_char(lhs: &char, rhs: &char, case_insensitive: bool) -> bool {
    if case_insensitive {
        lhs.to_ascii_lowercase() == rhs.to_ascii_lowercase()
    } else {
        lhs == rhs
    }
}
