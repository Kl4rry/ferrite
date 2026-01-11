use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime},
};

use anyhow::Result;
use regex::{Match, Regex};

#[derive(Debug)]
pub struct BlameHunk {
    pub commit: String,
    pub author: String,
    pub author_mail: String,
    pub author_time: SystemTime,
    pub author_tz: String,
    pub committer: String,
    pub committer_mail: String,
    pub committer_time: u64,
    pub committer_tz: String,
    pub summary: String,
    pub previous: String,
    pub filename: String,
    pub line: String,
    pub start_line: usize,
    pub len_lines: usize,
}

pub fn blame(path: impl AsRef<Path>) -> Result<Vec<BlameHunk>> {
    let mut cmd = Command::new("git");
    cmd.arg("blame");
    cmd.arg("--line-porcelain");
    cmd.arg(path.as_ref());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!(format!("{:?}", stderr));
    }

    let regex = Regex::new(
        r##"(\w+) .+\nauthor (.+)\nauthor-mail <(.+)>\nauthor-time (\d+)\nauthor-tz (.+)\ncommitter (.+)\ncommitter-mail <(.+)>\ncommitter-time (\d+)\ncommitter-tz (.+)\nsummary (.+)\nprevious (.+)\nfilename (.+)\n(.+)\n+"##,
    )?;

    let iter = regex.captures_iter(&stdout);
    let mut hunks: Vec<BlameHunk> = Vec::new();
    for (i, capture) in iter.enumerate() {
        let commit = err("commit", capture.get(1))?.as_str();
        if let Some(last) = hunks.last_mut()
            && last.commit == commit
        {
            last.len_lines += 1;
            continue;
        }

        let blame_hunk = BlameHunk {
            commit: commit.to_string(),
            author: err("author", capture.get(2))?.as_str().to_string(),
            author_mail: err("author_mail", capture.get(3))?.as_str().to_string(),
            author_time: epoch_to_system_time(
                err("author_time", capture.get(4))?.as_str().parse()?,
            ),
            author_tz: err("author_tz", capture.get(5))?.as_str().to_string(),
            committer: err("committer", capture.get(6))?.as_str().to_string(),
            committer_mail: err("committer_mail", capture.get(7))?.as_str().to_string(),
            committer_time: err("committer_time", capture.get(8))?.as_str().parse()?,
            committer_tz: err("committer_tz", capture.get(9))?.as_str().to_string(),
            summary: err("summary", capture.get(10))?.as_str().to_string(),
            previous: err("previous", capture.get(11))?.as_str().to_string(),
            filename: err("filename", capture.get(12))?.as_str().to_string(),
            line: err("line", capture.get(13))?.as_str().to_string(),
            start_line: i,
            len_lines: 1,
        };
        hunks.push(blame_hunk);
    }

    Ok(hunks)
}

fn epoch_to_system_time(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn err<'a>(field_name: &'static str, m: Option<Match<'a>>) -> Result<Match<'a>> {
    if let Some(m) = m {
        Ok(m)
    } else {
        anyhow::bail!(format!("Field {field_name} missing in blame output"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blame() {
        blame("/home/axel/projects/ferrite/src/main.rs").unwrap();
    }
}
