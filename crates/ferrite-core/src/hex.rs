use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::Result;
use ferrite_runtime::unique_id::UniqueId;
use slotmap::SlotMap;

mod input;

slotmap::new_key_type! {
    pub struct HexViewId;
}

#[derive(Default)]
pub struct HexView {
    pub line_pos: f64,
    unique_id: UniqueId,
}

impl HexView {
    pub fn unique_id(&self) -> UniqueId {
        self.unique_id
    }
}

pub struct Hex {
    // TODO: switch to sumtree for editing
    pub bytes: Vec<u8>,
    name: String,
    file: Option<PathBuf>,
    pub views: SlotMap<HexViewId, HexView>,
}

impl Hex {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = dunce::canonicalize(&path)?;
        let bytes = std::fs::read(&path)?;

        fn get_file_name(path: &Path) -> Result<String, io::Error> {
            let Some(name) = path.file_name() else {
                return Err(io::Error::other("path has no filename name"));
            };
            Ok(name.to_string_lossy().into())
        }

        Ok(Self {
            bytes,
            name: get_file_name(&path)?,
            file: Some(path),
            views: SlotMap::default(),
        })
    }

    pub fn vertical_scroll(&mut self, view_id: HexViewId, distance: f64) {
        self.views[view_id].line_pos += distance;
        self.views[view_id].line_pos = self.views[view_id]
            .line_pos
            .max(0.0)
            .min((self.bytes.len() / 0x10) as f64);
    }

    pub fn len_lines(&self) -> usize {
        self.bytes.len() / 0x10
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

// View related functions
impl Hex {
    pub fn create_view(&mut self) -> HexViewId {
        self.views.insert(HexView::default())
    }

    pub fn get_first_view(&self) -> Option<HexViewId> {
        self.views.keys().next()
    }

    pub fn get_first_view_or_create(&mut self) -> HexViewId {
        self.views
            .keys()
            .next()
            .unwrap_or_else(|| self.create_view())
    }

    pub fn remove_view(&mut self, view_id: HexViewId) {
        self.views.remove(view_id);
    }
}
