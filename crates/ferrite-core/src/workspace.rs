use std::{
    collections::HashMap,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use ferrite_utility::vec1::Vec1;
use serde::{Deserialize, Serialize};
use slotmap::{Key, SlotMap};

use super::buffer::Buffer;
use crate::{
    buffer::{ViewId, cursor::Cursor},
    event_loop_proxy::EventLoopProxy,
    file_explorer::{FileExplorer, FileExplorerId},
    indent::Indentation,
    layout::panes::{PaneKind, Panes, layout::Layout},
    watcher::{FileWatcher, TomlConfig},
};

slotmap::new_key_type! {
    pub struct BufferId;
}

#[derive(Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub actions: HashMap<String, Vec1<String>>,
}

impl WorkspaceConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config_path = get_config_path(path);
        let string = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&string)?)
    }
}

pub struct Workspace {
    pub buffers: SlotMap<BufferId, Buffer>,
    pub file_explorers: SlotMap<FileExplorerId, FileExplorer>,
    pub buffer_extra_data: Vec<BufferData>,
    pub panes: Panes,
    pub config: WorkspaceConfig,
    pub config_watcher: Option<FileWatcher<WorkspaceConfig, TomlConfig>>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceData {
    buffers: Vec<BufferData>,
    open_buffers: Vec<PathBuf>,
    layout: Layout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferData {
    pub path: PathBuf,
    pub cursors: Vec1<Cursor>,
    pub line_pos: usize,
    pub col_pos: usize,
    pub language: String,
    pub indent: Indentation,
}

impl Default for Workspace {
    fn default() -> Self {
        let mut buffers: SlotMap<BufferId, _> = SlotMap::with_key();
        let mut buffer = Buffer::new();
        let view_id = buffer.create_view();
        let buffer_id = buffers.insert(buffer);
        Self {
            buffers,
            file_explorers: SlotMap::with_key(),
            buffer_extra_data: Vec::new(),
            panes: Panes::new(buffer_id, view_id),
            config: WorkspaceConfig::default(),
            config_watcher: None,
        }
    }
}

impl Workspace {
    pub fn save_workspace(&self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let workspace_file = get_workspace_path(workspace_dir)?;
        let mut workspace_data = WorkspaceData {
            buffers: self.buffer_extra_data.clone(),
            open_buffers: Vec::new(),
            layout: Layout::from_panes(&self.panes, &self.buffers, &self.file_explorers),
        };

        for (path, buffer) in self
            .buffers
            .iter()
            .filter_map(|(_, buffer)| buffer.file().map(|path| (path, buffer)))
        {
            let language = &buffer.language_name();
            if language.starts_with("git-") && *language != "git-config" {
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

    pub fn load_workspace(load_buffers: bool, proxy: Box<dyn EventLoopProxy>) -> Result<Self> {
        let mut buffers: SlotMap<BufferId, Buffer> = SlotMap::with_key();
        let mut file_explorers: SlotMap<FileExplorerId, FileExplorer> = SlotMap::with_key();

        let workspace_dir = std::env::current_dir()?;
        let workspace_file = get_workspace_path(&workspace_dir)?;
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
                match Buffer::builder().from_file(path).build() {
                    Ok(mut buffer) => {
                        let buffer_data = workspace
                            .buffers
                            .iter()
                            .find(|buffer_data| buffer.file() == Some(&buffer_data.path));
                        if let Some(buffer_data) = buffer_data {
                            buffer.load_buffer_data(buffer_data);
                        }
                        buffers.insert(buffer);
                    }
                    Err(err) => tracing::error!("Error loading buffer: {}", &err),
                };
            }
        }

        let mut panes = workspace
            .layout
            .to_panes(&mut buffers, &mut file_explorers)
            .unwrap_or_else(|| Panes::new(BufferId::null(), ViewId::null()));

        if buffers.is_empty() {
            let mut buffer = Buffer::new();
            let view_id = buffer.create_view();
            let buffer_id = buffers.insert(buffer);
            panes.replace_current(PaneKind::Buffer(buffer_id, view_id));
        }

        if let PaneKind::Buffer(buffer_id, _) = panes.get_current_pane() {
            if buffers.get(buffer_id).is_none() {
                let (buffer_id, buffer) = buffers.iter_mut().next().unwrap();
                let view_id = buffer.create_view();
                panes.replace_current(PaneKind::Buffer(buffer_id, view_id));
            }
        }

        panes.ensure_current_pane_exists();
        for buffer in buffers.values_mut() {
            buffer.ensure_every_cursor_is_valid();
        }

        let config = WorkspaceConfig::load(&workspace_dir).unwrap_or_else(|err| {
            tracing::error!("Error loading workspace config: {err}");
            WorkspaceConfig::default()
        });

        let mut config_watcher = None;
        match FileWatcher::new(get_config_path(&workspace_dir), proxy.dup()) {
            Ok(watcher) => config_watcher = Some(watcher),
            Err(err) => tracing::error!("Error starting language config watcher: {err}"),
        }

        Ok(Self {
            buffers,
            file_explorers,
            buffer_extra_data: workspace.buffers.clone(),
            panes,
            config,
            config_watcher,
        })
    }
}

pub fn get_workspace_path(workspace_path: impl AsRef<Path>) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
        return Err(anyhow::Error::msg("Unable to find project directory"));
    };
    let path = dunce::canonicalize(&workspace_path)?;
    let path = path.to_string_lossy();
    let hash = Sha256::digest(path.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in hash {
        write!(&mut hex, "{:x}", byte)?;
    }
    Ok(directories.data_dir().join(format!(
        "ferrite-workspace-{}-{hex}.json",
        workspace_path
            .as_ref()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    )))
}

pub fn get_config_path(workspace_path: impl AsRef<Path>) -> PathBuf {
    workspace_path.as_ref().join(".editor/ferrite/config.toml")
}
