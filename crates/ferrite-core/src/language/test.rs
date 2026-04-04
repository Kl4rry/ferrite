#[cfg(test)]
mod tests {
    use crate::{
        language::{LANGUAGES, get_lang_config},
        theme::EditorTheme,
    };
    #[test]
    fn language_load() {
        for (name, _, _) in LANGUAGES.iter() {
            println!("{name}");
            let config = get_lang_config(*name);
            assert!(config.is_some());
            let theme = EditorTheme::default();
            for name in config.unwrap().capture_names() {
                println!("{name}");
                assert!(theme.try_get_syntax(name).is_some());
            }
        }
    }
}
