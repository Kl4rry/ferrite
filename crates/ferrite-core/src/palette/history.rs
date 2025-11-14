use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct History {
    entires: VecDeque<String>,
}

impl History {
    pub fn add(&mut self, text: String) {
        if let Some(entry) = self.entires.back()
            && *entry == text
        {
            return;
        }

        self.entires.push_front(text);
        if self.entires.len() > 1000 {
            self.entires.pop_back();
        }
    }

    pub fn len(&self) -> usize {
        self.entires.len()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.entires.get(index).map(|s| s.as_str())
    }
}
