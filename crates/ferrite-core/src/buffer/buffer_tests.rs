use std::fs;

use ferrite_utility::line_ending::DEFAULT_LINE_ENDING;
use tempdir::TempDir;

use super::{read, write};

#[test]
fn read_test_utf8() {
    const TEST_FILE: &'static str = "../../test_files/emoji-utf8.json";
    let (_, rope) = read::read_from_file(TEST_FILE).unwrap();
    let decoded = rope.to_string();
    let reference = fs::read_to_string(TEST_FILE).unwrap();

    assert_eq!(decoded.len(), reference.len());
    assert_eq!(decoded, reference);
}

#[test]
fn read_write_test_utf8() {
    const TEST_FILE: &'static str = "../../test_files/emoji-utf8.json";
    let (encoding, rope) = read::read_from_file(TEST_FILE).unwrap();
    let tmp_dir = TempDir::new("test").unwrap();
    let output_path = tmp_dir.path().join("output.json");
    write::write(encoding, DEFAULT_LINE_ENDING, rope.clone(), &output_path).unwrap();

    let written = fs::read_to_string(&output_path).unwrap();
    assert_eq!(written, rope.to_string());
    tmp_dir.close().unwrap();
}
