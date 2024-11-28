use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::OsString,
    fs::{self, FileType},
    path::{Path, PathBuf},
};

use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::{Rope, RopeSlice};

use crate::{
    buffer::Buffer,
    cmd::Cmd,
    picker::{fuzzy_match, Matchable},
};

slotmap::new_key_type! {
    pub struct FileExplorerId;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub file_type: FileType,
}

impl Matchable for DirEntry {
    fn as_match_str(&self) -> Cow<str> {
        self.path.file_name().unwrap().to_string_lossy()
    }

    fn display(&self) -> Cow<str> {
        self.path.file_name().unwrap().to_string_lossy()
    }
}

pub struct FileExplorer {
    path: PathBuf,
    entries: boxcar::Vec<DirEntry>,
    matching_entries: Vec<DirEntry>,
    index: usize,
    error: Option<std::io::Error>,
    pub buffer: Buffer,
    pub history: HashMap<PathBuf, OsString>,
}

impl FileExplorer {
    pub fn new(path: PathBuf) -> Self {
        let mut fe = Self {
            path: path.clone(),
            entries: boxcar::Vec::new(),
            matching_entries: Vec::new(),
            index: 0,
            error: None,
            buffer: Buffer::new(),
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

        self.entries = entries.into_iter().collect();
        self.path = path;

        self.buffer.set_text("");
        let view_id = self.buffer.get_first_view_or_create();
        self.buffer.start(view_id, false);
        self.handle_input(Cmd::Insert("".into()));

        self.index = 0;
        if let Some(name) = self.history.get(&self.path) {
            for (i, entry) in self.matching_entries.iter().enumerate() {
                if entry.path.file_name() == Some(name) {
                    self.index = i;
                }
            }
        }
    }

    pub fn handle_input(&mut self, input: Cmd) -> Option<PathBuf> {
        let mut enter = false;
        let mut new_input = false;
        match input {
            Cmd::MoveUp { .. } if !self.matching_entries.is_empty() => {
                if self.index == 0 {
                    self.index = self.matching_entries.len() - 1;
                } else {
                    self.index = self
                        .matching_entries
                        .len()
                        .saturating_sub(1)
                        .min(self.index.saturating_sub(1));
                }
            }
            Cmd::MoveDown { .. } if !self.matching_entries.is_empty() => {
                self.index += 1;
                if self.index >= self.matching_entries.len() {
                    self.index = 0;
                }
            }
            Cmd::Backspace | Cmd::BackspaceWord if self.buffer.rope().len_bytes() == 0 => {
                if let Some(parent) = self.path.parent() {
                    if let Some(file_name) = self.path.file_name() {
                        self.history.insert(parent.into(), file_name.to_owned());
                    }
                    self.change_dir(parent.into());
                }
            }
            Cmd::Insert(string) => {
                let rope = RopeSlice::from(string.as_str());
                let line = rope.line_without_line_ending(0);
                let view_id = self.buffer.get_first_view_or_create();
                let _ = self
                    .buffer
                    .handle_input(view_id, Cmd::Insert(line.to_string()));
                if line.len_bytes() != rope.len_bytes() {
                    enter = true;
                } else {
                    new_input = true;
                }
            }
            Cmd::Char(ch) if LineEnding::from_char(ch).is_some() => {
                enter = true;
            }
            cmd => {
                let view_id = self.buffer.get_first_view_or_create();
                let _ = self.buffer.handle_input(view_id, cmd);
                new_input = true;
            }
        }

        if new_input {
            let query = self.buffer.rope().to_string();
            if !query.is_empty() {
                let output = fuzzy_match::fuzzy_match::<DirEntry>(&query, &self.entries, None);
                self.matching_entries.clear();
                self.matching_entries
                    .extend(output.into_iter().map(|m| m.0.item));
            } else {
                self.matching_entries.clear();
                self.matching_entries
                    .extend(self.entries.iter().map(|(_, entry)| entry).cloned());
            }
        }

        self.index = self
            .index
            .clamp(0, self.matching_entries.len().saturating_sub(1));

        if enter && !self.matching_entries.is_empty() {
            let entry = &self.matching_entries[self.index];
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

        None
    }

    pub fn entries(&self) -> &[DirEntry] {
        &self.matching_entries
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
