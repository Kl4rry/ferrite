use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ferrite_ctx::ArenaVec;
use ferrite_utility::{utf32::ArenaUtf32, words};
use ropey::Rope;

use crate::timer::Timer;

pub struct CompletionSource {
    pub words: Arc<Mutex<Vec<String>>>,
    pub debounce_timer: Timer,
}

impl CompletionSource {
    pub fn new() -> Self {
        Self {
            words: Arc::new(Mutex::new(Vec::new())),
            debounce_timer: Timer::default(),
        }
    }

    pub fn update_words(&mut self, rope: Rope) {
        if self.debounce_timer.every(Duration::from_secs(3)) {
            let words = self.words.clone();
            tracing::debug!("update words in buffer");
            rayon::spawn(move || {
                let w = words::parse_words(&rope);
                *words.lock().unwrap() = w;
            });
        }
    }
}

impl Default for CompletionSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CompletionSource {
    fn clone(&self) -> Self {
        Self {
            words: Arc::new(Mutex::new(self.words.lock().unwrap().clone())),
            debounce_timer: self.debounce_timer.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Completer {
    pub matching_words: Vec<String>,
    pub last_query: String,
    pub visible: bool,
    pub index: usize,
}

impl Completer {
    pub fn new() -> Self {
        Self {
            matching_words: Vec::new(),
            last_query: String::new(),
            visible: false,
            index: 0,
        }
    }

    pub fn can_complete(&self) -> bool {
        self.visible && !self.matching_words.is_empty()
    }

    pub fn get_completion(&self) -> String {
        self.matching_words[self.index].clone()
    }

    pub fn next(&mut self) {
        self.index += 1;
        if self.index >= self.matching_words.len() {
            self.index = 0;
        }
    }

    pub fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.matching_words.len().saturating_sub(1);
            return;
        }
        self.index -= 1;
    }

    #[profiling::function]
    pub fn update_query(&mut self, words: Arc<Mutex<Vec<String>>>, query: String) {
        tracing::debug_span!("update_query");
        let arena = ferrite_ctx::Ctx::arena();
        self.last_query.clear();
        self.last_query.push_str(&query);

        if query.is_empty() {
            self.matching_words.clear();
            self.index = 0;
            return;
        }

        let guard = words.lock().unwrap();
        let mut matches = ArenaVec::new_in(&arena);
        {
            profiling::scope!("fuzzy search");
            tracing::debug!("fuzzy searching {} words", guard.len());
            let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);
            let needle = ArenaUtf32::from_str_in(&query, &arena);
            for haystack in &*guard {
                let haystack_utf32 = ArenaUtf32::from_str_in(haystack, &arena);
                if let Some(score) =
                    matcher.fuzzy_match(haystack_utf32.as_utf32_str(), needle.as_utf32_str())
                {
                    matches.push((score, haystack));
                }
            }
        }
        matches.sort_by_key(|(score, _)| *score);
        tracing::debug!("fuzzy match done");
        self.matching_words.clear();
        self.matching_words
            .extend(matches.into_iter().map(|(_, m)| m.clone()));
        self.index = 0;
        if self.matching_words.is_empty() {
            self.visible = false;
        }
    }
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}
