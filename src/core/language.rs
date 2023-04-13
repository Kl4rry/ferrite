use std::{collections::HashMap, path::Path, sync::Arc};

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
    langs.insert(
        "json",
        LanguageConfig::new(
            "json",
            ferrite_tree_sitter::tree_sitter_json::language(),
            ferrite_tree_sitter::tree_sitter_json::HIGHLIGHT_QUERY,
        ),
    );
    langs
});

pub fn get_language_from_path(path: impl AsRef<Path>) -> Option<&'static str> {
    static LANGUAGES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        let mut langs = HashMap::new();
        langs.insert("rs", "rust");
        langs.insert("json", "json");
        langs
    });

    let ext = path.as_ref().extension()?.to_string_lossy();
    LANGUAGES.get(ext.as_ref()).copied()
}

pub fn get_tree_sitter_language(language: &str) -> Option<LanguageConfig> {
    LANGUAGES.get(&language).cloned()
}
