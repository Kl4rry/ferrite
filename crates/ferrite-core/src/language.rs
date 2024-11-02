use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock, OnceLock},
};

use tree_sitter::Language;

use self::syntax::HighlightConfiguration;

pub mod detect;
pub mod syntax;

#[derive(Clone)]
pub struct TreeSitterConfig {
    pub name: String,
    pub highlight_config: Arc<HighlightConfiguration>,
}

impl TreeSitterConfig {
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

static LANGUAGES: LazyLock<HashMap<&'static str, OnceLock<TreeSitterConfig>>> =
    LazyLock::new(|| {
        let mut langs = HashMap::new();
        #[cfg(feature = "lang-rust")]
        langs.insert("rust", OnceLock::new());
        #[cfg(feature = "lang-json")]
        langs.insert("json", OnceLock::new());
        #[cfg(feature = "lang-c")]
        langs.insert("c", OnceLock::new());
        #[cfg(feature = "lang-cpp")]
        langs.insert("cpp", OnceLock::new());
        #[cfg(feature = "lang-cmake")]
        langs.insert("cmake", OnceLock::new());
        #[cfg(feature = "lang-css")]
        langs.insert("css", OnceLock::new());
        #[cfg(feature = "lang-glsl")]
        langs.insert("glsl", OnceLock::new());
        #[cfg(feature = "lang-html")]
        langs.insert("html", OnceLock::new());
        #[cfg(feature = "lang-md")]
        langs.insert("markdown", OnceLock::new());
        #[cfg(feature = "lang-python")]
        langs.insert("python", OnceLock::new());
        #[cfg(feature = "lang-toml")]
        langs.insert("toml", OnceLock::new());
        #[cfg(feature = "lang-xml")]
        langs.insert("xml", OnceLock::new());
        #[cfg(feature = "lang-yaml")]
        langs.insert("yaml", OnceLock::new());
        #[cfg(feature = "lang-c-sharp")]
        langs.insert("c-sharp", OnceLock::new());
        #[cfg(feature = "lang-fish")]
        langs.insert("fish", OnceLock::new());
        #[cfg(feature = "lang-comment")]
        langs.insert("comment", OnceLock::new());
        #[cfg(feature = "lang-javascript")]
        langs.insert("javascript", OnceLock::new());
        #[cfg(feature = "lang-bash")]
        langs.insert("bash", OnceLock::new());
        #[cfg(feature = "lang-ron")]
        langs.insert("ron", OnceLock::new());
        #[cfg(feature = "lang-fortran")]
        langs.insert("fortran", OnceLock::new());
        #[cfg(feature = "lang-zig")]
        langs.insert("zig", OnceLock::new());
        #[cfg(feature = "lang-hyprlang")]
        langs.insert("hyprlang", OnceLock::new());
        #[cfg(feature = "lang-go")]
        langs.insert("go", OnceLock::new());
        #[cfg(feature = "lang-typescript")]
        langs.insert("typescript", OnceLock::new());
        #[cfg(feature = "lang-ini")]
        langs.insert("ini", OnceLock::new());
        #[cfg(feature = "lang-diff")]
        langs.insert("diff", OnceLock::new());
        #[cfg(feature = "lang-git-config")]
        langs.insert("git-config", OnceLock::new());
        #[cfg(feature = "lang-git-commit")]
        langs.insert("git-commit", OnceLock::new());
        #[cfg(feature = "lang-rebase")]
        langs.insert("git-rebase", OnceLock::new());
        #[cfg(feature = "lang-dockerfile")]
        langs.insert("dockerfile", OnceLock::new());
        #[cfg(feature = "lang-protobuf")]
        langs.insert("protobuf", OnceLock::new());
        #[cfg(feature = "lang-lua")]
        langs.insert("lua", OnceLock::new());
        #[cfg(feature = "lang-nu")]
        langs.insert("nu", OnceLock::new());
        langs
    });

fn get_lang_config(name: &str) -> Option<TreeSitterConfig> {
    tracing::info!("Loading tree-sitter syntax for: `{name}`");
    Some(match name {
        #[cfg(feature = "lang-rust")]
        "rust" => TreeSitterConfig::new(
            "rust",
            ferrite_tree_sitter::tree_sitter_rust::language(),
            include_str!("../../../queries/rust/highlights.scm"),
            include_str!("../../../queries/rust/injections.scm"),
            include_str!("../../../queries/rust/locals.scm"),
        ),
        #[cfg(feature = "lang-json")]
        "json" => TreeSitterConfig::new(
            "json",
            ferrite_tree_sitter::tree_sitter_json::language(),
            include_str!("../../../queries/json/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-c")]
        "c" => TreeSitterConfig::new(
            "c",
            ferrite_tree_sitter::tree_sitter_c::language(),
            include_str!("../../../queries/c/highlights.scm"),
            include_str!("../../../queries/c/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cpp")]
        "cpp" => TreeSitterConfig::new(
            "cpp",
            ferrite_tree_sitter::tree_sitter_cpp::language(),
            include_str!("../../../queries/cpp/highlights.scm"),
            include_str!("../../../queries/cpp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cmake")]
        "cmake" => TreeSitterConfig::new(
            "cmake",
            ferrite_tree_sitter::tree_sitter_cmake::language(),
            include_str!("../../../queries/cmake/highlights.scm"),
            include_str!("../../../queries/cmake/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-css")]
        "css" => TreeSitterConfig::new(
            "css",
            ferrite_tree_sitter::tree_sitter_css::language(),
            include_str!("../../../queries/css/highlights.scm"),
            include_str!("../../../queries/css/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-glsl")]
        "glsl" => TreeSitterConfig::new(
            "glsl",
            ferrite_tree_sitter::tree_sitter_glsl::language(),
            include_str!("../../../queries/glsl/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-html")]
        "html" => TreeSitterConfig::new(
            "html",
            ferrite_tree_sitter::tree_sitter_html::language(),
            include_str!("../../../queries/html/highlights.scm"),
            include_str!("../../../queries/html/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-md")]
        "markdown" => TreeSitterConfig::new(
            "markdown",
            ferrite_tree_sitter::tree_sitter_md::language(),
            include_str!("../../../queries/markdown/highlights.scm"),
            include_str!("../../../queries/markdown/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-python")]
        "python" => TreeSitterConfig::new(
            "python",
            ferrite_tree_sitter::tree_sitter_python::language(),
            include_str!("../../../queries/python/highlights.scm"),
            include_str!("../../../queries/python/injections.scm"),
            include_str!("../../../queries/python/locals.scm"),
        ),
        #[cfg(feature = "lang-toml")]
        "toml" => TreeSitterConfig::new(
            "toml",
            ferrite_tree_sitter::tree_sitter_toml::language(),
            include_str!("../../../queries/toml/highlights.scm"),
            include_str!("../../../queries/toml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-xml")]
        "xml" => TreeSitterConfig::new(
            "xml",
            ferrite_tree_sitter::tree_sitter_xml::language(),
            include_str!("../../../queries/xml/highlights.scm"),
            include_str!("../../../queries/xml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-yaml")]
        "yaml" => TreeSitterConfig::new(
            "yaml",
            ferrite_tree_sitter::tree_sitter_yaml::language(),
            include_str!("../../../queries/yaml/highlights.scm"),
            include_str!("../../../queries/yaml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-c-sharp")]
        "c-sharp" => TreeSitterConfig::new(
            "c-sharp",
            ferrite_tree_sitter::tree_sitter_c_sharp::language(),
            include_str!("../../../queries/c-sharp/highlights.scm"),
            include_str!("../../../queries/c-sharp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-bash")]
        "bash" => TreeSitterConfig::new(
            "bash",
            ferrite_tree_sitter::tree_sitter_bash::language(),
            include_str!("../../../queries/bash/highlights.scm"),
            include_str!("../../../queries/bash/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-fish")]
        "fish" => TreeSitterConfig::new(
            "fish",
            ferrite_tree_sitter::tree_sitter_fish::language(),
            include_str!("../../../queries/fish/highlights.scm"),
            include_str!("../../../queries/fish/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-comment")]
        "comment" => TreeSitterConfig::new(
            "comment",
            ferrite_tree_sitter::tree_sitter_comment::language(),
            include_str!("../../../queries/comment/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-javascript")]
        "javascript" => TreeSitterConfig::new(
            "javascript",
            ferrite_tree_sitter::tree_sitter_javascript::language(),
            include_str!("../../../queries/javascript/highlights.scm"),
            include_str!("../../../queries/javascript/injections.scm"),
            include_str!("../../../queries/javascript/locals.scm"),
        ),
        #[cfg(feature = "lang-ron")]
        "ron" => TreeSitterConfig::new(
            "ron",
            ferrite_tree_sitter::tree_sitter_ron::language(),
            include_str!("../../../queries/ron/highlights.scm"),
            include_str!("../../../queries/ron/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-fortran")]
        "fortran" => TreeSitterConfig::new(
            "fortran",
            ferrite_tree_sitter::tree_sitter_fortran::language(),
            include_str!("../../../queries/fortran/highlights.scm"),
            include_str!("../../../queries/fortran/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-zig")]
        "zig" => TreeSitterConfig::new(
            "zig",
            ferrite_tree_sitter::tree_sitter_zig::language(),
            include_str!("../../../queries/zig/highlights.scm"),
            include_str!("../../../queries/zig/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-hyprlang")]
        "hyprlang" => TreeSitterConfig::new(
            "hyprlang",
            ferrite_tree_sitter::tree_sitter_hyprlang::language(),
            include_str!("../../../queries/hyprlang/highlights.scm"),
            include_str!("../../../queries/hyprlang/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-go")]
        "go" => TreeSitterConfig::new(
            "go",
            ferrite_tree_sitter::tree_sitter_go::language(),
            include_str!("../../../queries/go/highlights.scm"),
            include_str!("../../../queries/go/injections.scm"),
            include_str!("../../../queries/go/locals.scm"),
        ),
        #[cfg(feature = "lang-typescript")]
        "typescript" => TreeSitterConfig::new(
            "typescript",
            ferrite_tree_sitter::tree_sitter_typescript::language_typescript(),
            include_str!("../../../queries/typescript/highlights.scm"),
            include_str!("../../../queries/typescript/injections.scm"),
            include_str!("../../../queries/typescript/locals.scm"),
        ),
        #[cfg(feature = "lang-ini")]
        "ini" => TreeSitterConfig::new(
            "ini",
            ferrite_tree_sitter::tree_sitter_ini::language(),
            include_str!("../../../queries/ini/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-diff")]
        "diff" => TreeSitterConfig::new(
            "diff",
            ferrite_tree_sitter::tree_sitter_diff::language(),
            include_str!("../../../queries/diff/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-git-config")]
        "git-config" => TreeSitterConfig::new(
            "git-config",
            ferrite_tree_sitter::tree_sitter_git_config::language(),
            include_str!("../../../queries/git-config/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-git-commit")]
        "git-commit" => TreeSitterConfig::new(
            "git-commit",
            ferrite_tree_sitter::tree_sitter_gitcommit::language(),
            include_str!("../../../queries/git-commit/highlights.scm"),
            include_str!("../../../queries/git-commit/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-rebase")]
        "git-rebase" => TreeSitterConfig::new(
            "git-rebase",
            ferrite_tree_sitter::tree_sitter_rebase::language(),
            include_str!("../../../queries/git-rebase/highlights.scm"),
            include_str!("../../../queries/git-rebase/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-dockerfile")]
        "dockerfile" => TreeSitterConfig::new(
            "dockerfile",
            ferrite_tree_sitter::tree_sitter_dockerfile::language(),
            include_str!("../../../queries/dockerfile/highlights.scm"),
            include_str!("../../../queries/dockerfile/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-protobuf")]
        "protobuf" => TreeSitterConfig::new(
            "protobuf",
            ferrite_tree_sitter::tree_sitter_protobuf::language(),
            include_str!("../../../queries/protobuf/highlights.scm"),
            include_str!("../../../queries/protobuf/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-lua")]
        "lua" => TreeSitterConfig::new(
            "lua",
            ferrite_tree_sitter::tree_sitter_lua::language(),
            include_str!("../../../queries/lua/highlights.scm"),
            include_str!("../../../queries/lua/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-nu")]
        "nu" => TreeSitterConfig::new(
            "nu",
            ferrite_tree_sitter::tree_sitter_nu::language(),
            include_str!("../../../queries/nu/highlights.scm"),
            include_str!("../../../queries/nu/injections.scm"),
            "",
        ),
        _ => return None,
    })
}

pub enum Pattern {
    Suffix(&'static str),
    Name(&'static str),
}

impl Pattern {
    pub fn matches(&self, file: &str) -> bool {
        match self {
            Pattern::Suffix(suffix) => file.ends_with(suffix),
            Pattern::Name(name) => name.to_lowercase() == file.to_lowercase(),
        }
    }
}

pub fn get_language_from_path(path: impl AsRef<Path>) -> Option<&'static str> {
    use Pattern::*;
    static LANGUAGES: &[(Pattern, &str)] = &[
        (Suffix(".rs"), "rust"),
        (Suffix(".json"), "json"),
        (Suffix(".c"), "c"),
        (Suffix(".h"), "c"),
        (Suffix(".css"), "css"),
        (Suffix(".md"), "markdown"),
        (Suffix(".py"), "python"),
        (Suffix(".xml"), "xml"),
        (Suffix(".yaml"), "yaml"),
        (Suffix(".yml"), "yaml"),
        (Suffix(".cs"), "c-sharp"),
        (Suffix(".fish"), "fish"),
        (Suffix(".js"), "javascript"),
        (Suffix(".ron"), "ron"),
        (Suffix(".f"), "fortran"),
        (Suffix(".zig"), "zig"),
        (Suffix(".go"), "go"),
        (Suffix(".ts"), "ts"),
        (Suffix(".proto"), "protobuf"),
        (Suffix(".lua"), "lua"),
        (Suffix(".nu"), "nu"),
        (Name("hyprland.conf"), "hyprlang"),
        (Name("COMMIT_EDITMSG"), "git-commit"),
        (Name("git-rebase-todo"), "git-rebase"),
        // cmake
        (Name("CMakeLists.txt"), "cmake"),
        (Suffix(".cmake"), "cmake"),
        // toml
        (Suffix(".toml"), "toml"),
        (Name("Cargo.lock"), "toml"),
        // dockerfile
        (Name("Dockerfile"), "dockerfile"),
        (Name("Containerfile"), "dockerfile"),
        // glsl
        (Suffix(".glsl"), "glsl"),
        (Suffix(".vert"), "glsl"),
        (Suffix(".frag"), "glsl"),
        // html
        (Suffix(".html"), "html"),
        (Suffix(".html"), "html"),
        (Suffix(".htm"), "html"),
        (Suffix(".xhtml"), "html"),
        (Suffix(".shtml"), "html"),
        // c++
        (Suffix(".cpp"), "cpp"),
        (Suffix(".cc"), "cpp"),
        (Suffix(".cp"), "cpp"),
        (Suffix(".cxx"), "cpp"),
        (Suffix(".c++"), "cpp"),
        (Suffix(".C"), "cpp"),
        (Suffix(".h"), "cpp"),
        (Suffix(".hh"), "cpp"),
        (Suffix(".hpp"), "cpp"),
        (Suffix(".hxx"), "cpp"),
        (Suffix(".h++"), "cpp"),
        (Suffix(".inl"), "cpp"),
        (Suffix(".ipp"), "cpp"),
        (Suffix(".cx"), "cpp"),
        (Suffix(".tcc"), "cpp"),
        // shell
        (Suffix(".sh"), "bash"),
        (Suffix(".bash"), "bash"),
        (Suffix(".zsh"), "bash"),
        (Name(".bash_login"), "bash"),
        (Name(".bash_logout"), "bash"),
        (Name(".bash_profile"), "bash"),
        (Name(".bashrc"), "bash"),
        (Name(".profile"), "bash"),
        (Name(".zshenv"), "bash"),
        (Name(".zlogin"), "bash"),
        (Name(".zlogout"), "bash"),
        (Name(".zprofile"), "bash"),
        (Name(".zshrc"), "bash"),
        (Name("PKGBUILD"), "bash"),
        // ini
        (Suffix(".ini"), "ini"),
        (Suffix(".service"), "ini"),
        (Suffix(".automount"), "ini"),
        (Suffix(".device"), "ini"),
        (Suffix(".mount"), "ini"),
        (Suffix(".path"), "ini"),
        (Suffix(".service"), "ini"),
        (Suffix(".slice"), "ini"),
        (Suffix(".socket"), "ini"),
        (Suffix(".swap"), "ini"),
        (Suffix(".target"), "ini"),
        (Suffix(".timer"), "ini"),
        (Suffix(".container"), "ini"),
        (Suffix(".volume"), "ini"),
        (Suffix(".kube"), "ini"),
        (Suffix(".network"), "ini"),
        (Suffix(".properties"), "ini"),
        (Suffix(".cfg"), "ini"),
        (Suffix(".directory"), "ini"),
        (Name(".editorconfig"), "ini"),
        (Name("rclone.conf"), "ini"),
    ];

    let path = path.as_ref().file_name()?.to_string_lossy();
    LANGUAGES
        .iter()
        .find(|(suffix, _)| suffix.matches(&path))
        .map(|(_, lang)| lang)
        .copied()
}

pub fn get_tree_sitter_language(language: &str) -> Option<&'static TreeSitterConfig> {
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
    fn language_load() {
        for k in LANGUAGES.keys() {
            println!("{k}");
            assert!(get_lang_config(*k).is_some())
        }
    }
}
