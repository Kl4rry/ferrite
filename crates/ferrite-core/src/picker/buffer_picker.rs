use std::{
    borrow::Cow,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

use slotmap::SlotMap;

use super::Matchable;
use crate::{
    buffer::Buffer,
    event_loop_proxy::get_proxy,
    picker::{Preview, Previewer},
    workspace::BufferId,
};

pub fn buffer_injector(
    buffers: Vec<BufferItem>,
) -> impl FnOnce(nucleo::Injector<BufferItem>, Arc<AtomicBool>) {
    |injector, _running| {
        for buffer in buffers {
            injector.push(buffer, |item, utf32_string| {
                utf32_string[0] = nucleo::Utf32String::from(item.name.clone());
            });
        }
        get_proxy().request_render("buffer injector done");
    }
}

#[derive(Debug, Clone)]
pub struct BufferItem {
    pub id: BufferId,
    pub name: String,
    pub dirty: bool,
    pub order: Instant,
}

impl Matchable for BufferItem {
    fn as_match_str(&self) -> Cow<str> {
        self.name.as_str().into()
    }

    fn display(&self) -> Cow<str> {
        let mut output = Cow::Borrowed(self.name.as_str());
        if self.dirty {
            output += " (*)";
        }
        output
    }
}

impl Previewer<BufferItem> for SlotMap<BufferId, Buffer> {
    fn request_preview(&mut self, m: &BufferItem) -> Preview {
        match self.get_mut(m.id) {
            Some(buffer) => Preview::Buffer(buffer),
            None => Preview::Err,
        }
    }
}
