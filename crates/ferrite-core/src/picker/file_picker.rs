use std::{sync::Arc, thread};

use super::PickerOptionProvider;
use crate::pubsub::Subscriber;

pub struct FileFindProvider(pub Subscriber<boxcar::Vec<String>>);

impl PickerOptionProvider for FileFindProvider {
    type Matchable = String;
    fn get_options_reciver(&self) -> cb::Receiver<Arc<boxcar::Vec<Self::Matchable>>> {
        // TODO fix
        let mut subscriber = self.0.clone();
        let (tx, rx) = cb::bounded(1);
        thread::spawn(move || {
            loop {
                let Ok(value) = subscriber.recive() else {
                    break;
                };

                if tx.send(value).is_err() {
                    break;
                }
            }
        });

        rx
    }
}
