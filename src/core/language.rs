use std::{collections::HashMap, sync::Arc};

use once_cell::sync::Lazy;
use tree_sitter::{Language, Query};

pub mod syntax;

#[derive(Debug, Clone)]
pub struct LanguageConfig {
    pub name: String,
    pub grammar: Language,
    pub highlight_query: Arc<Query>,
}

impl LanguageConfig {
    pub fn new(
        name: impl Into<String>,
        grammar: Language,
        highlight_query: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            highlight_query: Arc::new(Query::new(grammar, &highlight_query.into()).unwrap()),
            grammar,
        }
    }
}

static LANGUAGES: Lazy<HashMap<&'static str, LanguageConfig>> = Lazy::new(|| {
    let mut langs = HashMap::new();
    langs.insert(
        "rust",
        LanguageConfig::new(
            "rust",
            ferrite_tree_sitter::tree_sitter_rust::language(),
            ferrite_tree_sitter::tree_sitter_rust::HIGHLIGHT_QUERY,
        ),
    );
    langs
});

pub fn get_tree_sitter_language(language: &str) -> Option<LanguageConfig> {
    LANGUAGES.get(&language).cloned()
}
