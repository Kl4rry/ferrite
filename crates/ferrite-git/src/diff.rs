use std::{
    fmt,
    fmt::Display,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use rand::RngExt;
use ropey::{Rope, RopeSlice};

struct RopeLineTokenSource<'a>(RopeSlice<'a>);

impl<'a> imara_diff::TokenSource for RopeLineTokenSource<'a> {
    type Token = RopeSlice<'a>;
    type Tokenizer = ropey::iter::Lines<'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        self.0.lines()
    }

    fn estimate_tokens(&self) -> u32 {
        self.0.len_lines().try_into().unwrap()
    }
}

pub fn get_path_in_repo(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let Some(repo_dir) = crate::repo::get_repo_directory() else {
        return Err(std::io::Error::other("no git repo found"));
    };
    // TODO: this will not work if the name of the file is not valid utf-8
    let path = path.as_ref().to_string_lossy();
    Ok(path
        .trim_start_matches(&format!("{}/", repo_dir))
        .to_string())
}

#[profiling::function]
pub fn get_diff_base(path: impl AsRef<Path>) -> Result<Rope, std::io::Error> {
    let path = get_path_in_repo(&path)?;

    let output = Command::new("git")
        .args(["show", "--textconv", &format!(":{path}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(String::from_utf8_lossy(
            &output.stderr,
        )));
    }
    let (_encoding, rope) = ferrite_utility::read::read(&*output.stdout)?;
    Ok(rope)
}

#[profiling::function]
pub fn line_diff(before: Rope, after: Rope) -> imara_diff::Diff {
    let input = imara_diff::InternedInput::new(
        RopeLineTokenSource(before.slice(..)),
        RopeLineTokenSource(after.slice(..)),
    );
    let mut diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
    diff.postprocess_with(
        &input.before,
        &input.after,
        imara_diff::IndentHeuristic::new(|token| {
            imara_diff::IndentLevel::for_ascii_line(input.interner[token].bytes(), 4)
        }),
    );

    diff
}

// creates a patch from a line range and a file
pub fn patch_from_line_range(
    after: Rope,
    path: impl AsRef<Path>,
    line_range: std::range::Range<usize>,
) -> Result<String, std::io::Error> {
    let path = get_path_in_repo(&path)?;
    let before = get_diff_base(&path)?;

    let input = imara_diff::InternedInput::new(
        RopeLineTokenSource(before.slice(..)),
        RopeLineTokenSource(after.slice(..)),
    );
    let mut diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
    diff.postprocess_with(
        &input.before,
        &input.after,
        imara_diff::IndentHeuristic::new(|token| {
            imara_diff::IndentLevel::for_ascii_line(input.interner[token].bytes(), 4)
        }),
    );

    let printer = RopeLineDiffPrinter(&input.interner);
    let partial_patch = PartialPatch::from_diff(
        &diff,
        &printer,
        &input.before,
        &input.after,
        ((line_range.start as u32)..(line_range.end as u32)).into(),
    );
    let patch = format!("--- a/{0}\n+++ b/{0}\n{1}", path, partial_patch);

    Ok(patch)
}

pub fn stage_line_range(
    after: Rope,
    path: impl AsRef<Path>,
    line_range: std::range::Range<usize>,
) -> Result<(), std::io::Error> {
    let patch = patch_from_line_range(after, path, line_range)?;

    let mut tmp_file_name: String = ".".into();
    tmp_file_name.extend(
        rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(20)
            .map(char::from),
    );

    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&tmp_file_name)?;
    if let error @ Err(_) = file.write_all(patch.as_bytes()) {
        fs::remove_file(&tmp_file_name)?;
        error?;
    }

    let result = apply_patch_to_index(&tmp_file_name);
    fs::remove_file(&tmp_file_name)?;
    result
}

fn apply_patch_to_index(tmp_file_name: &str) -> Result<(), std::io::Error> {
    let output = Command::new("git")
        .args(["apply", "--cached", &tmp_file_name])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(String::from_utf8_lossy(
            &output.stderr,
        )));
    }
    Ok(())
}

pub struct RopeLineDiffPrinter<'a>(&'a imara_diff::Interner<ropey::RopeSlice<'a>>);

impl imara_diff::UnifiedDiffPrinter for RopeLineDiffPrinter<'_> {
    fn display_header(
        &self,
        mut f: impl fmt::Write,
        start_before: u32,
        start_after: u32,
        len_before: u32,
        len_after: u32,
    ) -> fmt::Result {
        writeln!(
            f,
            "@@ -{},{} +{},{} @@",
            start_before + 1,
            len_before,
            start_after + 1,
            len_after
        )
    }

    fn display_context_token(
        &self,
        mut f: impl fmt::Write,
        token: imara_diff::Token,
    ) -> fmt::Result {
        write!(f, " {}", self.0[token])?;
        if !&self.0[token].ends_with_newline() {
            writeln!(f)?;
        }
        Ok(())
    }

    fn display_hunk(
        &self,
        mut f: impl fmt::Write,
        before: &[imara_diff::Token],
        after: &[imara_diff::Token],
    ) -> fmt::Result {
        if let Some(&last) = before.last() {
            for &token in before {
                let token = self.0[token];
                write!(f, "-{token}")?;
            }
            if !self.0[last].ends_with_newline() {
                writeln!(f)?;
            }
        }
        if let Some(&last) = after.last() {
            for &token in after {
                let token = self.0[token];
                write!(f, "+{token}")?;
            }
            if !self.0[last].ends_with_newline() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

pub struct PartialPatch<'a, P: imara_diff::UnifiedDiffPrinter> {
    printer: &'a P,
    diff: &'a imara_diff::Diff,
    before: &'a [imara_diff::Token],
    after: &'a [imara_diff::Token],
    line_range: std::range::Range<u32>,
}

impl<'a, P: imara_diff::UnifiedDiffPrinter> PartialPatch<'a, P> {
    fn from_diff(
        diff: &'a imara_diff::Diff,
        printer: &'a P,
        before: &'a [imara_diff::Token],
        after: &'a [imara_diff::Token],
        line_range: std::range::Range<u32>,
    ) -> Self {
        Self {
            diff,
            printer,
            before,
            after,
            line_range,
        }
    }
}

impl<P: imara_diff::UnifiedDiffPrinter> Display for PartialPatch<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const CONTEXT_LEN: u32 = 1;

        let mut pos = 0;
        let mut before_context_len = 0;
        let mut after_context_len = 0;
        let first_hunk = self.diff.hunks().next().unwrap_or_default();
        let mut before_context_start = first_hunk.before.start.saturating_sub(CONTEXT_LEN);
        let mut after_context_start = first_hunk.after.start.saturating_sub(CONTEXT_LEN);
        let mut buffer = String::new();
        for hunk in self.diff.hunks() {
            if !intersects(
                hunk.after.start,
                hunk.after.end.saturating_sub(1).max(hunk.after.start),
                self.line_range.start,
                self.line_range.end,
            ) {
                continue;
            }
            if hunk.before.start - pos > 2 * CONTEXT_LEN {
                if !buffer.is_empty() {
                    let end = (pos + CONTEXT_LEN).min(self.before.len() as u32);
                    self.printer.display_header(
                        &mut *f,
                        before_context_start,
                        after_context_start,
                        before_context_len + end - pos,
                        after_context_len + end - pos,
                    )?;
                    write!(f, "{buffer}")?;
                    for &token in &self.before[pos as usize..end as usize] {
                        self.printer.display_context_token(&mut *f, token)?;
                    }
                    buffer.clear();
                }
                pos = hunk.before.start - CONTEXT_LEN;
                before_context_start = pos;
                after_context_start = hunk.after.start - CONTEXT_LEN;
                before_context_len = 0;
                after_context_len = 0;
            }
            for &token in &self.before[pos as usize..hunk.before.start as usize] {
                self.printer.display_context_token(&mut buffer, token)?;
            }
            let context_len = hunk.before.start - pos;
            before_context_len += hunk.before.len() as u32 + context_len;
            after_context_len += hunk.after.len() as u32 + context_len;
            self.printer.display_hunk(
                &mut buffer,
                &self.before[hunk.before.start as usize..hunk.before.end as usize],
                &self.after[hunk.after.start as usize..hunk.after.end as usize],
            )?;
            pos = hunk.before.end;
        }
        if !buffer.is_empty() {
            let end = (pos + CONTEXT_LEN).min(self.before.len() as u32);
            self.printer.display_header(
                &mut *f,
                before_context_start,
                after_context_start,
                before_context_len + end - pos,
                after_context_len + end - pos,
            )?;
            write!(f, "{buffer}")?;
            for &token in &self.before[pos as usize..end as usize] {
                self.printer.display_context_token(&mut *f, token)?;
            }
            buffer.clear();
        }
        Ok(())
    }
}

pub trait EndsWithNewline {
    fn ends_with_newline(&self) -> bool;
}

impl EndsWithNewline for ropey::RopeSlice<'_> {
    fn ends_with_newline(&self) -> bool {
        self.bytes().last().map(|ch| ch == b'\n').unwrap_or(false)
    }
}

fn intersects(start1: u32, end1: u32, start2: u32, end2: u32) -> bool {
    !(start1 > end2 || end1 < start2)
}
