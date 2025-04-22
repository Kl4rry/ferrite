use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

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
    guard: std::cell::RefMut<'static, bumpalo::Bump>,
}

impl Deref for ArenaGuard {
    type Target = bumpalo::Bump;
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.guard
    }
}

impl DerefMut for ArenaGuard {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.guard
    }
}
