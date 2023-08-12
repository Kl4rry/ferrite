use std::{collections::HashMap, path::Path, sync::Arc};

use once_cell::sync::{Lazy, OnceCell};
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

static LANGUAGES: Lazy<HashMap<&'static str, OnceCell<LanguageConfig>>> = Lazy::new(|| {
    let mut langs = HashMap::new();
    #[cfg(feature = "lang-rust")]
    langs.insert("rust", OnceCell::new());
    #[cfg(feature = "lang-json")]
    langs.insert("json", OnceCell::new());
    #[cfg(feature = "lang-c")]
    langs.insert("c", OnceCell::new());
    #[cfg(feature = "lang-cpp")]
    langs.insert("cpp", OnceCell::new());
    #[cfg(feature = "lang-cmake")]
    langs.insert("cmake", OnceCell::new());
    #[cfg(feature = "lang-css")]
    langs.insert("css", OnceCell::new());
    #[cfg(feature = "lang-glsl")]
    langs.insert("glsl", OnceCell::new());
    #[cfg(feature = "lang-html")]
    langs.insert("html", OnceCell::new());
    #[cfg(feature = "lang-md")]
    langs.insert("markdown", OnceCell::new());
    #[cfg(feature = "lang-python")]
    langs.insert("python", OnceCell::new());
    #[cfg(feature = "lang-toml")]
    langs.insert("toml", OnceCell::new());
    #[cfg(feature = "lang-xml")]
    langs.insert("xml", OnceCell::new());
    #[cfg(feature = "lang-yaml")]
    langs.insert("yaml", OnceCell::new());
    #[cfg(feature = "lang-c-sharp")]
    langs.insert("c-sharp", OnceCell::new());
    #[cfg(feature = "lang-fish")]
    langs.insert("fish", OnceCell::new());
    #[cfg(feature = "lang-comment")]
    langs.insert("comment", OnceCell::new());
    #[cfg(feature = "lang-javascript")]
    langs.insert("javascript", OnceCell::new());
    #[cfg(feature = "lang-bash")]
    langs.insert("bash", OnceCell::new());
    langs
});

fn get_lang_config(name: &str) -> Option<LanguageConfig> {
    log::info!("Loading tree-sitter syntax for: `{name}`");
    Some(match name {
        #[cfg(feature = "lang-rust")]
        "rust" => LanguageConfig::new(
            "rust",
            ferrite_tree_sitter::tree_sitter_rust::language(),
            include_str!("../../queries/rust/highlights.scm"),
            include_str!("../../queries/rust/injections.scm"),
            include_str!("../../queries/rust/locals.scm"),
        ),
        #[cfg(feature = "lang-json")]
        "json" => LanguageConfig::new(
            "json",
            ferrite_tree_sitter::tree_sitter_json::language(),
            include_str!("../../queries/json/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-c")]
        "c" => LanguageConfig::new(
            "c",
            ferrite_tree_sitter::tree_sitter_c::language(),
            include_str!("../../queries/c/highlights.scm"),
            include_str!("../../queries/c/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cpp")]
        "cpp" => LanguageConfig::new(
            "cpp",
            ferrite_tree_sitter::tree_sitter_cpp::language(),
            include_str!("../../queries/cpp/highlights.scm"),
            include_str!("../../queries/cpp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cmake")]
        "cmake" => LanguageConfig::new(
            "cmake",
            ferrite_tree_sitter::tree_sitter_cmake::language(),
            include_str!("../../queries/cmake/highlights.scm"),
            include_str!("../../queries/cmake/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-css")]
        "css" => LanguageConfig::new(
            "css",
            ferrite_tree_sitter::tree_sitter_css::language(),
            include_str!("../../queries/css/highlights.scm"),
            include_str!("../../queries/css/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-glsl")]
        "glsl" => LanguageConfig::new(
            "glsl",
            ferrite_tree_sitter::tree_sitter_glsl::language(),
            include_str!("../../queries/glsl/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-html")]
        "html" => LanguageConfig::new(
            "html",
            ferrite_tree_sitter::tree_sitter_html::language(),
            include_str!("../../queries/html/highlights.scm"),
            include_str!("../../queries/html/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-md")]
        "markdown" => LanguageConfig::new(
            "markdown",
            ferrite_tree_sitter::tree_sitter_md::language(),
            include_str!("../../queries/markdown/highlights.scm"),
            include_str!("../../queries/markdown/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-python")]
        "python" => LanguageConfig::new(
            "python",
            ferrite_tree_sitter::tree_sitter_python::language(),
            include_str!("../../queries/python/highlights.scm"),
            include_str!("../../queries/python/injections.scm"),
            include_str!("../../queries/python/locals.scm"),
        ),
        #[cfg(feature = "lang-toml")]
        "toml" => LanguageConfig::new(
            "toml",
            ferrite_tree_sitter::tree_sitter_toml::language(),
            include_str!("../../queries/toml/highlights.scm"),
            include_str!("../../queries/toml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-xml")]
        "xml" => LanguageConfig::new(
            "xml",
            ferrite_tree_sitter::tree_sitter_xml::language(),
            include_str!("../../queries/xml/highlights.scm"),
            include_str!("../../queries/xml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-yaml")]
        "yaml" => LanguageConfig::new(
            "yaml",
            ferrite_tree_sitter::tree_sitter_yaml::language(),
            include_str!("../../queries/yaml/highlights.scm"),
            include_str!("../../queries/yaml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-c-sharp")]
        "c-sharp" => LanguageConfig::new(
            "c-sharp",
            ferrite_tree_sitter::tree_sitter_c_sharp::language(),
            include_str!("../../queries/c-sharp/highlights.scm"),
            include_str!("../../queries/c-sharp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-bash")]
        "bash" => LanguageConfig::new(
            "bash",
            ferrite_tree_sitter::tree_sitter_bash::language(),
            include_str!("../../queries/bash/highlights.scm"),
            include_str!("../../queries/bash/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-fish")]
        "fish" => LanguageConfig::new(
            "fish",
            ferrite_tree_sitter::tree_sitter_fish::language(),
            include_str!("../../queries/fish/highlights.scm"),
            include_str!("../../queries/fish/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-comment")]
        "comment" => LanguageConfig::new(
            "comment",
            ferrite_tree_sitter::tree_sitter_comment::language(),
            include_str!("../../queries/comment/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-javascript")]
        "javascript" => LanguageConfig::new(
            "javascript",
            ferrite_tree_sitter::tree_sitter_javascript::language(),
            include_str!("../../queries/javascript/highlights.scm"),
            include_str!("../../queries/javascript/injections.scm"),
            include_str!("../../queries/javascript/locals.scm"),
        ),
        _ => return None,
    })
}

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
        langs.insert("md", "markdown");
        langs.insert("py", "python");
        langs.insert("toml", "toml");
        langs.insert("xml", "xml");
        langs.insert("yaml", "yaml");
        langs.insert("yml", "yaml");
        langs.insert("cs", "c-sharp");
        langs.insert("sh", "bash");
        langs.insert("bash", "bash");
        langs.insert("bashrc", "bash");
        langs.insert("fish", "fish");
        langs.insert("js", "javascript");
        langs
    });

    let ext = path.as_ref().extension()?.to_string_lossy();
    LANGUAGES.get(ext.as_ref()).copied()
}

pub fn get_tree_sitter_language(language: &str) -> Option<&'static LanguageConfig> {
    LANGUAGES
        .get(language)
        .map(|cell| cell.get_or_init(|| get_lang_config(language).unwrap()))
}

pub fn get_available_languages() -> Vec<&'static str> {
    LANGUAGES.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_load_test() {
        for k in LANGUAGES.keys() {
            println!("{k}");
            assert!(get_lang_config(*k).is_some())
        }
    }
}
