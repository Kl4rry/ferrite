use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    cmd::Cmd,
    config::{editor::KeymapAndMetadata, Editor},
    keymap::Key,
};
#[derive(Debug, Serialize, Deserialize)]
pub struct Keymapping {
    pub key: Key,
    pub cmd: Cmd,
    pub ignore_modifiers: bool,
}

#[derive(Debug)]
pub struct Keymap {
    pub normal: Vec<Keymapping>,
    pub input_modes: HashMap<String, Vec<Keymapping>>,
}

impl Keymap {
    pub fn from_editor(editor: &Editor) -> Self {
        let mut default = Self::default();
        for (
            key,
            KeymapAndMetadata {
                mode,
                cmd,
                ignore_modifiers,
            },
        ) in &editor.keymap
        {
            let keymapping = Keymapping {
                key: key.clone(),
                cmd: cmd.clone(),
                ignore_modifiers: *ignore_modifiers,
            };
            if mode == "normal" {
                default.normal.insert(0, keymapping);
            } else {
                default
                    .input_modes
                    .entry(mode.clone())
                    .or_default()
                    .insert(0, keymapping);
            }
        }
        default
    }

    pub fn to_map(&self) -> HashMap<Key, KeymapAndMetadata> {
        let keymap = Keymap::default();
        let mut output = HashMap::new();
        for keymapping in keymap.normal {
            output.insert(
                keymapping.key,
                KeymapAndMetadata {
                    cmd: keymapping.cmd,
                    ignore_modifiers: keymapping.ignore_modifiers,
                    mode: String::from("normal"),
                },
            );
        }

        for (mode, keymap) in keymap.input_modes {
            for keymapping in keymap {
                output.insert(
                    keymapping.key,
                    KeymapAndMetadata {
                        cmd: keymapping.cmd,
                        ignore_modifiers: keymapping.ignore_modifiers,
                        mode: mode.clone(),
                    },
                );
            }
        }
        output
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            normal: crate::keymap::get_default_mappings()
                .into_iter()
                .map(|(key, cmd, ignore_modifiers)| Keymapping {
                    key,
                    cmd,
                    ignore_modifiers,
                })
                .collect(),
            input_modes: {
                let mut hash_map = HashMap::new();
                hash_map.insert(
                    "chords".into(),
                    crate::keymap::get_default_chords()
                        .into_iter()
                        .map(|(key, cmd, ignore_modifiers)| Keymapping {
                            key,
                            cmd,
                            ignore_modifiers,
                        })
                        .collect(),
                );
                hash_map
            },
        }
    }
}
