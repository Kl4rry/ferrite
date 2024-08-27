use std::{
    borrow::Cow,
    fs,
    path::{self, Path, PathBuf},
};

#[cfg(any(windows, target_os = "macos"))]
fn normalize(s: &str) -> Cow<str> {
    // case insensitive
    Cow::Owned(s.to_lowercase())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn normalize(s: &str) -> Cow<str> {
    Cow::Borrowed(s)
}

pub fn complete_file_path(path: &str) -> Vec<PathBuf> {
    #[cfg(unix)]
    let path = path.to_string();

    #[cfg(windows)]
    let mut path = path.to_string();

    #[cfg(windows)]
    unsafe {
        // safe because one ascii char is replacing another ascii char
        for b in path.as_bytes_mut() {
            if *b == b'/' {
                *b = b'\\';
            }
        }
    }

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

    let mut entries: Vec<PathBuf> = Vec::new();

    // if dir doesn't exist, then don't offer any completions
    if !dir.exists() {
        return entries;
    }

    if let Ok(read_dir) = dir.read_dir() {
        let file_name = normalize(file_name);
        for entry in read_dir.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                let ns = normalize(s);
                if ns.starts_with(file_name.as_ref()) {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }

                        entries.push(path.into());
                    }
                }
            }
        }
    }

    entries
}
