use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use cb::select;
use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::RopeSlice;

use self::fuzzy_match::FuzzyMatch;
use super::buffer::{Buffer, error::BufferError};
use crate::{buffer::ViewId, cmd::Cmd, event_loop_proxy::EventLoopProxy};

pub mod buffer_picker;
pub mod file_picker;
pub mod file_previewer;
pub mod file_scanner;
pub mod fuzzy_match;
pub mod global_search_picker;

pub enum Preview<'a> {
    Buffer(&'a mut Buffer),
    SharedBuffer(Arc<Mutex<Buffer>>),
    Loading,
    Binary, // TODO add hex preview
    TooLarge,
    Err,
}

pub trait Previewer<M: Matchable> {
    fn request_preview(&mut self, m: &M) -> Preview;
}

pub struct PickerResult<M: Matchable> {
    matches: Vec<(FuzzyMatch<M>, usize)>,
    total: usize,
}

pub struct Picker<M: Matchable> {
    search_field: Buffer,
    view_id: ViewId,
    selected: usize,
    previewer: Option<Box<dyn Previewer<M>>>,
    result: PickerResult<M>,
    choice: Option<M>,
    tx: cb::Sender<String>,
    rx: cb::Receiver<PickerResult<M>>,
}

impl<M> Picker<M>
where
    M: Matchable + Send + Sync + Clone + 'static,
{
    pub fn new<T: PickerOptionProvider<Matchable = M> + Send + Sync + 'static>(
        option_provder: T,
        previewer: Option<Box<dyn Previewer<M>>>,
        proxy: Box<dyn EventLoopProxy>,
        path: Option<PathBuf>,
    ) -> Self {
        let mut search_field = Buffer::new();
        let view_id = search_field.create_view();
        search_field.set_view_lines(view_id, 1);

        let (search_tx, search_rx): (_, cb::Receiver<String>) = cb::unbounded();
        let (result_tx, result_rx): (_, cb::Receiver<PickerResult<M>>) = cb::unbounded();

        thread::spawn(move || {
            let mut options = Arc::new(boxcar::Vec::new());
            let mut query = String::new();
            let options_recv = option_provder.get_options_reciver();

            loop {
                select! {
                    recv(search_rx) -> new_query => {
                        match new_query {
                            Ok(new_query) => {
                                query = new_query;
                            }
                            Err(_) => break,
                        }
                    }
                    recv(options_recv) -> new_options => {
                        match new_options {
                            Ok(new_options) => {
                                options = new_options;
                            }
                            Err(_) => {
                                match search_rx.recv() {
                                    Ok(new_query) => {
                                        query = new_query;
                                    }
                                    Err(_) => break,
                                }
                            },
                        }
                    }
                }

                if !search_rx.is_empty() || !options_recv.is_empty() {
                    proxy.request_render();
                    continue;
                }

                {
                    let output = fuzzy_match::fuzzy_match::<M>(&query, &*options, path.as_deref());
                    let result = PickerResult {
                        matches: output,
                        total: options.count(),
                    };
                    if result_tx.send(result).is_err() {
                        break;
                    }
                }

                proxy.request_render();
            }
        });

        Self {
            search_field,
            view_id,
            selected: 0,
            choice: None,
            previewer,
            tx: search_tx,
            rx: result_rx,
            result: PickerResult {
                matches: Vec::new(),
                total: 0,
            },
        }
    }
}

impl<M> Picker<M>
where
    M: Matchable,
{
    pub fn search_field(&mut self) -> &mut Buffer {
        &mut self.search_field
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn get_choice(&mut self) -> Option<M> {
        self.choice.take()
    }

    fn poll_rx(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.result = result;
        }
    }

    pub fn get_matches(&mut self) -> &[(FuzzyMatch<M>, usize)] {
        self.poll_rx();
        &self.result.matches
    }

    pub fn get_total(&mut self) -> usize {
        self.poll_rx();
        self.result.total
    }

    pub fn handle_input(&mut self, input: Cmd) -> Result<(), BufferError> {
        let mut enter = false;
        match input {
            Cmd::MoveUp { .. } => {
                if self.selected == 0 {
                    self.selected = self.get_matches().len().saturating_sub(1);
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            Cmd::MoveDown { .. } | Cmd::TabOrIndent { .. } => self.selected += 1,
            Cmd::Insert { text } => {
                let rope = RopeSlice::from(text.as_str());
                let line = rope.line_without_line_ending(0);
                self.search_field.handle_input(
                    self.view_id,
                    Cmd::Insert {
                        text: line.to_string(),
                    },
                )?;
                if line.len_bytes() != rope.len_bytes() {
                    enter = true;
                } else {
                    let _ = self.tx.send(self.search_field.to_string());
                }
            }
            Cmd::Char { ch } if LineEnding::from_char(ch).is_some() => {
                enter = true;
            }
            input => {
                self.search_field.handle_input(self.view_id, input)?;
                let _ = self.tx.send(self.search_field.to_string());
            }
        }

        if self.selected >= self.get_matches().len() {
            self.selected = 0;
        }

        if enter {
            let selected = self.selected;
            self.choice = self
                .get_matches()
                .get(selected)
                .map(|(FuzzyMatch { item, .. }, _)| item)
                .cloned();
        }
        Ok(())
    }

    pub fn get_current_preview(&mut self) -> Option<Preview> {
        let selected = self.selected;
        let (choice, _) = &self.result.matches.get(selected)?;
        let choice = &choice.item;
        Some(self.previewer.as_mut()?.request_preview(choice))
    }

    pub fn has_previewer(&self) -> bool {
        self.previewer.is_some()
    }
}

pub trait Matchable: Clone {
    fn as_match_str(&self) -> Cow<str>;
    fn display(&self) -> Cow<str>;
}

pub trait PickerOptionProvider {
    type Matchable: Matchable;
    fn get_options_reciver(&self) -> cb::Receiver<Arc<boxcar::Vec<Self::Matchable>>>;
}

impl Matchable for String {
    fn as_match_str(&self) -> Cow<str> {
        self.as_str().into()
    }

    fn display(&self) -> Cow<str> {
        self.as_str().into()
    }
}
