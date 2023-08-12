use std::{sync::Arc, thread};

use super::SearchOptionProvider;
use crate::ferrite_core::pubsub::Subscriber;

pub struct FileFindProvider(pub Subscriber<Vec<String>>);

impl SearchOptionProvider for FileFindProvider {
    type Matchable = String;
    fn get_options_reciver(&self) -> cb::Receiver<Arc<Vec<Self::Matchable>>> {
        // TODO fix
        let mut subscriber = self.0.clone();
        let (tx, rx) = cb::bounded(1);
        thread::spawn(move || loop {
            let Ok(value) = subscriber.recive() else {
                break;
            };

            if tx.send(value).is_err() {
                break;
            }
        });
        rx
    }
}
