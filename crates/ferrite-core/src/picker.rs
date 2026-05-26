use std::{
    borrow::Cow,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use ferrite_runtime::unique_id::UniqueId;
use nucleo::Nucleo;

use super::buffer::{Buffer, error::BufferError};
use crate::{
    cmd::Cmd,
    event_loop_proxy::{EventLoopProxy, UserEvent},
    mini_buffer::MiniBuffer,
};

pub mod buffer_picker;
pub mod file_picker;
pub mod file_previewer;
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

pub struct Picker<M: Matchable + Send + Sync + Clone + 'static> {
    search_field: MiniBuffer,
    selected: usize,
    previewer: Option<Box<dyn Previewer<M>>>,
    choice: Option<M>,
    nucleo: Nucleo<M>,
    running: Arc<AtomicBool>,
    unique_id: UniqueId,
}

impl<M> Picker<M>
where
    M: Matchable + Send + Sync + Clone + 'static,
{
    pub fn new(
        previewer: Option<Box<dyn Previewer<M>>>,
        proxy: Box<dyn EventLoopProxy<UserEvent>>,
    ) -> Self {
        let nucleo_proxy = proxy.dup();

        Self {
            search_field: MiniBuffer::new(),
            selected: 0,
            choice: None,
            previewer,
            nucleo: Nucleo::new(
                nucleo::Config::DEFAULT,
                Arc::new(move || nucleo_proxy.request_render("picker result recv")),
                None,
                1,
            ),
            running: Arc::new(AtomicBool::new(true)),
            unique_id: UniqueId::new(),
        }
    }
}

impl<M> Picker<M>
where
    M: Matchable + Send + Sync + Clone + 'static,
{
    pub fn search_field(&mut self) -> &mut MiniBuffer {
        &mut self.search_field
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn set_selected(&mut self, index: usize) {
        self.selected = index.min(self.get_snapshot().matched_item_count() as usize);
    }

    pub fn set_choice(&mut self, index: usize) {
        self.set_selected(index);
        let selected = self.selected;
        self.choice = self
            .get_snapshot()
            .get_matched_item(selected as u32)
            .map(|item| item.data)
            .cloned();
    }

    pub fn get_choice(&mut self) -> Option<M> {
        self.choice.take()
    }

    pub fn tick(&mut self) {
        self.nucleo.tick(10);
    }

    pub fn get_snapshot(&self) -> &nucleo::Snapshot<M> {
        self.nucleo.snapshot()
    }

    pub fn move_up(&mut self) {
        self.get_snapshot();
        if self.selected == 0 {
            let count = self.get_snapshot().matched_item_count() as usize;
            self.selected = count.saturating_sub(1);
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn move_down(&mut self) {
        self.selected += 1;
    }

    pub fn handle_input(&mut self, input: Cmd) -> Result<(), BufferError> {
        self.nucleo.tick(10);
        let mut enter = false;
        let append = false;
        match input {
            Cmd::MoveUp { .. } | Cmd::TabOrIndent { back: true } => self.move_up(),
            Cmd::MoveDown { .. } | Cmd::TabOrIndent { back: false } => self.move_down(),
            Cmd::VerticalScroll { distance } => {
                if distance.is_sign_negative() {
                    self.move_up();
                } else {
                    self.move_down()
                }
            }
            Cmd::Char { .. } => {
                // TODO check if cursor is at end
                // settings append to true makes nucleo faster
                self.search_field.handle_input(input)?;
            }
            input => enter |= self.search_field.handle_input(input)?,
        }
        self.nucleo.pattern.reparse(
            0,
            &self.search_field.buffer.to_string(),
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            append,
        );
        self.nucleo.tick(10);

        let count = self.get_snapshot().matched_item_count() as usize;
        if self.selected >= count {
            self.selected = count.saturating_sub(1);
        }

        if enter {
            self.set_choice(self.selected);
        }
        Ok(())
    }

    pub fn get_current_preview(&mut self) -> Option<Preview> {
        let snapshot = self.nucleo.snapshot();
        let item = snapshot.get_item(self.selected as u32)?;
        Some(self.previewer.as_mut()?.request_preview(item.data))
    }

    pub fn has_previewer(&self) -> bool {
        self.previewer.is_some()
    }

    pub fn unique_id(&self) -> UniqueId {
        self.unique_id
    }

    pub fn set_injector<F: FnOnce(nucleo::Injector<M>, Arc<AtomicBool>)>(&self, f: F) {
        (f)(self.nucleo.injector(), self.running.clone())
    }
}

impl<M: Matchable + Send + Sync + 'static> Drop for Picker<M> {
    fn drop(&mut self) {
        self.running.store(true, Ordering::Relaxed);
    }
}

pub trait Matchable: Clone {
    fn as_match_str(&self) -> Cow<str>;
    fn display(&self) -> Cow<str>;
}

impl Matchable for String {
    fn as_match_str(&self) -> Cow<str> {
        self.as_str().into()
    }

    fn display(&self) -> Cow<str> {
        self.as_str().into()
    }
}
