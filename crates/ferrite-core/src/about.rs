pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn git_hash() -> &'static str {
    env!("GIT_HASH")
}

pub fn git_hash_short() -> &'static str {
    let long = git_hash();
    &long[..(long.len().min(7))]
}
