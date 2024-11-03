use std::{
    alloc::{GlobalAlloc, System},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A debugging allocator that tracks number of allocations and memory used
pub struct Talloc;

impl Talloc {
    #[inline(always)]
    pub fn num_allocations() -> usize {
        NUMBER_OF_ALLOCATIONS.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn total_memory_allocated() -> usize {
        TOTAL_MEMORY_ALLOCATED.load(Ordering::Relaxed)
    }

    pub fn phase_allocations() -> usize {
        PHASE_ALLOCATIONS.load(Ordering::Relaxed)
    }

    pub fn reset_phase_allocations() {
        PHASE_ALLOCATIONS.store(0, Ordering::Relaxed);
    }
}

static NUMBER_OF_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_MEMORY_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PHASE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Talloc {
    #[inline(always)]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        NUMBER_OF_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        TOTAL_MEMORY_ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        PHASE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System::alloc(&System, layout)
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        NUMBER_OF_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
        TOTAL_MEMORY_ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System::dealloc(&System, ptr, layout)
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        NUMBER_OF_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        TOTAL_MEMORY_ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        PHASE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System::alloc_zeroed(&System, layout)
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            TOTAL_MEMORY_ALLOCATED.fetch_add(new_size - layout.size(), Ordering::Relaxed);
        } else {
            TOTAL_MEMORY_ALLOCATED.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
        }
        PHASE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System::realloc(&System, ptr, layout, new_size)
    }
}
