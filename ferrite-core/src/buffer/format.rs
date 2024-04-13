use std::time::Duration;

use ropey::Rope;
use subprocess::{Exec, PopenError, Redirection};
use utility::graphemes::ensure_grapheme_boundary_next_byte;

use super::{Buffer, Cursor};

fn format(formatter: &str, rope: Rope) -> Result<String, PopenError> {
    let mut child = Exec::cmd(formatter)
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
        .limit_time(Duration::from_secs(1));
    let (stdout, stderr) = com.read()?;
    let exit_status = child.wait()?;

    if exit_status.success() {
        Ok(String::from_utf8_lossy(&stdout.unwrap()).into())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            String::from_utf8_lossy(&stderr.unwrap()),
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
        .limit_time(Duration::from_secs(1));
    let (stdout, stderr) = com.read()?;
    let exit_status = child.wait()?;

    if exit_status.success() {
        Ok(String::from_utf8_lossy(&stdout.unwrap()).into())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            String::from_utf8_lossy(&stderr.unwrap()),
        ))?
    }
}

impl Buffer {
    pub fn format(&mut self, formatter: &str) -> Result<(), PopenError> {
        if self.rope.len_bytes() == 0 {
            return Ok(());
        }

        self.history.begin(self.cursor, self.dirty);
        let new_rope = format(formatter, self.rope.clone())?;

        let len = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len, &new_rope);

        let pos = ensure_grapheme_boundary_next_byte(
            self.rope.slice(..),
            self.cursor.position.min(self.rope.len_bytes()),
        );

        self.cursor.position = pos;
        self.cursor.anchor = pos;

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }

        self.mark_dirty();
        self.history.finish();
        Ok(())
    }

    pub fn format_selection(&mut self, formatter: &str) -> Result<(), PopenError> {
        if self.rope.len_bytes() == 0 {
            return Ok(());
        }

        self.history.begin(self.cursor, self.dirty);
        let new_rope = format_selection(formatter, self.rope.clone(), &self.cursor)?;

        let len = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len, &new_rope);

        let pos = ensure_grapheme_boundary_next_byte(
            self.rope.slice(..),
            self.cursor.position.min(self.rope.len_bytes()),
        );

        self.cursor.position = pos;
        self.cursor.anchor = pos;

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }

        self.mark_dirty();
        self.history.finish();
        Ok(())
    }
}
