use std::path::Path;

pub fn trim_path(start: &str, path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let without_start = path_str.trim_start_matches(start);
    if without_start < start {
        without_start
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .to_string()
    } else {
        without_start.to_string()
    }
}
