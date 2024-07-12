use std::{
    mem,
    sync::{Arc, RwLock},
    thread,
};

use sorted_vec::SortedSet;

use super::{file_daemon::LexicallySortedString, PickerOptionProvider};
use crate::pubsub::Subscriber;

pub struct FileFindProvider(pub Subscriber<SortedSet<LexicallySortedString>>);

impl PickerOptionProvider for FileFindProvider {
    type Matchable = String;
    fn get_options_reciver(&self) -> cb::Receiver<Arc<RwLock<Vec<Self::Matchable>>>> {
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

        // SAFE because both Sorted and LexicallySortedString are repr transparent to Vec and String
        unsafe { mem::transmute::<_, cb::Receiver<Arc<RwLock<Vec<Self::Matchable>>>>>(rx) }
    }
}
