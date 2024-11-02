use std::fs;

use ferrite_utility::{line_ending::DEFAULT_LINE_ENDING, vec1::Vec1};
use tempdir::TempDir;

use super::{read, write};
use crate::buffer::{Buffer, Cursor, View};

#[test]
fn read_utf8() {
    const TEST_FILE: &'static str = "../../test_files/emoji-utf8.json";
    let (_, rope) = read::read_from_file(TEST_FILE).unwrap();
    let decoded = rope.to_string();
    let reference = fs::read_to_string(TEST_FILE).unwrap();

    assert_eq!(decoded.len(), reference.len());
    assert_eq!(decoded, reference);
}

#[test]
fn read_write_utf8() {
    const TEST_FILE: &'static str = "../../test_files/emoji-utf8.json";
    let (encoding, rope) = read::read_from_file(TEST_FILE).unwrap();
    let tmp_dir = TempDir::new("test").unwrap();
    let output_path = tmp_dir.path().join("output.json");
    write::write(encoding, DEFAULT_LINE_ENDING, rope.clone(), &output_path).unwrap();

    let written = fs::read_to_string(&output_path).unwrap();
    assert_eq!(written, rope.to_string());
    tmp_dir.close().unwrap();
}

#[test]
fn insert_random_ascii() {
    for _ in 0..100 {
        use rand::Rng;
        fn get_random_text() -> String {
            let mut rng = rand::thread_rng();
            let mut output = Vec::new();
            for _ in 0..rng.gen_range(0..100) {
                output.push(rng.gen_range(0..128));
            }
            unsafe { String::from_utf8_unchecked(output) }
        }

        let mut rng = rand::thread_rng();
        let mut buffer = Buffer::new();
        let view_id = buffer.get_first_view_or_create();

        for _ in 0..1000 {
            match rng.gen_range(0..5) {
                0 => {
                    buffer.move_left_char(view_id, false);
                }
                1 => {
                    buffer.move_left_char(view_id, false);
                }
                2 => {
                    buffer.move_up(view_id, false, false, 0);
                }
                3 => {
                    buffer.move_down(view_id, false, false, 0);
                }
                4 => {
                    let text = get_random_text();
                    buffer.insert_text(view_id, &text, false);
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn coalesce_random() {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    for _ in 0..10 {
        let mut cursors = Vec::new();

        for _ in 0..1000 {
            let position = rng.gen_range(0..100);
            let anchor = rng.gen_range(0..100);
            let cursor = Cursor {
                position,
                anchor,
                affinity: 0,
            };
            cursors.push(cursor);
        }

        let mut view = View {
            cursors: Vec1::from_vec(cursors).unwrap(),
            ..Default::default()
        };

        view.coalesce_cursors();

        for i in 0..view.cursors.len() {
            for j in 0..view.cursors.len() {
                if i == j {
                    continue;
                }

                if view.cursors[i].intersects(view.cursors[j]) {
                    eprintln!("\n{:?}\n{:?}\n", view.cursors[i], view.cursors[j]);
                }

                assert!(!view.cursors[i].intersects(view.cursors[j]));
            }
        }
    }
}

#[test]
fn colaese_single() {
    let cursor1 = Cursor {
        position: 18,
        anchor: 93,
        affinity: 0,
    };
    let cursor2 = Cursor {
        position: 51,
        anchor: 74,
        affinity: 0,
    };

    let mut view = View {
        cursors: Vec1::from_vec(vec![cursor1, cursor2]).unwrap(),
        ..Default::default()
    };

    view.coalesce_cursors();

    for i in 0..view.cursors.len() {
        for j in 0..view.cursors.len() {
            if i == j {
                continue;
            }

            if view.cursors[i].intersects(view.cursors[j]) {
                eprintln!("\n{:?}\n{:?}\n", view.cursors[i], view.cursors[j]);
            }

            assert!(!view.cursors[i].intersects(view.cursors[j]));
        }
    }
}

#[test]
fn colaese_single_edge() {
    let cursor1 = Cursor {
        position: 0,
        anchor: 65,
        affinity: 0,
    };
    let cursor2 = Cursor {
        position: 0,
        anchor: 0,
        affinity: 0,
    };

    let mut view = View {
        cursors: Vec1::from_vec(vec![cursor1, cursor2]).unwrap(),
        ..Default::default()
    };

    view.coalesce_cursors();

    for i in 0..view.cursors.len() {
        for j in 0..view.cursors.len() {
            if i == j {
                continue;
            }

            if view.cursors[i].intersects(view.cursors[j]) {
                eprintln!("\n{:?}\n{:?}\n", view.cursors[i], view.cursors[j]);
            }

            assert!(!view.cursors[i].intersects(view.cursors[j]));
        }
    }
}
