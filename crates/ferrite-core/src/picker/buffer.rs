use std::{borrow::Cow, sync::Arc};

use slotmap::SlotMap;

use super::{Matchable, PickerOptionProvider};
use crate::{
    buffer::Buffer,
    picker::{Preview, Previewer},
    workspace::BufferId,
};

pub struct BufferFindProvider(pub Arc<boxcar::Vec<BufferItem>>);

impl PickerOptionProvider for BufferFindProvider {
    type Matchable = BufferItem;

    fn get_options_reciver(&self) -> cb::Receiver<Arc<boxcar::Vec<Self::Matchable>>> {
        let (tx, rx) = cb::bounded(1);
        let _ = tx.send(self.0.clone());
        rx
    }
}

#[derive(Debug, Clone)]
pub struct BufferItem {
    pub id: BufferId,
    pub name: String,
    pub dirty: bool,
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
