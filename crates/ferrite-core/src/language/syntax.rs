use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use cb::Sender;
use ropey::Rope;

use crate::{
    event_loop_proxy::{EventLoopProxy, UserEvent},
    language::{LanguageLoader, TreeSitterConfig, language_to_name, name_to_language},
};

struct SyntaxWorker {
    name: &'static str,
    rope_tx: Sender<Rope>,
    tree_sitter_config: &'static TreeSitterConfig,
}

pub struct SharedSyntax {
    pub syntax: tree_house::Syntax,
    pub rope: Rope,
}

impl SyntaxWorker {
    fn new(
        language: tree_house::Language,
        tree_sitter_config: &'static TreeSitterConfig,
        proxy: Box<dyn EventLoopProxy<UserEvent>>,
        shared_syntax: Arc<Mutex<Option<SharedSyntax>>>,
    ) -> Result<Self> {
        let (rope_tx, rope_rx) = cb::unbounded::<Rope>();

        let Some(name) = language_to_name(language) else {
            anyhow::bail!("unkown language with id: {}", language.idx());
        };
        thread::spawn(move || {
            tracing::info!("Highlight thread started for `{name}`");
            let mut rope;

            loop {
                rope = match rope_rx.recv() {
                    Ok(rope) => rope,
                    Err(err) => {
                        tracing::info!("Exiting highlight thread: {err}");
                        break;
                    }
                };

                if !rope_rx.is_empty() {
                    continue;
                }

                let time = Instant::now();
                let syntax = tree_house::Syntax::new(
                    rope.slice(..),
                    language,
                    Duration::from_secs(1000),
                    &LanguageLoader,
                );
                tracing::debug!(
                    "syntax parsing took: {}us or {}ms",
                    time.elapsed().as_micros(),
                    time.elapsed().as_millis()
                );

                match syntax {
                    Ok(syntax) => {
                        *shared_syntax.lock().unwrap() = Some(SharedSyntax { syntax, rope });
                        proxy.request_render("syntax update parsed");
                    }
                    Err(err) => {
                        tracing::error!("Error parsing syntax: {}", err);
                    }
                }
            }

            tracing::info!("Syntax provider thread exit");
        });

        Ok(Self {
            tree_sitter_config,
            name,
            rope_tx,
        })
    }

    fn update_text(&self, rope: Rope) {
        let _ = self.rope_tx.send(rope);
    }
}

pub struct Syntax {
    syntax_woker: Option<SyntaxWorker>,
    proxy: Box<dyn EventLoopProxy<UserEvent>>,
    shared_syntax: Arc<Mutex<Option<SharedSyntax>>>,
}

impl Syntax {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Self {
        Self {
            syntax_woker: None,
            proxy,
            shared_syntax: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_language(&mut self, language: &str) -> Result<()> {
        if language == "text" {
            return Ok(());
        }
        match name_to_language(language) {
            Some((lang, tree_sitter_config)) => {
                tracing::info!("set lang to `{language}`");
                self.shared_syntax = Arc::new(Mutex::new(None));
                self.syntax_woker = Some(SyntaxWorker::new(
                    lang,
                    tree_sitter_config,
                    self.proxy.dup(),
                    self.shared_syntax.clone(),
                )?);
                Ok(())
            }
            None => bail!("Unknown language: `{language}`"),
        }
    }

    pub fn get_language_name(&self) -> Option<&str> {
        Some(self.syntax_woker.as_ref()?.name)
    }

    pub fn update_text(&self, rope: Rope) {
        if let Some(syntax) = &self.syntax_woker {
            syntax.update_text(rope);
        }
    }

    pub fn get_syntax(&self) -> MutexGuard<Option<SharedSyntax>> {
        self.shared_syntax.lock().unwrap()
    }

    pub fn tree_sitter_config(&self) -> Option<&'static TreeSitterConfig> {
        Some(self.syntax_woker.as_ref()?.tree_sitter_config)
    }
}
