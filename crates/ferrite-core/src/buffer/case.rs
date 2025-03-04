use std::str::FromStr;

use anyhow::bail;
use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToTitleCase, ToTrainCase,
};
use serde::{Deserialize, Serialize};

use super::{Buffer, ViewId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl FromStr for Case {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
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
            _ => bail!("'{s}' is not valid case"),
        })
    }
}

impl Case {
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
    // TODO make multicursor aware
    pub fn transform_case(&mut self, view_id: ViewId, case: Case) {
        self.views[view_id].cursors.clear();
        if !self.views[view_id].cursors.first().has_selection() {
            return;
        }

        self.history.begin(self.get_all_cursors(), self.dirty);
        let start_byte_idx = self.views[view_id]
            .cursors
            .first()
            .position
            .min(self.views[view_id].cursors.first().anchor);
        let end_byte_idx = self.views[view_id]
            .cursors
            .first()
            .position
            .max(self.views[view_id].cursors.first().anchor);
        let string = self.rope.slice(start_byte_idx..end_byte_idx).to_string();
        let output = case.transform(&string);

        self.history
            .replace(&mut self.rope, start_byte_idx..end_byte_idx, &output);

        if self.views[view_id].cursors.first().position < self.views[view_id].cursors.first().anchor
        {
            self.views[view_id].cursors.first_mut().position = start_byte_idx;
            self.views[view_id].cursors.first_mut().anchor = start_byte_idx + output.len();
        } else {
            self.views[view_id].cursors.first_mut().anchor = start_byte_idx;
            self.views[view_id].cursors.first_mut().position = start_byte_idx + output.len();
        }

        self.update_affinity(view_id);

        if self.views[view_id].clamp_cursor {
            self.center_on_main_cursor(view_id);
        }

        self.mark_dirty();
        self.history.finish();
    }
}
