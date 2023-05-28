use std::sync::{Arc, RwLock};

use flume::{Receiver, RecvError, SendError, Sender};

pub struct Publisher<T> {
    sender: Sender<()>,
    data: Arc<RwLock<Arc<T>>>,
}

impl<T> Publisher<T> {
    pub fn publish(&self, value: T) -> Result<(), SendError<()>> {
        *self.data.write().unwrap() = Arc::new(value);
        self.sender.send(())
    }
}

pub struct Subscriber<T> {
    data: Arc<RwLock<Arc<T>>>,
    reciver: Receiver<()>,
    has_recived: bool,
}

impl<T> Subscriber<T> {
    pub fn recive(&mut self) -> Result<Arc<T>, RecvError> {
        if !self.has_recived {
            self.has_recived = true;
            return Ok(self.data.read().unwrap().clone());
        }

        self.reciver.recv()?;
        Ok(self.data.read().unwrap().clone())
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
    let data = Arc::new(RwLock::new(Arc::new(value)));
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
