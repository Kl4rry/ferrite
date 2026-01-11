use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime},
};

use anyhow::Result;

// modified version of: https://github.com/mitsu-ksgr/git-blame-parser

#[derive(Debug)]
pub struct BlameHunk {
    pub commit: String,
    pub original_line_no: usize,
    pub final_line_no: usize,

    pub filename: String,
    pub summary: String,

    /// The contents of the actual line
    pub content: String,

    // previous
    pub previous_commit: Option<String>,
    pub previous_filepath: Option<String>,

    /// Set to true when blame output contains `boundary`.
    pub boundary: bool,

    pub author: String,
    pub author_mail: String,
    pub author_time: SystemTime,
    pub author_tz: String,

    pub committer: String,
    pub committer_mail: String,
    pub committer_time: SystemTime,
    pub committer_tz: String,

    pub start_line: usize,
    pub len_lines: usize,
}

impl Default for BlameHunk {
    fn default() -> Self {
        Self {
            commit: String::new(),
            original_line_no: 0,
            final_line_no: 0,
            filename: String::new(),
            summary: String::new(),
            content: String::new(),
            previous_commit: None,
            previous_filepath: None,
            boundary: false,
            author: String::new(),
            author_mail: String::new(),
            author_time: SystemTime::UNIX_EPOCH,
            author_tz: String::new(),
            committer: String::new(),
            committer_mail: String::new(),
            committer_time: SystemTime::UNIX_EPOCH,
            committer_tz: String::new(),
            start_line: 0,
            len_lines: 0,
        }
    }
}

fn parse_one_blame(porcelain: &[&str]) -> Result<BlameHunk> {
    let mut blame = BlameHunk::default();

    // Parse header
    if let Some(header) = porcelain.first() {
        let parts: Vec<&str> = header.split_whitespace().collect();
        blame.commit = parts[0].to_string();

        if let Some(lineno) = parts.get(1) {
            blame.original_line_no = lineno.parse::<usize>().unwrap_or(0);
        }
        if let Some(lineno) = parts.get(2) {
            blame.final_line_no = lineno.parse::<usize>().unwrap_or(0);
        }
    } else {
        anyhow::bail!("blame parse error: no header");
    }

    // Parse details
    for line in porcelain.iter().skip(1) {
        if line.starts_with('\t') {
            let src = line.strip_prefix('\t').unwrap_or(line);
            blame.content = src.to_string();
        } else {
            match line.split_once(' ') {
                Some(("filename", value)) => blame.filename = value.to_string(),
                Some(("summary", value)) => blame.summary = value.to_string(),

                Some(("author", value)) => blame.author = value.to_string(),
                Some(("author-mail", value)) => blame.author_mail = value.to_string(),
                Some(("author-time", value)) => {
                    blame.author_time = epoch_to_system_time(value.parse::<u64>().unwrap_or(0))
                }
                Some(("author-tz", value)) => blame.author_tz = value.to_string(),

                Some(("committer", value)) => blame.committer = value.to_string(),
                Some(("committer-mail", value)) => blame.committer_mail = value.to_string(),
                Some(("committer-time", value)) => {
                    blame.committer_time = epoch_to_system_time(value.parse::<u64>().unwrap_or(0))
                }
                Some(("committer-tz", value)) => blame.committer_tz = value.to_string(),

                Some(("previous", value)) => {
                    if let Some((commit, filepath)) = value.split_once(' ') {
                        blame.previous_commit = Some(commit.to_string());
                        blame.previous_filepath = Some(filepath.to_string());
                    }
                }

                None => match *line {
                    "boundary" => blame.boundary = true,
                    _ => continue,
                },

                _ => continue,
            }
        }
    }

    Ok(blame)
}

fn parse(porcelain: &str) -> Result<Vec<BlameHunk>> {
    let lines = porcelain.lines();
    let mut blames: Vec<BlameHunk> = Vec::new();

    let mut i = 0;
    let mut blob: Vec<&str> = Vec::new();
    for line in lines {
        blob.push(line);

        // end of one blame output.
        if line.starts_with('\t') {
            let mut blame = parse_one_blame(&blob)?;
            /*if let Some(last) = blames.last_mut()
                && blame.commit == last.commit
            {
                blame.len_lines += 1;
            } else {*/
            blame.start_line = i;
            blame.len_lines = 1;
            blames.push(blame);
            //}
            blob.clear();
            i += 1;
        }
    }

    Ok(blames)
}

#[rustfmt::skip]
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

    parse(&stdout)
}

fn epoch_to_system_time(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blame() {
        blame("/home/axel/projects/ferrite/src/main.rs").unwrap();
    }
}
