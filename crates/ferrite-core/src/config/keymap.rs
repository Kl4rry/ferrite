use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    cmd::Cmd,
    config::{editor::KeymapAndMetadata, Editor},
    keymap::{Exclusiveness, Key},
};
#[derive(Debug, Serialize, Deserialize)]
pub struct Keymapping {
    pub key: Key,
    pub cmd: Cmd,
    pub exclusiveness: Exclusiveness,
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
                exclusiveness,
            },
        ) in &editor.keymap
        {
            let keymapping = Keymapping {
                key: key.clone(),
                cmd: cmd.clone(),
                exclusiveness: *exclusiveness,
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
                    exclusiveness: keymapping.exclusiveness,
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
                        exclusiveness: keymapping.exclusiveness,
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
                .map(|(key, cmd, exclusiveness)| Keymapping {
                    key,
                    cmd,
                    exclusiveness,
                })
                .collect(),
            input_modes: {
                let mut hash_map = HashMap::new();
                hash_map.insert(
                    "chords".into(),
                    crate::keymap::get_default_choords()
                        .into_iter()
                        .map(|(key, cmd, exclusiveness)| Keymapping {
                            key,
                            cmd,
                            exclusiveness,
                        })
                        .collect(),
                );
                hash_map
            },
        }
    }
}
