use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub type ArenaString<'a> = bumpalo::collections::String<'a>;
pub type ArenaVec<'a, T> = bumpalo::collections::Vec<'a, T>;
pub type Arena = bumpalo::Bump;

pub use bumpalo::format;

thread_local!(static CTX: Ctx = Ctx(Rc::new(Inner { arena_alloc: RefCell::new(bumpalo::Bump::new()) })));

struct Inner {
    arena_alloc: RefCell<bumpalo::Bump>,
}

#[derive(Clone)]
pub struct Ctx(Rc<Inner>);

impl Ctx {
    pub fn arena() -> ArenaGuard {
        let ctx = Self::get();
        ArenaGuard {
            guard: unsafe { (*Rc::as_ptr(&ctx.0)).arena_alloc.borrow() },
            _rc: ctx.0,
        }
    }

    pub fn arena_mut() -> MutArenaGuard {
        let ctx = Self::get();
        MutArenaGuard {
            guard: unsafe { (*Rc::as_ptr(&ctx.0)).arena_alloc.borrow_mut() },
            _rc: ctx.0,
        }
    }

    pub fn get() -> Self {
        CTX.with(|v| v.clone())
    }
}

pub struct ArenaGuard {
    _rc: Rc<Inner>,
    guard: std::cell::Ref<'static, bumpalo::Bump>,
}

impl Deref for ArenaGuard {
    type Target = bumpalo::Bump;
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.guard
    }
}

pub struct MutArenaGuard {
    _rc: Rc<Inner>,
    guard: std::cell::RefMut<'static, bumpalo::Bump>,
}

impl Deref for MutArenaGuard {
    type Target = bumpalo::Bump;
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.guard
    }
}

impl DerefMut for MutArenaGuard {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.guard
    }
}
