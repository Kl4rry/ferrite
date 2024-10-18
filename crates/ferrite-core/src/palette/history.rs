use std::collections::VecDeque;

#[derive(Debug)]
pub struct History {
    entires: VecDeque<String>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entires: [
                String::from("ls"),
                String::from("echo ls"),
                String::from("pwd"),
            ]
            .into(),
        }
    }
}

impl History {
    pub fn add(&mut self, text: String) {
        if let Some(entry) = self.entires.back() {
            if *entry == text {
                return;
            }
        }

        self.entires.push_back(text);
        if self.entires.len() > 1000 {
            self.entires.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.entires.len()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.entires.get(index).map(|s| s.as_str())
    }
}
