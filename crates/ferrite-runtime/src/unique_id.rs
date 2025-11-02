use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct UniqueId(NonZeroU64);

impl UniqueId {
    pub fn new() -> Self {
        unsafe {
            Self(NonZeroU64::new_unchecked(
                NEXT_ID.fetch_add(1, Ordering::Relaxed),
            ))
        }
    }

    #[inline(always)]
    pub fn value(&self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Debug for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)
    }
}
