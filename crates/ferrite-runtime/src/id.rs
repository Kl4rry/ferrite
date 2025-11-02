use std::num::NonZeroU64;

use ahash::RandomState;

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct Id(NonZeroU64);

impl Id {
    pub const NULL: Self = Self(NonZeroU64::MAX);

    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero (very bad luck)
        }
    }

    pub fn new(source: impl std::hash::Hash) -> Self {
        Self::from_hash(RandomState::with_seeds(1, 2, 3, 4).hash_one(source))
    }

    pub fn with(self, child: impl std::hash::Hash) -> Self {
        use std::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0.get());
        child.hash(&mut hasher);
        Self::from_hash(hasher.finish())
    }

    #[inline(always)]
    pub fn value(&self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)
    }
}

impl From<&'static str> for Id {
    #[inline]
    fn from(string: &'static str) -> Self {
        Self::new(string)
    }
}

impl From<String> for Id {
    #[inline]
    fn from(string: String) -> Self {
        Self::new(string)
    }
}
