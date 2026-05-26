use std::{
    borrow::Cow,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use ferrite_geom::point::Point;
use ferrite_utility::{graphemes::RopeGraphemeExt, rope_reader::RopeReader};
use grep_matcher::Matcher as _;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, sinks::UTF8};
use ignore::{WalkBuilder, WalkState};

use super::{Matchable, file_previewer::is_text_file};
use crate::{
    buffer::Buffer,
    config::editor::PickerConfig,
    event_loop_proxy::get_proxy,
    picker::{Preview, Previewer, file_picker::filter_picker_entry},
};

pub fn global_search_injector(
    query: String,
    config: &PickerConfig,
    case_insensitive: bool,
) -> impl FnOnce(nucleo::Injector<GlobalSearchMatch>, Arc<AtomicBool>) {
    let show_hidden = config.show_hidden;
    let follow_ignore = config.follow_ignore;
    let follow_git_global = config.follow_git_global;
    let follow_gitignore = config.follow_gitignore;
    let follow_git_exclude = config.follow_git_exclude;

    move |injector, running| {
        thread::spawn(move || {
            let root = std::env::current_dir().unwrap();
            let matcher = RegexMatcherBuilder::new()
                .fixed_strings(true)
                .multi_line(false)
                .case_insensitive(case_insensitive)
                .build(&query)
                .unwrap();

            let mut builder = WalkBuilder::new(std::env::current_dir().unwrap());
            let walk_parallel = builder
                .follow_links(false)
                .hidden(!show_hidden)
                .ignore(follow_ignore)
                .git_global(follow_git_global)
                .git_ignore(follow_gitignore)
                .git_exclude(follow_git_exclude)
                .filter_entry(move |entry| filter_picker_entry(entry, &root, true))
                .build_parallel();

            walk_parallel.run(move || {
                let matcher = matcher.clone();

                let running = running.clone();
                let injector = injector.clone();
                Box::new(move |result| {
                    if !running.load(Ordering::Relaxed) {
                        tracing::debug!("Shutting down global file searcher");
                        return WalkState::Quit;
                    }
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

                    let Ok(mut buffer) = Buffer::builder().simple(true).from_file(path).build()
                    else {
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
                                injector.push(
                                    GlobalSearchMatch {
                                        buffer: buffer.clone(),
                                        name: name.clone(),
                                        line: rope_line.trim_start_whitespace().to_string(),
                                        match_location: (
                                            Point::new(start_col, lnum),
                                            Point::new(end_col, lnum),
                                        ),
                                    },
                                    |item, utf32_string| {
                                        utf32_string[0] =
                                            nucleo::Utf32String::from(&*item.display())
                                    },
                                );
                            }
                            Ok(true)
                        }),
                    ) {
                        tracing::error!("Search error: {err}");
                    }

                    WalkState::Continue
                })
            });

            get_proxy().request_render("global search injector done");
        });
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
