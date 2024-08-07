use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use ferrite_utility::graphemes::RopeGraphemeExt;
use serde::{Deserialize, Serialize};
use slotmap::{Key, SlotMap};

use super::buffer::{Buffer, Cursor};
use crate::panes::{PaneKind, Panes};

slotmap::new_key_type! {
    pub struct BufferId;
}

pub struct Workspace {
    pub buffers: SlotMap<BufferId, Buffer>,
    pub panes: Panes,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceData {
    buffers: Vec<BufferData>,
    current_buffer: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub struct BufferData {
    path: PathBuf,
    cursor: Cursor,
    line_pos: usize,
}

impl Default for Workspace {
    fn default() -> Self {
        let mut buffers: SlotMap<BufferId, _> = SlotMap::with_key();
        let buffer_id = buffers.insert(Buffer::new());
        Self {
            buffers,
            panes: Panes::new(buffer_id),
        }
    }
}

impl Workspace {
    pub fn save_workspace(&self) -> Result<()> {
        let workspace_file = get_workspace_path(std::env::current_dir()?)?;
        let mut workspace_data = WorkspaceData {
            buffers: Vec::new(),
            current_buffer: None,
        };

        if let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() {
            workspace_data.current_buffer = self.buffers[buffer_id].file().map(|p| p.to_path_buf());
        }

        for (path, buffer) in self
            .buffers
            .iter()
            .filter_map(|(_, buffer)| buffer.file().map(|path| (path, buffer)))
        {
            let language = &buffer.language_name();
            if language.starts_with("git-") && language != "git-config" {
                continue;
            }

            let buffer_data = BufferData {
                path: path.to_path_buf(),
                cursor: buffer.cursor(),
                line_pos: buffer.line_pos(),
            };
            workspace_data.buffers.push(buffer_data);
        }

        fs::create_dir_all(workspace_file.parent().unwrap())?;
        fs::write(
            &workspace_file,
            serde_json::to_string_pretty(&workspace_data)?.as_bytes(),
        )?;
        tracing::info!("Save workspace to: {workspace_file:?}");
        Ok(())
    }

    pub fn load_workspace() -> Result<Self> {
        let mut buffers: SlotMap<BufferId, _> = SlotMap::with_key();
        let mut panes = Panes::new(BufferId::null());

        let workspace_file = get_workspace_path(std::env::current_dir()?)?;
        let workspace: WorkspaceData = serde_json::from_str(&fs::read_to_string(workspace_file)?)?;
        for buffer_data in &workspace.buffers {
            tracing::info!("Loaded workspace buffer: {}", buffer_data.path.display());
            match Buffer::from_file(&buffer_data.path) {
                Ok(mut buffer) => {
                    let cursor = buffer_data.cursor;
                    let line_pos = buffer_data.line_pos;
                    buffer.vertical_scroll(line_pos as i64);
                    let postion = buffer
                        .rope()
                        .byte_to_point(cursor.position.min(buffer.len_bytes()));
                    let anchor = buffer
                        .rope()
                        .byte_to_point(cursor.anchor.min(buffer.len_bytes()));
                    buffer.set_cursor_pos(postion.column, postion.line);
                    buffer.set_anchor_pos(anchor.column, anchor.line);
                    buffer.ensure_cursor_is_valid();
                    buffers.insert(buffer);
                }
                Err(err) => tracing::error!("Error loading buffer: {}", &err),
            };
        }

        if let Some(current_buffer) = &workspace.current_buffer {
            for (buffer_id, buffer) in &buffers {
                if buffer.file().unwrap() == current_buffer {
                    panes.replace_current(PaneKind::Buffer(buffer_id));
                }
            }
        }

        if buffers.is_empty() {
            let buffer_id = buffers.insert(Buffer::new());
            panes.replace_current(PaneKind::Buffer(buffer_id));
        }

        if let PaneKind::Buffer(buffer_id) = panes.get_current_pane() {
            if buffers.get(buffer_id).is_none() {
                panes.replace_current(PaneKind::Buffer(buffers.keys().next().unwrap()));
            }
        }

        Ok(Self { buffers, panes })
    }
}

pub fn get_workspace_path(workspace_path: impl AsRef<Path>) -> Result<PathBuf> {
    let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
        return Err(anyhow::Error::msg("Unable to find project directory"));
    };
    let path = dunce::canonicalize(&workspace_path)?;
    let path = path.to_string_lossy();
    let hash = blake3::hash(path.as_bytes());
    let hex = hash.to_hex();
    Ok(directories.data_dir().join(format!(
        "ferrite-workspace-{}-{hex}.json",
        workspace_path
            .as_ref()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    )))
}
