use std::sync::Arc;

use cb::{Receiver, RecvError, SendError, Sender};

pub struct Publisher<T> {
    sender: Sender<()>,
    data: Arc<T>,
}

impl<T> Publisher<T> {
    pub fn modify(&self, f: impl FnOnce(&T)) {
        (f)(&*self.data);
    }

    pub fn publish(&self) -> Result<(), SendError<()>> {
        self.sender.send(())
    }
}

pub struct Subscriber<T> {
    data: Arc<T>,
    reciver: Receiver<()>,
    has_recived: bool,
}

impl<T> Subscriber<T> {
    pub fn recive(&mut self) -> Result<Arc<T>, RecvError> {
        if !self.has_recived {
            self.has_recived = true;
            return Ok(self.data.clone());
        }

        self.reciver.recv()?;
        Ok(self.data.clone())
    }

    pub fn get(&self) -> Arc<T> {
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
    let (sender, reciver) = cb::unbounded::<()>();
    let data = Arc::new(value);
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
