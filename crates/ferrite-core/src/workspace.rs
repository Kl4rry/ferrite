use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use slotmap::{Key, SlotMap};

use super::buffer::{Buffer, Cursor};
use crate::panes::{PaneKind, Panes};

slotmap::new_key_type! {
    pub struct BufferId;
}

pub struct Workspace {
    pub buffers: SlotMap<BufferId, Buffer>,
    pub buffer_extra_data: Vec<BufferData>,
    pub panes: Panes,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceData {
    buffers: Vec<BufferData>,
    open_buffers: Vec<PathBuf>,
    current_buffer: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferData {
    pub path: PathBuf,
    pub cursor: Cursor,
    pub line_pos: usize,
}

impl Default for Workspace {
    fn default() -> Self {
        let mut buffers: SlotMap<BufferId, _> = SlotMap::with_key();
        let buffer_id = buffers.insert(Buffer::new());
        Self {
            buffers,
            buffer_extra_data: Vec::new(),
            panes: Panes::new(buffer_id),
        }
    }
}

impl Workspace {
    pub fn save_workspace(&self) -> Result<()> {
        let workspace_file = get_workspace_path(std::env::current_dir()?)?;
        let mut workspace_data = WorkspaceData {
            buffers: self.buffer_extra_data.clone(),
            open_buffers: Vec::new(),
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

            workspace_data.open_buffers.push(path.to_path_buf());
        }

        fs::create_dir_all(workspace_file.parent().unwrap())?;
        fs::write(
            &workspace_file,
            serde_json::to_string_pretty(&workspace_data)?.as_bytes(),
        )?;
        tracing::info!("Save workspace to: {workspace_file:?}");
        Ok(())
    }

    pub fn load_workspace(load_buffers: bool) -> Result<Self> {
        let mut buffers: SlotMap<BufferId, Buffer> = SlotMap::with_key();
        let mut panes = Panes::new(BufferId::null());

        let workspace_file = get_workspace_path(std::env::current_dir()?)?;
        let workspace: WorkspaceData = serde_json::from_str(&fs::read_to_string(workspace_file)?)?;

        if load_buffers {
            for path in &workspace.open_buffers {
                let Ok(path) = dunce::canonicalize(path) else {
                    continue;
                };
                // Avoid loading the same buffer twice as everthing assumes that buffers are unique
                if buffers
                    .iter()
                    .any(|(_, buffer)| buffer.file() == Some(&path))
                {
                    continue;
                }
                tracing::info!("Loaded workspace buffer: {}", path.display());
                match Buffer::from_file(path) {
                    Ok(buffer) => {
                        buffers.insert(buffer);
                    }
                    Err(err) => tracing::error!("Error loading buffer: {}", &err),
                };
            }
        }

        for buffer_data in &workspace.buffers {
            for (_, buffer) in &mut buffers {
                if let Some(path) = buffer.file() {
                    if buffer_data.path == path {
                        buffer.load_buffer_data(buffer_data);
                    }
                }
            }
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

        Ok(Self {
            buffers,
            buffer_extra_data: workspace.buffers.clone(),
            panes,
        })
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
