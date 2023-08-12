use std::cmp;

use rayon::prelude::*;
use sublime_fuzzy::{ContinuousMatch, FuzzySearch, Scoring};

use super::Matchable;

#[derive(Debug, Clone)]
pub struct FuzzyMatch<T: Matchable> {
    pub score: i64,
    pub item: T,
    pub matches: Vec<MatchIndex>,
}

#[derive(Debug, Clone, Copy)]
pub struct MatchIndex {
    pub start: usize,
    pub len: usize,
}

impl From<ContinuousMatch> for MatchIndex {
    fn from(value: ContinuousMatch) -> Self {
        Self {
            start: value.start(),
            len: value.len(),
        }
    }
}

impl<T: Matchable> PartialEq for FuzzyMatch<T> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.item.as_match_str() == other.item.as_match_str()
    }
}

impl<T: Matchable> Eq for FuzzyMatch<T> {}

impl<T: Matchable> PartialOrd for FuzzyMatch<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.score.partial_cmp(&other.score) {
            Some(cmp::Ordering::Equal) => (),
            Some(cmp::Ordering::Greater) => return Some(cmp::Ordering::Less),
            Some(cmp::Ordering::Less) => return Some(cmp::Ordering::Greater),
            ord => return ord,
        }
        Some(lexical_sort::natural_lexical_cmp(
            &self.item.as_match_str(),
            &self.item.as_match_str(),
        ))
    }
}

impl<T: Matchable> Ord for FuzzyMatch<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub fn fuzzy_match<T: Send + Sync + Matchable>(term: &str, items: Vec<T>) -> Vec<FuzzyMatch<T>> {
    let scoring = Scoring::emphasize_distance();
    let mut matches: Vec<_> = items
        .into_par_iter()
        .filter_map(|item| {
            if term.is_empty() {
                return Some(FuzzyMatch {
                    score: 0,
                    item,
                    matches: Vec::new(),
                });
            }

            FuzzySearch::new(term, &item.as_match_str())
                .score_with(&scoring)
                .best_match()
                .map(|m| FuzzyMatch {
                    score: m.score() as i64,
                    item,
                    matches: m.continuous_matches().map(|m| m.into()).collect(),
                })
        })
        .collect();

    matches.sort();
    matches
}
