use std::{collections::HashMap, path::Path, sync::Arc};

use once_cell::sync::Lazy;
use tree_sitter::Language;

use self::syntax::HighlightConfiguration;

pub mod syntax;

#[derive(Clone)]
pub struct LanguageConfig {
    pub name: String,
    pub highlight_config: Arc<HighlightConfiguration>,
}

impl LanguageConfig {
    pub fn new(
        name: impl Into<String>,
        grammar: Language,
        highlight_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Self {
        Self {
            name: name.into(),
            highlight_config: Arc::new(
                HighlightConfiguration::new(
                    grammar,
                    highlight_query,
                    injection_query,
                    locals_query,
                )
                .unwrap(),
            ),
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
            include_str!("../../queries/rust/highlights.scm"),
            include_str!("../../queries/rust/injections.scm"),
            include_str!("../../queries/rust/locals.scm"),
        ),
    );
    langs.insert(
        "json",
        LanguageConfig::new(
            "json",
            ferrite_tree_sitter::tree_sitter_json::language(),
            include_str!("../../queries/json/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "c",
        LanguageConfig::new(
            "c",
            ferrite_tree_sitter::tree_sitter_c::language(),
            include_str!("../../queries/c/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "cpp",
        LanguageConfig::new(
            "cpp",
            ferrite_tree_sitter::tree_sitter_cpp::language(),
            include_str!("../../queries/cpp/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "cmake",
        LanguageConfig::new(
            "cmake",
            ferrite_tree_sitter::tree_sitter_cmake::language(),
            include_str!("../../queries/cmake/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "css",
        LanguageConfig::new(
            "css",
            ferrite_tree_sitter::tree_sitter_css::language(),
            include_str!("../../queries/css/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "glsl",
        LanguageConfig::new(
            "glsl",
            ferrite_tree_sitter::tree_sitter_glsl::language(),
            include_str!("../../queries/glsl/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "html",
        LanguageConfig::new(
            "html",
            ferrite_tree_sitter::tree_sitter_html::language(),
            include_str!("../../queries/html/highlights.scm"),
            "",
            "",
        ),
    );
    /*langs.insert(
        "markdown",
        LanguageConfig::new(
            "markdown",
            ferrite_tree_sitter::tree_sitter_md::language(),
            include_str!("../../queries/markdown/highlights.scm"),
            "",
            "",
        ),
    );*/
    langs.insert(
        "python",
        LanguageConfig::new(
            "python",
            ferrite_tree_sitter::tree_sitter_python::language(),
            include_str!("../../queries/python/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "toml",
        LanguageConfig::new(
            "toml",
            ferrite_tree_sitter::tree_sitter_toml::language(),
            include_str!("../../queries/toml/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "xml",
        LanguageConfig::new(
            "xml",
            ferrite_tree_sitter::tree_sitter_xml::language(),
            include_str!("../../queries/xml/highlights.scm"),
            "",
            "",
        ),
    );
    langs.insert(
        "yaml",
        LanguageConfig::new(
            "yaml",
            ferrite_tree_sitter::tree_sitter_yaml::language(),
            include_str!("../../queries/yaml/highlights.scm"),
            "",
            "",
        ),
    );
    langs
});

// TODO make this functions use more then extension
pub fn get_language_from_path(path: impl AsRef<Path>) -> Option<&'static str> {
    static LANGUAGES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        let mut langs = HashMap::new();
        langs.insert("rs", "rust");
        langs.insert("json", "json");
        langs.insert("c", "c");
        langs.insert("h", "c");
        langs.insert("cpp", "cpp");
        langs.insert("cc", "cpp");
        langs.insert("hpp", "cpp");
        langs.insert("cx", "cpp");
        langs.insert("tcc", "cpp");
        langs.insert("css", "css");
        langs.insert("glsl", "glsl");
        langs.insert("vert", "glsl");
        langs.insert("frag", "glsl");
        langs.insert("html", "html");
        //langs.insert("md", "markdown");
        langs.insert("py", "python");
        langs.insert("toml", "toml");
        langs.insert("xml", "xml");
        langs.insert("yaml", "yaml");
        langs.insert("yml", "yaml");
        langs
    });

    let ext = path.as_ref().extension()?.to_string_lossy();
    LANGUAGES.get(ext.as_ref()).copied()
}

pub fn get_tree_sitter_language(language: &str) -> Option<LanguageConfig> {
    LANGUAGES.get(&language).cloned()
}
