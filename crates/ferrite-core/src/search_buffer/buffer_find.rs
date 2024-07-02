use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};

use super::{Matchable, SearchOptionProvider};

pub struct BufferFindProvider(pub Arc<RwLock<Vec<BufferItem>>>);

impl SearchOptionProvider for BufferFindProvider {
    type Matchable = BufferItem;

    fn get_options_reciver(&self) -> cb::Receiver<Arc<RwLock<Vec<Self::Matchable>>>> {
        let (tx, rx) = cb::bounded(1);
        let _ = tx.send(self.0.clone());
        rx
    }
}

#[derive(Debug, Clone)]
pub struct BufferItem {
    pub id: usize,
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
