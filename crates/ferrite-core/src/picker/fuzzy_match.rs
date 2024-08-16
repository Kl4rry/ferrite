use std::{cmp, path::Path};

use rayon::prelude::*;
use sublime_fuzzy::{ContinuousMatch, FuzzySearch, Scoring};

use super::Matchable;

#[derive(Debug, Clone)]
pub struct FuzzyMatch<T: Matchable> {
    pub score: i64,
    pub proximity: i64,
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
        Some(self.cmp(other))
    }
}

impl<T: Matchable> Ord for FuzzyMatch<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.score.cmp(&other.score) {
            cmp::Ordering::Equal => match self.proximity.cmp(&other.proximity) {
                cmp::Ordering::Equal => lexical_sort::natural_lexical_cmp(
                    &self.item.as_match_str(),
                    &self.item.as_match_str(),
                ),
                cmp::Ordering::Greater => cmp::Ordering::Less,
                cmp::Ordering::Less => cmp::Ordering::Greater,
            },
            cmp::Ordering::Greater => cmp::Ordering::Less,
            cmp::Ordering::Less => cmp::Ordering::Greater,
        }
    }
}

pub fn fuzzy_match<'a, T>(
    term: &str,
    items: &'a boxcar::Vec<T>,
    path: Option<&Path>,
) -> Vec<(FuzzyMatch<T>, usize)>
where
    &'a T: Send + Sync,
    T: Matchable + Send + Sync,
{
    let scoring = Scoring::emphasize_distance();
    let mut matches: Vec<_> = items
        .iter()
        .par_bridge()
        .filter_map(|(i, item)| {
            let item = item.clone();
            if term.is_empty() {
                return Some((
                    FuzzyMatch {
                        score: 0,
                        proximity: 0,
                        item,
                        matches: Vec::new(),
                    },
                    i,
                ));
            }

            let proximity = match path {
                Some(path) => {
                    let mut missed = false;
                    let mut path = path.iter();
                    Path::new(&*item.as_match_str())
                        .components()
                        .skip_while(|c| matches!(c, std::path::Component::CurDir))
                        .map(|c| {
                            // if we've already missed, each additional dir is one further away
                            if missed {
                                return -1;
                            }

                            // we want to score positively if c matches the next segment from target path
                            if let Some(p) = path.next() {
                                if p == c.as_os_str() {
                                    // matching path segment!
                                    return 1;
                                } else {
                                    // non-matching path segment
                                    missed = true;
                                }
                            }

                            -1
                        })
                        .sum()
                }
                None => 0,
            };

            let mut score = 0;
            let mut matches = Vec::new();

            for term in term.split_ascii_whitespace() {
                if term.is_empty() {
                    continue;
                }
                if let Some(m) = FuzzySearch::new(term, &item.as_match_str())
                    .score_with(&scoring)
                    .best_match()
                {
                    score += m.score() as i64;
                    matches.extend(m.continuous_matches().map(|m| MatchIndex::from(m)));
                } else {
                    return None;
                }
            }

            matches.sort_by(|a, b| a.start.cmp(&b.start));

            Some((
                FuzzyMatch {
                    score,
                    proximity,
                    item,
                    matches,
                },
                i,
            ))
        })
        .collect();

    matches.par_sort();
    matches
}
