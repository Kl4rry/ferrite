use std::{collections::HashMap, path::PathBuf};

use ferrite_utility::vec1::Vec1;
use serde::{Deserialize, Serialize};

use crate::workspace::{Cursor, History, Indentation, Layout, PaletteMode};

#[derive(Serialize, Deserialize)]
pub struct Workspace {
    pub buffers: Vec<Buffer>,
    pub open_buffers: Vec<PathBuf>,
    pub layout: Layout,
    #[serde(default)] // Default as old data might not have this field
    pub palette_histories: HashMap<PaletteMode, History>,
    #[serde(default)] // Default as old data might not have this field
    pub jump_list: JumpList,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JumpList {
    pub stack: Vec<JumpPoint>,
    pub current_point: i64,
}

#[derive(Serialize, Deserialize)]
pub enum JumpPoint {
    File {
        file: PathBuf,
        cursors: Vec1<Cursor>,
        line_pos: f64,
        col_pos: f64,
    },
    FileExplorer(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buffer {
    pub path: PathBuf,
    pub cursors: Vec1<Cursor>,
    pub line_pos: usize,
    pub col_pos: usize,
    pub language: String,
    pub indent: Indentation,
}
