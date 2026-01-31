use std::{
    path::Path,
    sync::{
        Arc, LazyLock, OnceLock,
        atomic::{AtomicU32, Ordering},
    },
};

use tree_house::highlighter::Highlight;
use tree_house_bindings::Grammar;
use tree_sitter_language::LanguageFn;

pub mod detect;
pub mod syntax;

pub struct TreeSitterConfig {
    pub name: String,
    pub language_config: Arc<tree_house::LanguageConfig>,
    pub capture_names: Vec<String>,
}

impl TreeSitterConfig {
    pub fn new(
        name: impl Into<String>,
        grammar: LanguageFn,
        highlight_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Self {
        let config = tree_house::LanguageConfig::new(
            Grammar::try_from(grammar).unwrap(),
            highlight_query,
            injection_query,
            locals_query,
        )
        .unwrap();
        let mut capture_names = Vec::new();
        let mut i = 0;
        config.configure(|s| {
            capture_names.push(s.to_string());
            let idx = i;
            i += 1;
            Some(Highlight::new(idx))
        });
        Self {
            name: name.into(),
            language_config: Arc::new(config),
            capture_names,
        }
    }

    pub fn capture_names(&self) -> &[String] {
        &self.capture_names
    }
}

fn get_id() -> u32 {
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

static LANGUAGES: LazyLock<Vec<(&str, u32, OnceLock<TreeSitterConfig>)>> = LazyLock::new(|| {
    vec![
        #[cfg(feature = "lang-rust")]
        ("rust", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-bash")]
        ("bash", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-json")]
        ("json", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-c")]
        ("c", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-cpp")]
        ("cpp", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-cmake")]
        ("cmake", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-css")]
        ("css", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-glsl")]
        ("glsl", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-html")]
        ("html", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-md")]
        ("markdown", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-python")]
        ("python", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-toml")]
        ("toml", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-xml")]
        ("xml", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-yaml")]
        ("yaml", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-c-sharp")]
        ("c-sharp", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-fish")]
        ("fish", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-comment")]
        ("comment", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-javascript")]
        ("javascript", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-ron")]
        ("ron", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-fortran")]
        ("fortran", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-zig")]
        ("zig", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-hyprlang")]
        ("hyprlang", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-go")]
        ("go", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-typescript")]
        ("typescript", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-ini")]
        ("ini", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-diff")]
        ("diff", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-git-config")]
        (git - "config", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-git-commit")]
        (git - "commit", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-rebase")]
        (git - "rebase", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-dockerfile")]
        ("dockerfile", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-protobuf")]
        ("protobuf", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-lua")]
        ("lua", get_id(), OnceLock::new()),
        #[cfg(feature = "lang-nu")]
        ("nu", get_id(), OnceLock::new()),
    ]
});

fn get_lang_config(name: &str) -> Option<TreeSitterConfig> {
    tracing::info!("Loading tree-sitter syntax for: `{name}`");
    Some(match name {
        #[cfg(feature = "lang-rust")]
        "rust" => TreeSitterConfig::new(
            "rust",
            ferrite_tree_sitter::tree_sitter_rust::LANGUAGE,
            include_str!("../../../queries/rust/highlights.scm"),
            include_str!("../../../queries/rust/injections.scm"),
            include_str!("../../../queries/rust/locals.scm"),
        ),
        #[cfg(feature = "lang-bash")]
        "bash" => TreeSitterConfig::new(
            "bash",
            ferrite_tree_sitter::tree_sitter_bash::LANGUAGE,
            include_str!("../../../queries/bash/highlights.scm"),
            include_str!("../../../queries/bash/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-json")]
        "json" => TreeSitterConfig::new(
            "json",
            ferrite_tree_sitter::tree_sitter_json::LANGUAGE,
            include_str!("../../../queries/json/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-c")]
        "c" => TreeSitterConfig::new(
            "c",
            ferrite_tree_sitter::tree_sitter_c::LANGUAGE,
            include_str!("../../../queries/c/highlights.scm"),
            include_str!("../../../queries/c/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cpp")]
        "cpp" => TreeSitterConfig::new(
            "cpp",
            ferrite_tree_sitter::tree_sitter_cpp::LANGUAGE,
            include_str!("../../../queries/cpp/highlights.scm"),
            include_str!("../../../queries/cpp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-cmake")]
        "cmake" => TreeSitterConfig::new(
            "cmake",
            ferrite_tree_sitter::tree_sitter_cmake::LANGUAGE,
            include_str!("../../../queries/cmake/highlights.scm"),
            include_str!("../../../queries/cmake/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-css")]
        "css" => TreeSitterConfig::new(
            "css",
            ferrite_tree_sitter::tree_sitter_css::LANGUAGE,
            include_str!("../../../queries/css/highlights.scm"),
            include_str!("../../../queries/css/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-glsl")]
        "glsl" => TreeSitterConfig::new(
            "glsl",
            ferrite_tree_sitter::tree_sitter_glsl::LANGUAGE,
            include_str!("../../../queries/glsl/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-html")]
        "html" => TreeSitterConfig::new(
            "html",
            ferrite_tree_sitter::tree_sitter_html::LANGUAGE,
            include_str!("../../../queries/html/highlights.scm"),
            include_str!("../../../queries/html/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-md")]
        "markdown" => TreeSitterConfig::new(
            "markdown",
            ferrite_tree_sitter::tree_sitter_md::LANGUAGE,
            include_str!("../../../queries/markdown/highlights.scm"),
            include_str!("../../../queries/markdown/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-python")]
        "python" => TreeSitterConfig::new(
            "python",
            ferrite_tree_sitter::tree_sitter_python::LANGUAGE,
            include_str!("../../../queries/python/highlights.scm"),
            include_str!("../../../queries/python/injections.scm"),
            include_str!("../../../queries/python/locals.scm"),
        ),
        #[cfg(feature = "lang-toml")]
        "toml" => TreeSitterConfig::new(
            "toml",
            ferrite_tree_sitter::tree_sitter_toml::LANGUAGE,
            include_str!("../../../queries/toml/highlights.scm"),
            include_str!("../../../queries/toml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-xml")]
        "xml" => TreeSitterConfig::new(
            "xml",
            ferrite_tree_sitter::tree_sitter_xml::LANGUAGE,
            include_str!("../../../queries/xml/highlights.scm"),
            include_str!("../../../queries/xml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-yaml")]
        "yaml" => TreeSitterConfig::new(
            "yaml",
            ferrite_tree_sitter::tree_sitter_yaml::LANGUAGE,
            include_str!("../../../queries/yaml/highlights.scm"),
            include_str!("../../../queries/yaml/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-c-sharp")]
        "c-sharp" => TreeSitterConfig::new(
            "c-sharp",
            ferrite_tree_sitter::tree_sitter_c_sharp::LANGUAGE,
            include_str!("../../../queries/c-sharp/highlights.scm"),
            include_str!("../../../queries/c-sharp/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-fish")]
        "fish" => TreeSitterConfig::new(
            "fish",
            ferrite_tree_sitter::tree_sitter_fish::LANGUAGE,
            include_str!("../../../queries/fish/highlights.scm"),
            include_str!("../../../queries/fish/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-comment")]
        "comment" => TreeSitterConfig::new(
            "comment",
            ferrite_tree_sitter::tree_sitter_comment::LANGUAGE,
            include_str!("../../../queries/comment/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-javascript")]
        "javascript" => TreeSitterConfig::new(
            "javascript",
            ferrite_tree_sitter::tree_sitter_javascript::LANGUAGE,
            include_str!("../../../queries/javascript/highlights.scm"),
            include_str!("../../../queries/javascript/injections.scm"),
            include_str!("../../../queries/javascript/locals.scm"),
        ),
        #[cfg(feature = "lang-ron")]
        "ron" => TreeSitterConfig::new(
            "ron",
            ferrite_tree_sitter::tree_sitter_ron::LANGUAGE,
            include_str!("../../../queries/ron/highlights.scm"),
            include_str!("../../../queries/ron/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-fortran")]
        "fortran" => TreeSitterConfig::new(
            "fortran",
            ferrite_tree_sitter::tree_sitter_fortran::LANGUAGE,
            include_str!("../../../queries/fortran/highlights.scm"),
            include_str!("../../../queries/fortran/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-zig")]
        "zig" => TreeSitterConfig::new(
            "zig",
            ferrite_tree_sitter::tree_sitter_zig::LANGUAGE,
            include_str!("../../../queries/zig/highlights.scm"),
            include_str!("../../../queries/zig/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-hyprlang")]
        "hyprlang" => TreeSitterConfig::new(
            "hyprlang",
            ferrite_tree_sitter::tree_sitter_hyprlang::LANGUAGE,
            include_str!("../../../queries/hyprlang/highlights.scm"),
            include_str!("../../../queries/hyprlang/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-go")]
        "go" => TreeSitterConfig::new(
            "go",
            ferrite_tree_sitter::tree_sitter_go::LANGUAGE,
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
            ferrite_tree_sitter::tree_sitter_ini::LANGUAGE,
            include_str!("../../../queries/ini/highlights.scm"),
            "",
            "",
        ),
        #[cfg(feature = "lang-diff")]
        "diff" => TreeSitterConfig::new(
            "diff",
            ferrite_tree_sitter::tree_sitter_diff::LANGUAGE,
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
            ferrite_tree_sitter::tree_sitter_gitcommit::LANGUAGE,
            include_str!("../../../queries/git-commit/highlights.scm"),
            include_str!("../../../queries/git-commit/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-rebase")]
        "git-rebase" => TreeSitterConfig::new(
            "git-rebase",
            ferrite_tree_sitter::tree_sitter_rebase::LANGUAGE,
            include_str!("../../../queries/git-rebase/highlights.scm"),
            include_str!("../../../queries/git-rebase/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-dockerfile")]
        "dockerfile" => TreeSitterConfig::new(
            "dockerfile",
            ferrite_tree_sitter::tree_sitter_dockerfile::LANGUAGE,
            include_str!("../../../queries/dockerfile/highlights.scm"),
            include_str!("../../../queries/dockerfile/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-protobuf")]
        "protobuf" => TreeSitterConfig::new(
            "protobuf",
            ferrite_tree_sitter::tree_sitter_protobuf::LANGUAGE,
            include_str!("../../../queries/protobuf/highlights.scm"),
            include_str!("../../../queries/protobuf/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-lua")]
        "lua" => TreeSitterConfig::new(
            "lua",
            ferrite_tree_sitter::tree_sitter_lua::LANGUAGE,
            include_str!("../../../queries/lua/highlights.scm"),
            include_str!("../../../queries/lua/injections.scm"),
            "",
        ),
        #[cfg(feature = "lang-nu")]
        "nu" => TreeSitterConfig::new(
            "nu",
            ferrite_tree_sitter::tree_sitter_nu::LANGUAGE,
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
    Contains(&'static str),
}

impl Pattern {
    pub fn matches(&self, file: &str) -> bool {
        match self {
            Pattern::Suffix(suffix) => file.ends_with(suffix),
            Pattern::Name(name) => name.to_lowercase() == file.to_lowercase(),
            Pattern::Contains(name) => file.to_lowercase().contains(&name.to_lowercase()),
        }
    }
}

#[profiling::function]
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
        (Contains("Dockerfile"), "dockerfile"),
        (Contains("Containerfile"), "dockerfile"),
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

pub fn language_to_name(language: tree_house::Language) -> Option<&'static str> {
    for (name, id, _) in LANGUAGES.iter() {
        if tree_house::Language(*id) == language {
            return Some(name);
        }
    }
    None
}

pub fn name_to_language(name: &str) -> Option<(tree_house::Language, &'static TreeSitterConfig)> {
    for (language, id, config) in LANGUAGES.iter() {
        if *language == name {
            let config = config.get_or_init(|| get_lang_config(language).unwrap());
            return Some((tree_house::Language(*id), config));
        }
    }
    None
}

pub fn get_available_languages() -> Vec<&'static str> {
    LANGUAGES.iter().map(|(name, _, _)| name).copied().collect()
}

pub struct LanguageLoader;

impl tree_house::LanguageLoader for LanguageLoader {
    fn language_for_marker(
        &self,
        marker: tree_house::InjectionLanguageMarker<'_>,
    ) -> Option<tree_house::Language> {
        match marker {
            tree_house::InjectionLanguageMarker::Name(name) => {
                for (language, id, _) in LANGUAGES.iter() {
                    if *language == name {
                        return Some(tree_house::Language(*id));
                    }
                }
                None
            }
            tree_house::InjectionLanguageMarker::Match(_) => None,
            tree_house::InjectionLanguageMarker::Filename(_) => None,
            tree_house::InjectionLanguageMarker::Shebang(_) => None,
        }
    }

    fn get_config(&self, lang: tree_house::Language) -> Option<&tree_house::LanguageConfig> {
        for (language, id, cell) in LANGUAGES.iter() {
            if tree_house::Language(*id) == lang {
                return Some(
                    &*cell
                        .get_or_init(|| get_lang_config(language).unwrap())
                        .language_config,
                );
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_load() {
        for (name, _, _) in LANGUAGES.iter() {
            println!("{name}");
            assert!(get_lang_config(*name).is_some())
        }
    }
}
