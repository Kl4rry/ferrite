use std::path::PathBuf;

use lexical_sort::StringSort;
use rayon::prelude::*;

use super::SearchOptionProvider;

pub struct FileFindProvider(pub PathBuf);

impl SearchOptionProvider for FileFindProvider {
    type Matchable = String;
    fn get_options(&self) -> Vec<Self::Matchable> {
        let path: PathBuf = self.0.clone();
        let path_str = path.to_string_lossy().to_string();

        let mut files = Vec::new();

        let entries: Vec<_> = jwalk::WalkDir::new(&path)
            .follow_links(true)
            .into_iter()
            .filter_map(|result| result.ok())
            .collect();
        files.par_extend(
            entries
                .par_iter()
                .filter(|entry| entry.file_type().is_file())
                .filter_map(|entry| {
                    let path = entry.path();
                    if tree_magic_mini::from_filepath(&path)?.starts_with("text") {
                        Some(
                            path.to_string_lossy()
                                .trim_start_matches(&path_str)
                                .trim_start_matches(std::path::MAIN_SEPARATOR)
                                .to_string(),
                        )
                    } else {
                        None
                    }
                }),
        );
        files.string_sort(lexical_sort::natural_lexical_cmp);
        files
    }
}
