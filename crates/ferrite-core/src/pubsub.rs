use std::sync::{Arc, RwLock};

use flume::{Receiver, RecvError, SendError, Sender};

pub struct Publisher<T> {
    sender: Sender<()>,
    data: Arc<RwLock<T>>,
}

impl<T> Publisher<T> {
    pub fn modify(&self, f: impl FnOnce(&mut T)) {
        let mut mut_ref = self.data.write().unwrap();
        (f)(&mut *mut_ref);
    }

    pub fn publish(&self) -> Result<(), SendError<()>> {
        self.sender.send(())
    }
}

pub struct Subscriber<T> {
    data: Arc<RwLock<T>>,
    reciver: Receiver<()>,
    has_recived: bool,
}

impl<T> Subscriber<T> {
    pub fn recive(&mut self) -> Result<Arc<RwLock<T>>, RecvError> {
        if !self.has_recived {
            self.has_recived = true;
            return Ok(self.data.clone());
        }

        self.reciver.recv()?;
        Ok(self.data.clone())
    }

    pub fn get(&self) -> Arc<RwLock<T>> {
        self.data.clone()
    }
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            reciver: self.reciver.clone(),
            has_recived: false,
        }
    }
}

pub fn create<T>(value: T) -> (Publisher<T>, Subscriber<T>) {
    let (sender, reciver) = flume::unbounded::<()>();
    let data = Arc::new(RwLock::new(value));
    (
        Publisher {
            sender,
            data: data.clone(),
        },
        Subscriber {
            reciver,
            data,
            has_recived: false,
        },
    )
}
