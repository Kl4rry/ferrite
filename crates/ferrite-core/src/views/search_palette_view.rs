use std::sync::Arc;

use ferrite_ctx::{ArenaString, format};
use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::tui_buf_ext::TuiBufExt;

use crate::{cmd::Cmd, config::keymap::Keymap, theme::EditorTheme};

pub struct SearchPaletteView {
    theme: Arc<EditorTheme>,
    keymap: Arc<Keymap>,
    replace: bool,
}

impl SearchPaletteView {
    pub fn new(theme: Arc<EditorTheme>, keymap: Arc<Keymap>, replace: bool) -> Self {
        Self {
            theme,
            keymap,
            replace,
        }
    }
}

impl View<()> for SearchPaletteView {
    fn render(&self, (): &mut (), bounds: Bounds, painter: &mut Painter) {
        let area = bounds.grid_bounds();
        let layer = painter.create_layer("command palette search view", bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;

        let arena = ferrite_ctx::Ctx::arena();
        let mut next = ArenaString::new_in(&arena);
        let mut prev = ArenaString::new_in(&arena);
        let mut case = ArenaString::new_in(&arena);
        let mut replace = ArenaString::new_in(&arena);
        let mut replace_match = ArenaString::new_in(&arena);
        for keymapping in &self.keymap.normal {
            match keymapping.cmd {
                Cmd::NextMatch => {
                    next = format!(in &*arena,
                        " Next match ({}{})",
                        keymapping.key.modifiers,
                        keymapping.key.keycode.to_string(),
                    )
                }
                Cmd::PrevMatch => {
                    prev = format!(in &*arena,
                        " Prev match ({}{})",
                        keymapping.key.modifiers,
                        keymapping.key.keycode.to_string(),
                    )
                }
                Cmd::CaseInsensitive if !self.replace => {
                    case = format!(in &*arena,
                        " Toggle case senitiviy ({}{})",
                        keymapping.key.modifiers,
                        keymapping.key.keycode.to_string(),
                    )
                }
                Cmd::Replace if !self.replace => {
                    replace = format!(in &*arena,
                        " Replace ({}{})",
                        keymapping.key.modifiers,
                        keymapping.key.keycode.to_string(),
                    )
                }
                Cmd::ReplaceCurrentMatch if self.replace => {
                    replace_match = format!(in &*arena,
                        " Replace ({}{})",
                        keymapping.key.modifiers,
                        keymapping.key.keycode.to_string(),
                    )
                }
                _ => continue,
            }
        }

        buf.draw_string(
            area.x as u16,
            area.y as u16,
            format!(in &*arena,"{}{}{}{}{}", case, prev, next, replace, replace_match),
            area.into(),
            self.theme.text,
        );
    }
}
