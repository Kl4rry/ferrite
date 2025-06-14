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
        if self.debounce_timer.every(Duration::from_secs(5)) {
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
    pub visible: bool,
    pub index: usize,
}

impl Completer {
    pub fn new() -> Self {
        Self {
            matching_words: Vec::new(),
            visible: false,
            index: 0,
        }
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
        matches.sort();
        self.matching_words.clear();
        self.matching_words
            .extend(matches.into_iter().map(|(_, s)| s.to_string()));
        self.index = 0;
    }
}
