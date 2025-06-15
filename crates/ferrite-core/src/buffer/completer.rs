use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ferrite_utility::words;
use rayon::prelude::*;
use ropey::Rope;
use sublime_fuzzy::{FuzzySearch, Scoring};

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
            rayon::spawn(move || {
                let w = words::parse_words(&rope);
                *words.lock().unwrap() = w;
            });
        }
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
        self.index = (self.index + 1) % self.matching_words.len();
    }

    pub fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.matching_words.len().saturating_sub(1);
            return;
        }
        self.index -= 1;
    }

    pub fn update_query(&mut self, words: Arc<Mutex<Vec<String>>>, query: String) {
        self.last_query.clear();
        self.last_query.push_str(&query);

        if query.is_empty() {
            self.matching_words.clear();
            self.index = 0;
            return;
        }

        let scoring = Scoring::emphasize_word_starts();
        let guard = words.lock().unwrap();
        let mut matches: Vec<(isize, &str)> = guard
            .par_iter()
            .filter_map(|word| {
                if let Some(m) = FuzzySearch::new(&query, &word)
                    .score_with(&scoring)
                    .best_match()
                {
                    Some((m.score(), word.as_str()))
                } else {
                    None
                }
            })
            .collect();
        matches.sort_by(|a, b| a.0.cmp(&b.0));
        matches.reverse();
        self.matching_words.clear();
        self.matching_words
            .extend(matches.into_iter().map(|(_, s)| s.to_string()));
        self.index = 0;
    }
}
