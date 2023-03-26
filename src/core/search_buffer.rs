use std::{borrow::Cow, sync::mpsc, thread};

use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::{event_loop::TuiEventLoopProxy, input::InputCommand};

pub mod buffer_find;
pub mod file_find;
pub mod fuzzy_match;

#[derive(Debug)]
pub struct SearchBuffer<M> {
    search_field: Buffer,
    selected: usize,
    result: Vec<M>,
    choice: Option<M>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<Vec<M>>,
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
        search_field.clamp_cursor = false;

        let (search_tx, search_rx): (_, mpsc::Receiver<String>) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        thread::spawn(move || {
            let options = option_provder.get_options();
            if result_tx.send(options.clone()).is_err() {
                return;
            }
            proxy.request_render();
            while let Ok(term) = search_rx.recv() {
                let output = fuzzy_match::fuzzy_match(&term, options.clone());
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
    M: Clone,
{
    pub fn search_field(&self) -> &Buffer {
        &self.search_field
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn get_choice(&mut self) -> Option<M> {
        self.choice.take()
    }

    pub fn get_result(&mut self) -> &[M] {
        if let Ok(result) = self.rx.try_recv() {
            self.result = result;
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
                    self.selected = 0;
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

        if self.selected > self.get_result().len() {
            self.selected = 0;
        }

        if enter {
            let selected = self.selected;
            self.choice = self.get_result().get(selected).cloned();
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
    fn get_options(&self) -> Vec<Self::Matchable>;
}

impl Matchable for String {
    fn as_match_str(&self) -> Cow<str> {
        self.as_str().into()
    }

    fn display(&self) -> Cow<str> {
        self.as_str().into()
    }
}
