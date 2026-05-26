use std::{fs::File, path::Path};

use ignore::DirEntry;

use crate::{config::editor::PickerConfig, pubsub::Subscriber};

pub struct FileFindProvider(pub Subscriber<boxcar::Vec<String>>);
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use ferrite_utility::trim::trim_path;

use crate::event_loop_proxy::get_proxy;

pub fn filter_picker_entry(entry: &DirEntry, root: &Path, dedup_symlinks: bool) -> bool {
    // We always want to ignore popular VCS directories, otherwise if
    // `ignore` is turned off, we end up with a lot of noise
    // in our picker.
    if matches!(
        entry.file_name().to_str(),
        Some(".git" | ".pijul" | ".jj" | ".hg" | ".svn")
    ) {
        return false;
    }

    // We also ignore symlinks that point inside the current directory
    // if `dedup_links` is enabled.
    if dedup_symlinks && entry.path_is_symlink() {
        return entry
            .path()
            .canonicalize()
            .ok()
            .is_some_and(|path| !path.starts_with(root));
    }

    true
}

fn get_text_file_path(path: PathBuf) -> Option<PathBuf> {
    if is_text_file(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_text_file(path: impl AsRef<Path>) -> bool {
    let Ok(mut file) = File::open(&path) else {
        return false;
    };

    let mut buf = [0; 1024];
    let Ok(read) = file.read(&mut buf) else {
        return false;
    };

    let content_type = content_inspector::inspect(&buf[..read]);
    content_type.is_text()
}

pub fn file_injector(
    config: &PickerConfig,
    source_file_cache: Option<Arc<boxcar::Vec<String>>>,
    target_file_cache: Option<Arc<boxcar::Vec<String>>>,
) -> impl FnOnce(nucleo::Injector<String>, Arc<AtomicBool>) {
    let path = std::env::current_dir().unwrap();
    let path_str: String = path.to_string_lossy().into();
    let show_only_text_files = config.show_only_text_files;
    let iterator = ignore::WalkBuilder::new(&path)
        .follow_links(config.follow_symlinks)
        .hidden(!config.show_hidden)
        .ignore(config.follow_ignore)
        .git_global(config.follow_git_global)
        .git_ignore(config.follow_gitignore)
        .git_exclude(config.follow_git_exclude)
        .filter_entry(move |entry| filter_picker_entry(entry, &path, true))
        .sort_by_file_name(move |lhs, rhs| {
            let lhs = lhs.to_string_lossy();
            let rhs = rhs.to_string_lossy();
            ferrite_utility::natural_cmp::natural_cmp(&lhs, &rhs)
        })
        .build()
        .filter_map(move |entry| {
            let entry = entry.ok()?;
            if !entry.path().is_file() {
                return None;
            }

            let path = if show_only_text_files {
                get_text_file_path(entry.into_path())?
            } else {
                entry.into_path()
            };

            Some(trim_path(&path_str, &path))
        });

    |injector, _running| {
        rayon::spawn(move || {
            if let Some(cache) = source_file_cache {
                for (_, string) in cache.iter() {
                    injector.push(string.clone(), |item, utf32_string| {
                        utf32_string[0] = nucleo::Utf32String::from(item.as_str())
                    });
                }
            } else {
                for string in iterator {
                    if let Some(cache) = &target_file_cache {
                        cache.push(string.clone());
                    }
                    injector.push(string, |item, utf32_string| {
                        utf32_string[0] = nucleo::Utf32String::from(item.as_str())
                    });
                }
            }

            get_proxy().request_render("file injector done");
        });
    }
}
