use std::collections::HashMap;

use ropey::{Rope, RopeSlice};

use crate::graphemes::is_word_char;

pub fn count_words<'a>(words: &mut HashMap<RopeSlice<'a>, usize>, rope: &'a Rope) {
    let mut word_start = 0;
    let mut is_on_word = false;
    for (i, ch) in rope.chars().enumerate() {
        let is_word_char = is_word_char(ch);
        if !is_on_word && is_word_char {
            word_start = i;
            is_on_word = true;
            continue;
        }

        if is_on_word && !is_word_char {
            is_on_word = false;
            let slice = rope.slice(word_start..i);
            if slice.chars().any(|ch| !ch.is_ascii_digit()) {
                let count = words.entry(slice).or_default();
                *count += 1;
            }
        }
    }

    if is_on_word {
        let slice = rope.slice(word_start..rope.len_chars());
        if slice.chars().any(|ch| !ch.is_ascii_digit()) {
            let count = words.entry(slice).or_default();
            *count += 1;
        }
    }
}

pub fn parse_words(rope: &Rope) -> Vec<String> {
    let mut words = HashMap::new();
    count_words(&mut words, rope);
    words.keys().map(|rope| rope.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use std::{hint::black_box, time::Instant};

    use super::*;

    #[test]
    fn count_words_test() {
        let text = Rope::from_str("oof doof bbb\naaa oof oof choo_f moo-o");
        let mut words = HashMap::new();
        count_words(&mut words, &text);
        assert_eq!(words[&RopeSlice::from("oof")], 3);
        assert_eq!(words[&RopeSlice::from("doof")], 1);
        assert_eq!(words[&RopeSlice::from("bbb")], 1);
        assert_eq!(words[&RopeSlice::from("aaa")], 1);
        assert_eq!(words[&RopeSlice::from("choo_f")], 1);
        assert_eq!(words[&RopeSlice::from("moo")], 1);
        assert_eq!(words[&RopeSlice::from("o")], 1);
        assert!(words.get(&RopeSlice::from("a")).is_none());
    }

    #[test]
    fn count_words_large_json() {
        let text = include_str!("../../../test_files/emoji-utf8.json");
        let mut words = HashMap::new();
        let start = Instant::now();
        black_box(count_words(&mut words, &Rope::from_str(text)));
        eprintln!("{:?}", start.elapsed());
    }
}
