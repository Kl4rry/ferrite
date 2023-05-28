use std::{borrow::Cow, sync::Arc, thread};

use cb::select;
use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use self::fuzzy_match::FuzzyMatch;
use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::{event_loop::TuiEventLoopProxy, input::InputCommand};

pub mod buffer_find;
pub mod file_daemon;
pub mod file_find;
pub mod fuzzy_match;

pub struct SearchBuffer<M: Matchable> {
    search_field: Buffer,
    selected: usize,
    result: Vec<FuzzyMatch<M>>,
    choice: Option<M>,
    tx: cb::Sender<String>,
    rx: cb::Receiver<Vec<FuzzyMatch<M>>>,
}

impl<M> SearchBuffer<M>
where
    M: Matchable + Send + Sync + Clone + 'static,
{
    pub fn new<T: SearchOptionProvider<Matchable = M> + Send + Sync + 'static>(
        option_provder: T,
        proxy: TuiEventLoopProxy,
    ) -> Self {
        let mut search_field = Buffer::new();
        search_field.set_view_lines(1);

        let (search_tx, search_rx): (_, cb::Receiver<String>) = cb::unbounded();
        let (result_tx, result_rx): (_, cb::Receiver<Vec<FuzzyMatch<_>>>) = cb::unbounded();

        thread::spawn(move || {
            let mut options = Arc::new(Vec::new());
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
                        if let Ok(new_options) = new_options {
                            options = new_options;
                        }
                    }
                }

                let output = fuzzy_match::fuzzy_match(&query, (*options).clone());
                if result_tx.send(output).is_err() {
                    break;
                }

                proxy.request_render();
            }
        });

        Self {
            search_field,
            selected: 0,
            choice: None,
            tx: search_tx,
            rx: result_rx,
            result: Vec::new(),
        }
    }
}

impl<M> SearchBuffer<M>
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

    pub fn get_result(&mut self) -> &[FuzzyMatch<M>] {
        if let Ok(result) = self.rx.try_recv() {
            self.result = result;
            self.selected = 0;
        }
        &self.result
    }

    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        let mut enter = false;
        match input {
            InputCommand::MoveUp { .. } => self.selected = self.selected.saturating_sub(1),
            InputCommand::MoveDown { .. } | InputCommand::Tab { .. } => self.selected += 1,
            InputCommand::Insert(string) => {
                let rope = RopeSlice::from(string.as_str());
                let line = rope.line_without_line_ending(0);
                self.search_field
                    .handle_input(InputCommand::Insert(line.to_string()))?;
                if line.len_bytes() != rope.len_bytes() {
                    enter = true;
                } else {
                    let _ = self.tx.send(self.search_field.to_string());
                }
            }
            InputCommand::Char(ch) if LineEnding::from_char(ch).is_some() => {
                enter = true;
            }
            input => {
                self.search_field.handle_input(input)?;
                let _ = self.tx.send(self.search_field.to_string());
            }
        }

        if self.selected >= self.get_result().len() {
            self.selected = 0;
        }

        if enter {
            let selected = self.selected;
            self.choice = self
                .get_result()
                .get(selected)
                .map(|FuzzyMatch { item, .. }| item)
                .cloned();
        }
        Ok(())
    }
}

pub trait Matchable: Clone {
    fn as_match_str(&self) -> Cow<str>;
    fn display(&self) -> Cow<str>;
}

pub trait SearchOptionProvider {
    type Matchable: Matchable;
    fn get_options_reciver(&self) -> cb::Receiver<Arc<Vec<Self::Matchable>>>;
}

impl Matchable for String {
    fn as_match_str(&self) -> Cow<str> {
        self.as_str().into()
    }

    fn display(&self) -> Cow<str> {
        self.as_str().into()
    }
}

impl Matchable for &str {
    fn as_match_str(&self) -> Cow<str> {
        Cow::Borrowed(self)
    }

    fn display(&self) -> Cow<str> {
        Cow::Borrowed(self)
    }
}
