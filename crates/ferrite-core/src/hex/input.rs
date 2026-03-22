use anyhow::Result;

use crate::{
    cmd::Cmd,
    hex::{Hex, HexViewId},
};

impl Hex {
    pub fn handle_input(&mut self, view_id: HexViewId, input: Cmd) -> Result<()> {
        use Cmd::*;
        match input {
            VerticalScroll { distance } => self.vertical_scroll(view_id, distance),
            _ => return Ok(()),
        }

        Ok(())
    }
}
