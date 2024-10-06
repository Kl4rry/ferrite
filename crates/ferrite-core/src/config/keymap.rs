use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    cmd::Cmd,
    keymap::{Exclusiveness, Key},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Keymapping {
    pub key: Key,
    pub cmd: Cmd,
    pub exclusiveness: Exclusiveness,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keymap {
    pub normal: Vec<Keymapping>,
    #[serde(flatten)]
    pub input_modes: HashMap<String, Vec<Keymapping>>,
}

impl Keymap {
    pub fn create_default_config(overwrite: bool) -> Result<()> {
        let config = Self::get_default_location()?;

        let mut config_folder = config.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        if !config.exists() || overwrite {
            fs::write(
                config,
                serde_json::to_string_pretty(&Self::default()).unwrap(),
            )?;
        }

        Ok(())
    }

    pub fn load_from_default_location() -> Result<Self> {
        let path = Self::get_default_location()?;

        let mut config_folder = path.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
    }

    pub fn get_default_location() -> Result<PathBuf> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };
        Ok(directories.config_dir().join("keymap.json"))
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
