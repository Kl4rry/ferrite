use std::process::Command;

#[profiling::function]
pub fn get_current_branch() -> Option<String> {
    match Command::new("git")
        .args(["branch", "--show-current"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().into())
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}

#[profiling::function]
pub fn get_current_head() -> Option<String> {
    match Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().into())
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}

#[profiling::function]
pub fn get_git_directory() -> Option<String> {
    match Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().into())
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}

#[profiling::function]
pub fn get_repo_directory() -> Option<String> {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().into())
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}
