use std::time::Duration;

use ropey::Rope;
use subprocess::{Exec, PopenError, Redirection};

use super::Buffer;

fn format_rope(formatter: &str, rope: Rope) -> Result<String, PopenError> {
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

impl Buffer {
    pub fn format(&mut self, formatter: &str) -> Result<(), PopenError> {
        if self.rope.len_bytes() == 0 {
            return Ok(());
        }

        self.history.begin(self.cursor, self.dirty);
        let new_rope = format_rope(formatter, self.rope.clone())?;

        let len = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len, &new_rope);

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }

        self.mark_dirty();
        self.history.finish();
        Ok(())
    }
}
