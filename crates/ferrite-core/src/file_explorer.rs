use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, FileType},
    path::{Path, PathBuf},
};

use ropey::Rope;

use crate::cmd::Cmd;

slotmap::new_key_type! {
    pub struct FileExplorerId;
}

pub struct DirEntry {
    pub path: PathBuf,
    pub file_type: FileType,
}

pub struct FileExplorer {
    path: PathBuf,
    entries: Vec<DirEntry>,
    index: usize,
    error: Option<std::io::Error>,
    pub history: HashMap<PathBuf, OsString>,
}

impl FileExplorer {
    pub fn new(path: PathBuf) -> Self {
        let mut fe = Self {
            path: path.clone(),
            entries: Vec::new(),
            index: 0,
            error: None,
            history: HashMap::new(),
        };
        fe.change_dir(path);
        fe
    }

    pub fn change_dir(&mut self, path: PathBuf) {
        let mut entries = Vec::new();
        match fs::read_dir(&path) {
            Ok(dir) => {
                for entry in dir.filter_map(|e| e.ok()) {
                    let Ok(file_type) = entry.file_type() else {
                        continue;
                    };
                    let path = entry.path();
                    let string = path.to_string_lossy();
                    let rope = Rope::from_str(&string);
                    if rope.len_lines() > 1 {
                        tracing::error!("Error file path line break");
                        continue;
                    }
                    entries.push(DirEntry { path, file_type });
                }
                self.error = None;
            }
            Err(err) => {
                self.error = Some(err);
                return;
            }
        }

        entries.sort_by(|a, b| {
            lexical_sort::natural_lexical_cmp(
                &a.path.file_name().unwrap().to_string_lossy(),
                &b.path.file_name().unwrap().to_string_lossy(),
            )
        });

        if let Some(file_name) = self
            .entries
            .get(self.index)
            .and_then(|p| p.path.file_name())
        {
            self.history.insert(self.path.clone(), file_name.to_owned());
        }

        self.entries = entries;
        self.path = path;

        self.index = 0;
        if let Some(name) = self.history.get(&self.path) {
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.path.file_name() == Some(name) {
                    self.index = i;
                }
            }
        }
    }

    pub fn handle_input(&mut self, input: Cmd) -> Option<PathBuf> {
        match input {
            Cmd::MoveUp { .. } if !self.entries.is_empty() => {
                if self.index == 0 {
                    self.index = self.entries.len() - 1;
                } else {
                    self.index = self
                        .entries
                        .len()
                        .saturating_sub(1)
                        .min(self.index.saturating_sub(1));
                }
            }
            Cmd::MoveDown { .. } if !self.entries.is_empty() => {
                self.index += 1;
                if self.index >= self.entries.len() {
                    self.index = 0;
                }
            }
            Cmd::MoveRight { .. } if !self.entries.is_empty() => {
                let entry = &self.entries[self.index];
                if entry.file_type.is_dir() {
                    self.change_dir(entry.path.to_path_buf());
                }
            }
            Cmd::MoveLeft { .. } => {
                if let Some(parent) = self.path.parent() {
                    if let Some(file_name) = self.path.file_name() {
                        self.history.insert(parent.into(), file_name.to_owned());
                    }
                    self.change_dir(parent.into());
                }
            }
            Cmd::Char(ch) if !self.entries.is_empty() => {
                if ch == '\n' {
                    let entry = &self.entries[self.index];
                    let path = if entry.file_type.is_symlink() {
                        fs::read_link(&entry.path).ok()?
                    } else {
                        entry.path.clone()
                    };
                    if path.is_file() {
                        return Some(entry.path.clone());
                    } else if path.is_dir() {
                        self.change_dir(entry.path.clone());
                    }
                }
            }
            _ => (),
        }

        None
    }

    pub fn entries(&self) -> &[DirEntry] {
        &self.entries
    }

    pub fn directory(&self) -> &Path {
        &self.path
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn error(&self) -> &Option<std::io::Error> {
        &self.error
    }
}
