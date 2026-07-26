use ferrite_ctx::{ArenaString, ArenaVec};
use ropey::{Rope, RopeSlice};

mod patterns;

/// returns language with highest score
#[profiling::function]
fn score_languages(text: &Rope) -> (&'static str, i32) {
    let arena = ferrite_ctx::Ctx::arena();

    let mut result = ArenaVec::new_in(&arena);
    let mut text_short = ArenaString::with_capacity_in(8192, &arena);

    for (language, patterns) in patterns::LANGUAGES {
        let mut temp = ArenaVec::new_in(&arena);
        for (pattern, score) in *patterns {
            let regex = match regex::RegexBuilder::new(pattern).multi_line(true).build() {
                Ok(regex) => regex,
                Err(err) => panic!("Error: {} is not a valid regex\n{err}", pattern),
            };
            temp.push((regex, score));
        }

        text_short.clear();
        text_short.extend(text.slice(..text.len_chars().min(8192)).chunks());

        let mut total = 0;
        for (pattern, score) in temp {
            if pattern.is_match(&text_short) {
                total += score;
            }
        }
        result.push((*language, total));
    }

    result.sort_by_key(|v| -v.1);

    return result[0];
}

#[profiling::function]
pub fn detect_language(inital_guess: Option<&str>, text: Rope) -> Option<&'static str> {
    tracing::debug!("inital_guess: {inital_guess:?}");

    let (language, score) = score_languages(&text);
    tracing::debug!("top scoring language: {} {}", language, score);
    match inital_guess {
        Some(_) if score >= 15 => Some(language),
        None if score >= 10 => Some(language),
        _ => return None,
    }
}

#[profiling::function]
fn detect_markers(text: RopeSlice, markers: &[&str]) -> usize {
    let start = text.slice(..text.len_chars().min(1000)).to_string();
    let mut count = 0;
    for marker in markers {
        count += start.contains(marker) as usize;
    }
    count
}
