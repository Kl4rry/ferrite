use std::{
    path::Path,
    process::{Command, Stdio},
};

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

#[profiling::function]
pub fn get_diff_base(path: impl AsRef<Path>) -> Result<Rope, std::io::Error> {
    let Some(repo_dir) = crate::repo::get_repo_directory() else {
        return Err(std::io::Error::other("no git repo found"));
    };
    // TODO: this will not work if the name of the file is not valid utf-8
    let path = path.as_ref().to_string_lossy();
    let path = path.trim_start_matches(&format!("{}/", repo_dir));
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
