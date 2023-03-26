use rayon::prelude::*;

use super::Matchable;

pub fn fuzzy_match<T: Send + Sync + Matchable>(term: &str, items: Vec<T>) -> Vec<T> {
    let keywords: Vec<String> = term.split_whitespace().map(|s| s.to_lowercase()).collect();
    items
        .into_par_iter()
        .map(|item| {
            let mut score = 0;
            for keyword in &keywords {
                score += item.as_match_str().to_lowercase().contains(keyword) as usize;
            }
            (item, score)
        })
        .filter(|(_, score)| *score >= keywords.len())
        .map(|(item, _)| item)
        .collect()
}
