use std::borrow::Cow;

use super::{Matchable, SearchOptionProvider};

pub struct BufferFindProvider(pub Vec<BufferItem>);

impl SearchOptionProvider for BufferFindProvider {
    type Matchable = BufferItem;

    fn get_options(&self) -> Vec<Self::Matchable> {
        self.0.clone()
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
