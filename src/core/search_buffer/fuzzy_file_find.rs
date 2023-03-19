use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use rayon::prelude::*;

use super::ResultProvider;
use crate::tui_app::event_loop::TuiEventLoopProxy;

pub struct FuzzyFileFindProvider {
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<Vec<String>>,
    result: Vec<String>,
}

impl FuzzyFileFindProvider {
    pub fn new(path: impl Into<PathBuf>, proxy: TuiEventLoopProxy) -> Self {
        let (search_tx, search_rx): (_, Receiver<String>) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        let path: PathBuf = path.into();
        let path_str = path.to_string_lossy().to_string();
        thread::spawn(move || {
            let mut path_cache = Vec::new();

            while let Ok(term) = search_rx.recv() {
                let entries: Vec<_> = jwalk::WalkDir::new(&path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|result| result.ok())
                    .collect();
                path_cache.clear();
                path_cache.par_extend(
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

                let keywords: Vec<String> =
                    term.split_whitespace().map(|s| s.to_lowercase()).collect();
                let mut rankings: Vec<_> = path_cache
                    .par_iter()
                    .map(|path| {
                        let mut score = 0;
                        for keyword in &keywords {
                            score += path.to_lowercase().contains(keyword) as usize;
                        }
                        (score, path.as_str())
                    })
                    .filter(|(score, _)| *score >= keywords.len())
                    .collect();

                rankings.sort_by_key(|item| item.1);

                let mut output = Vec::new();
                for (_, path) in rankings.iter().take(100) {
                    output.push(path.to_string());
                }

                if result_tx.send(output.clone()).is_err() {
                    break;
                }

                proxy.request_render();
            }
        });

        let _ = search_tx.send(String::new());
        Self {
            tx: search_tx,
            rx: result_rx,
            result: Vec::new(),
        }
    }
}

impl ResultProvider for FuzzyFileFindProvider {
    fn poll_result(&mut self) -> &[String] {
        if let Ok(result) = self.rx.try_recv() {
            self.result = result;
        }
        &self.result
    }

    fn search(&mut self, term: String) {
        let _ = self.tx.send(term);
    }
}
