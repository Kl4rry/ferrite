use std::os::raw::c_int;

const M_MMAP_THRESHOLD: c_int = -3;

#[cfg(target_env = "gnu")]
unsafe extern "C" {
    fn malloc_trim(pad: usize);
    fn mallopt(param: c_int, value: c_int) -> c_int;
}

#[inline]
pub fn trim(_pad: usize) {
    #[cfg(target_env = "gnu")]
    unsafe {
        malloc_trim(_pad);
    }
}

/// Prevents glibc from hoarding memory via memory fragmentation.
#[inline]
pub fn limit_mmap_threshold(_threshold: i32) {
    #[cfg(target_env = "gnu")]
    unsafe {
        mallopt(M_MMAP_THRESHOLD, _threshold as c_int);
    }
}
