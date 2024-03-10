use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToTitleCase, ToTrainCase,
};

use super::Buffer;

#[derive(Debug, Clone)]
pub enum Case {
    Lower,
    Upper,
    Snake,
    Kebab,
    Camel,
    Pascal,
    Title,
    Train,
    ScreamingSnake,
    ScreamingKebab,
}

impl Case {
    pub fn from_str(s: &str) -> Self {
        match s {
            "lower" => Case::Lower,
            "upper" => Case::Upper,
            "snake" => Case::Snake,
            "kebab" => Case::Kebab,
            "camel" => Case::Camel,
            "pascal" => Case::Pascal,
            "title" => Case::Title,
            "train" => Case::Train,
            "screaming-snake" => Case::ScreamingSnake,
            "screaming-kebab" => Case::ScreamingKebab,
            _ => panic!("'{s}' is not valid case"),
        }
    }

    pub fn transform(&self, s: &str) -> String {
        match self {
            Case::Lower => s.to_lowercase(),
            Case::Upper => s.to_uppercase(),
            Case::Snake => s.to_snake_case(),
            Case::Kebab => s.to_kebab_case(),
            Case::Camel => s.to_lower_camel_case(),
            Case::Pascal => s.to_pascal_case(),
            Case::Title => s.to_title_case(),
            Case::Train => s.to_train_case(),
            Case::ScreamingSnake => s.to_shouty_snake_case(),
            Case::ScreamingKebab => s.to_shouty_kebab_case(),
        }
    }
}

impl Buffer {
    pub fn transform_case(&mut self, case: Case) {
        if !self.cursor.has_selection() {
            return;
        }

        self.history.begin(self.cursor, self.dirty);
        let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
        let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
        let string = self.rope.slice(start_byte_idx..end_byte_idx).to_string();
        let output = case.transform(&string);

        self.history
            .replace(&mut self.rope, start_byte_idx..end_byte_idx, &output);

        if self.cursor.position < self.cursor.anchor {
            self.cursor.position = start_byte_idx;
            self.cursor.anchor = start_byte_idx + output.len();
        } else {
            self.cursor.anchor = start_byte_idx;
            self.cursor.position = start_byte_idx + output.len();
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }

        self.mark_dirty();
        self.history.finish();
    }
}
