use std::path::Path;

pub fn trim_path(start: &str, path: &Path) -> String {
    let path_str = path.to_string_lossy();
    let trimmed = path_str.trim_start_matches(start);
    if trimmed.len() < path_str.len() {
        trimmed
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .to_string()
    } else {
        trimmed.to_string()
    }
}
