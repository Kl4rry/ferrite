use std::{
    borrow::Cow,
    cmp::Ordering,
    fs::{self, Metadata},
    path::{self, Path, PathBuf},
};

use sublime_fuzzy::{FuzzySearch, Scoring};

#[cfg(any(windows, target_os = "macos"))]
fn normalize(s: &str) -> Cow<str> {
    // case insensitive
    Cow::Owned(s.to_lowercase())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn normalize(s: &str) -> Cow<str> {
    Cow::Borrowed(s)
}

pub fn complete_file_path(path: &str, executable_only: bool) -> Vec<PathBuf> {
    #[cfg(unix)]
    let path = path.to_string();

    #[cfg(windows)]
    let path = {
        let mut path = path.to_string();
        // safe because one ascii char is replacing another ascii char
        unsafe {
            for b in path.as_bytes_mut() {
                if *b == b'/' {
                    *b = b'\\';
                }
            }
        }
        path
    };

    let sep = path::MAIN_SEPARATOR;
    let (dir_name, file_name) = match path.rfind(sep) {
        Some(idx) => path.split_at(idx + sep.len_utf8()),
        None => ("", path.as_str()),
    };

    let home_dir = if let Some(directories) = directories::UserDirs::new() {
        directories.home_dir().into()
    } else {
        PathBuf::new()
    };

    let expanded_dir_name = if dir_name.starts_with("~") {
        let mut dir_name = dir_name.to_string();
        dir_name.replace_range(..1, &home_dir.to_string_lossy());
        dir_name
    } else {
        dir_name.to_string()
    };

    let dir_path = Path::new(&expanded_dir_name);
    let dir = if dir_path.is_relative() {
        std::env::current_dir().unwrap().join(dir_path)
    } else {
        dir_path.to_path_buf()
    };

    // if dir doesn't exist, then don't offer any completions
    if !dir.exists() {
        return Vec::new();
    }

    let mut entries: Vec<(isize, bool, PathBuf)> = Vec::new();
    let scoring = Scoring::emphasize_word_starts();

    if let Ok(read_dir) = dir.read_dir() {
        let file_name = normalize(file_name);
        for entry in read_dir.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                if file_name.is_empty() {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }

                        if !executable_only || is_executable(&metadata) || metadata.is_dir() {
                            entries.push((0, false, path.into()));
                        }
                    }
                } else {
                    let ns = normalize(s);
                    if let Some(m) = FuzzySearch::new(&file_name, &ns)
                        .score_with(&scoring)
                        .best_match()
                        && let Ok(metadata) = fs::metadata(entry.path())
                    {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }

                        if !executable_only || is_executable(&metadata) || metadata.is_dir() {
                            entries.push((m.score(), ns.starts_with(&*file_name), path.into()));
                        }
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| match b.1.cmp(&a.1) {
        Ordering::Less => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
        Ordering::Equal => b.0.cmp(&a.0),
    });
    entries.into_iter().map(|(_, _, p)| p).collect()
}

fn is_executable(_metadata: &Metadata) -> bool {
    #[cfg(unix)]
    let value = std::os::unix::fs::PermissionsExt::mode(&_metadata.permissions()) & 0o111 != 0;
    #[cfg(windows)]
    let value = true;
    value
}
