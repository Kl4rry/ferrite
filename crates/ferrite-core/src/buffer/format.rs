use std::time::Duration;

use ropey::Rope;
use slotmap::Key;
use subprocess::{Exec, PopenError, Redirection};

use super::{Buffer, Cursor, ViewId};

fn format(formatter: &str, rope: Rope) -> Result<String, PopenError> {
    let mut parts = formatter.split_whitespace();
    let Some(first) = parts.next() else {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid formatter").into(),
        );
    };

    let mut child = Exec::cmd(first)
        .args(&parts.collect::<Vec<_>>())
        .stdin(Redirection::Pipe)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .popen()?;

    let mut input = Vec::new();
    for chunk in rope.chunks() {
        input.extend_from_slice(chunk.as_bytes());
    }

    let mut com = child
        .communicate_start(Some(input))
        .limit_time(Duration::from_secs(3));
    let (stdout, stderr) = com.read()?;
    let exit_status = child.wait()?;

    let stdout_output: String = String::from_utf8_lossy(&stdout.unwrap()).into();
    let stderr_output: String = String::from_utf8_lossy(&stderr.unwrap()).into();
    if exit_status.success() && stderr_output.is_empty() {
        Ok(stdout_output)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            stderr_output,
        ))?
    }
}

fn format_selection(formatter: &str, rope: Rope, cursor: &Cursor) -> Result<String, PopenError> {
    let mut parts = formatter.split_whitespace();
    let Some(first) = parts.next() else {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid formatter").into(),
        );
    };

    let mut cmd = Exec::cmd(first);
    let start = cursor.position.min(cursor.anchor);
    let end = cursor.position.max(cursor.anchor);
    let len = end - start;

    let start = start.to_string();
    let len = len.to_string();
    let end = end.to_string();

    for part in parts {
        let arg = part
            .replace("%start%", &start)
            .replace("%len%", &len)
            .replace("%end%", &end);
        cmd = cmd.arg(arg);
    }

    let mut child = cmd
        .stdin(Redirection::Pipe)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Pipe)
        .popen()?;

    let mut input = Vec::new();
    for chunk in rope.chunks() {
        input.extend_from_slice(chunk.as_bytes());
    }

    let mut com = child
        .communicate_start(Some(input))
        .limit_time(Duration::from_secs(3));
    let (stdout, stderr) = com.read()?;
    let exit_status = child.wait()?;

    let stdout_output: String = String::from_utf8_lossy(&stdout.unwrap()).into();
    let stderr_output: String = String::from_utf8_lossy(&stderr.unwrap()).into();
    if exit_status.success() && stderr_output.is_empty() {
        Ok(stdout_output)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            stderr_output,
        ))?
    }
}

impl Buffer {
    pub fn format(&mut self, view_id: Option<ViewId>, formatter: &str) -> Result<(), PopenError> {
        let view_id = view_id.unwrap_or(ViewId::null());
        if self.read_only {
            return Ok(());
        }

        if self.rope.len_bytes() == 0 {
            return Ok(());
        }

        self.history
            .begin(view_id, self.get_all_cursors(), self.dirty);
        let new_rope = format(formatter, self.rope.clone())?;

        let cursor_positions = self.get_cursor_positions();

        let len = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len, &new_rope);

        self.restore_cursor_positions(cursor_positions);

        self.mark_dirty();
        self.update_searchers();

        self.history.finish();
        self.on_file_changed(None);
        Ok(())
    }

    pub fn format_selection(&mut self, view_id: ViewId, formatter: &str) -> Result<(), PopenError> {
        if self.read_only {
            return Ok(());
        }

        if self.rope.len_bytes() == 0 {
            return Ok(());
        }

        self.history
            .begin(view_id, self.get_all_cursors(), self.dirty);
        let new_rope = format_selection(
            formatter,
            self.rope.clone(),
            self.views[view_id].cursors.first(),
        )?;

        let cursor_positions = self.get_cursor_positions();

        let len = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len, &new_rope);

        self.restore_cursor_positions(cursor_positions);

        if self.views[view_id].clamp_cursor {
            self.center_on_main_cursor(view_id);
        }

        self.mark_dirty();
        self.update_searchers();

        self.history.finish();
        self.on_file_changed(Some(view_id));
        Ok(())
    }
}
