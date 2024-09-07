use std::{
    borrow::Cow,
    io::{Read, Write},
    iter::Peekable,
    sync::{Arc, Mutex},
};

use ferrite_utility::{graphemes::RopeGraphemeExt, point::Point};
use grep::{
    matcher::Matcher,
    regex::RegexMatcherBuilder,
    searcher::{sinks::UTF8, Searcher},
};
use ignore::{WalkBuilder, WalkState};
use ropey::{iter::Chunks, Rope};

use super::{Matchable, PickerOptionProvider};
use crate::{
    buffer::Buffer,
    config::PickerConfig,
    picker::{Preview, Previewer},
};

struct RopeReader<'a> {
    chunks: Peekable<Chunks<'a>>,
    bytes_read: usize,
}

impl<'a> RopeReader<'a> {
    pub fn new(rope: &'a Rope) -> Self {
        Self {
            chunks: rope.chunks().peekable(),
            bytes_read: 0,
        }
    }
}

impl Read for RopeReader<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        match self.chunks.peek() {
            Some(chunk) => {
                let current = &chunk.as_bytes()[self.bytes_read..];
                let bytes_to_read = buf.len().min(current.len());
                // This will always work so we can ignore the error
                let _ = buf.write_all(&chunk.as_bytes()[..bytes_to_read]);

                if bytes_to_read == current.len() {
                    self.chunks.next();
                }
                Ok(bytes_to_read)
            }
            None => Ok(0),
        }
    }
}

pub struct GlobalSearchProvider {
    output: Arc<boxcar::Vec<GlobalSearchMatch>>,
    config: PickerConfig,
    case_insenstive: bool,
    query: String,
}

impl GlobalSearchProvider {
    pub fn new(query: String, config: PickerConfig, case_insenstive: bool) -> Self {
        Self {
            output: Arc::new(boxcar::Vec::new()),
            config,
            case_insenstive,
            query,
        }
    }
}

impl PickerOptionProvider for GlobalSearchProvider {
    type Matchable = GlobalSearchMatch;

    fn get_options_reciver(&self) -> cb::Receiver<Arc<boxcar::Vec<Self::Matchable>>> {
        let (tx, rx) = cb::unbounded();

        let matcher = RegexMatcherBuilder::new()
            .fixed_strings(true)
            .multi_line(false)
            .case_insensitive(self.case_insenstive)
            .build(&self.query)
            .unwrap();

        let mut builder = WalkBuilder::new(std::env::current_dir().unwrap());
        let walk_parallel = builder
            .follow_links(false)
            .ignore(self.config.follow_ignore)
            .git_global(self.config.follow_git_global)
            .git_ignore(self.config.follow_gitignore)
            .git_exclude(self.config.follow_git_exclude)
            .build_parallel();

        walk_parallel.run(move || {
            let matcher = matcher.clone();
            let output = self.output.clone();
            let tx = tx.clone();

            Box::new(move |result| {
                let dir_entry = match result {
                    Ok(entry) => {
                        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                            return WalkState::Continue;
                        }
                        entry
                    }
                    Err(_) => return WalkState::Continue,
                };

                let path = dir_entry.path();
                let Ok(mut buffer) = Buffer::from_file(path) else {
                    return WalkState::Continue;
                };

                buffer.clamp_cursor = true;
                let name = buffer.name().to_string();
                let rope = buffer.rope().clone();
                let buffer = Arc::new(Mutex::new(buffer));

                let mut i = 0;
                if let Err(err) = Searcher::new().search_reader(
                    &matcher,
                    RopeReader::new(&rope.clone()),
                    UTF8(|lnum, line| {
                        if let Some(mymatch) = matcher.find(line.as_bytes())? {
                            let lnum = lnum as usize - 1;
                            let rope_line = rope.line(lnum);
                            let start_col = rope_line.byte_to_col(mymatch.start());
                            let end_col = rope_line.byte_to_col(mymatch.end());
                            output.push(GlobalSearchMatch {
                                buffer: buffer.clone(),
                                name: name.clone(),
                                match_location: (
                                    Point::new(start_col, lnum),
                                    Point::new(end_col, lnum),
                                ),
                            });
                            let _ = tx.send(self.output.clone());
                            i += 1;
                        }
                        Ok(true)
                    }),
                ) {
                    tracing::error!("Search error: {err}");
                }

                WalkState::Continue
            })
        });

        rx
    }
}

#[derive(Clone)]
pub struct GlobalSearchMatch {
    pub buffer: Arc<Mutex<Buffer>>,
    pub name: String,
    pub match_location: (Point<usize>, Point<usize>),
}

impl Matchable for GlobalSearchMatch {
    fn as_match_str(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }

    fn display(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }
}

pub struct GlobalSearchPreviewer;
impl Previewer<GlobalSearchMatch> for GlobalSearchPreviewer {
    fn request_preview(&mut self, m: &GlobalSearchMatch) -> Preview {
        {
            let mut guard = m.buffer.lock().unwrap();
            let (start, end) = m.match_location;
            guard.select_area(start, end, false);
            guard.clamp_cursor = true;
            guard.center_on_cursor();
        }
        Preview::SharedBuffer(m.buffer.clone())
    }
}
