use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    thread,
};

use ferrite_geom::point::Point;
use ferrite_utility::{graphemes::RopeGraphemeExt, rope_reader::RopeReader};
use grep_matcher::Matcher as _;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, sinks::UTF8};
use ignore::{WalkBuilder, WalkState};

use super::{Matchable, PickerOptionProvider, file_previewer::is_text_file};
use crate::{
    buffer::Buffer,
    config::editor::PickerConfig,
    picker::{Preview, Previewer},
};

pub struct GlobalSearchProvider {
    output: Arc<boxcar::Vec<GlobalSearchMatch>>,
    config: PickerConfig,
    case_insensitive: bool,
    query: String,
}

impl GlobalSearchProvider {
    pub fn new(query: String, config: PickerConfig, case_insensitive: bool) -> Self {
        Self {
            output: Arc::new(boxcar::Vec::new()),
            config,
            case_insensitive,
            query,
        }
    }
}

impl PickerOptionProvider for GlobalSearchProvider {
    type Matchable = GlobalSearchMatch;

    fn get_options_reciver(&self) -> cb::Receiver<Arc<boxcar::Vec<Self::Matchable>>> {
        let (tx, rx) = cb::unbounded();
        let case_insensitive = self.case_insensitive;
        let query = self.query.clone();
        let config = self.config;
        let output = self.output.clone();

        thread::spawn(move || {
            let matcher = RegexMatcherBuilder::new()
                .fixed_strings(true)
                .multi_line(false)
                .case_insensitive(case_insensitive)
                .build(&query)
                .unwrap();

            let mut builder = WalkBuilder::new(std::env::current_dir().unwrap());
            let walk_parallel = builder
                .follow_links(false)
                .hidden(!config.show_hidden)
                .ignore(config.follow_ignore)
                .git_global(config.follow_git_global)
                .git_ignore(config.follow_gitignore)
                .git_exclude(config.follow_git_exclude)
                .overrides(config.overrides())
                .build_parallel();

            walk_parallel.run(move || {
                let matcher = matcher.clone();
                let output = output.clone();
                let tx = tx.clone();

                Box::new(move |result| {
                    let dir_entry = match result {
                        Ok(entry) => {
                            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                                return WalkState::Continue;
                            }
                            entry
                        }
                        Err(_) => return WalkState::Continue,
                    };

                    let path = dir_entry.path();
                    if !is_text_file(path).unwrap_or(false) {
                        return WalkState::Continue;
                    }
                    let Ok(mut buffer) = Buffer::builder().from_file(path).build() else {
                        return WalkState::Continue;
                    };

                    let view_id = buffer.create_view();
                    buffer.views[view_id].clamp_cursor = true;
                    let name = buffer.name().to_string();
                    let rope = buffer.rope().clone();
                    let buffer = Arc::new(Mutex::new(buffer));

                    if let Err(err) = Searcher::new().search_reader(
                        &matcher,
                        RopeReader::new(rope.slice(..)),
                        UTF8(|lnum, line| {
                            if let Some(mymatch) = matcher.find(line.as_bytes())? {
                                let lnum = lnum as usize - 1;
                                let rope_line = rope.line(lnum);
                                let start_col = rope_line.byte_to_col(mymatch.start());
                                let end_col = rope_line.byte_to_col(mymatch.end());
                                output.push(GlobalSearchMatch {
                                    buffer: buffer.clone(),
                                    name: name.clone(),
                                    line: rope_line.trim_start_whitespace().to_string(),
                                    match_location: (
                                        Point::new(start_col, lnum),
                                        Point::new(end_col, lnum),
                                    ),
                                });
                                let _ = tx.send(output.clone());
                            }
                            Ok(true)
                        }),
                    ) {
                        tracing::error!("Search error: {err}");
                    }

                    WalkState::Continue
                })
            });
        });

        rx
    }
}

#[derive(Clone)]
pub struct GlobalSearchMatch {
    pub buffer: Arc<Mutex<Buffer>>,
    pub name: String,
    pub line: String,
    pub match_location: (Point<usize>, Point<usize>),
}

impl Matchable for GlobalSearchMatch {
    fn as_match_str(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }

    fn display(&self) -> Cow<str> {
        format!(
            "{}:{}: {}",
            self.name, self.match_location.0.line, self.line
        )
        .into()
    }
}

pub struct GlobalSearchPreviewer;
impl Previewer<GlobalSearchMatch> for GlobalSearchPreviewer {
    fn request_preview(&mut self, m: &GlobalSearchMatch) -> Preview {
        {
            let mut guard = m.buffer.lock().unwrap();
            let (start, end) = m.match_location;
            let view_id = guard.get_first_view().unwrap();
            guard.select_area(view_id, start, end);
            guard.views[view_id].clamp_cursor = true;
            guard.center_on_main_cursor(view_id);
        }
        Preview::SharedBuffer(m.buffer.clone())
    }
}
