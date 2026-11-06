//! A counting `GlobalAlloc` wrapper: the bytes allocated now and their peak
//! since the last reset (#1210). A test binary installs it as its
//! `#[global_allocator]` to assert a scenario's heap peak without the GUI.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts the live bytes of the allocator it wraps. The counters are
/// statistics, not synchronisation, so every atomic is `Relaxed`; `fetch_max`
/// keeps the peak monotone between resets regardless of ordering.
pub struct PeakAlloc<A = System> {
    inner: A,
    current: AtomicUsize,
    peak: AtomicUsize,
}

impl PeakAlloc<System> {
    pub const fn system() -> Self {
        Self::new(System)
    }
}

impl<A> PeakAlloc<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            current: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }

    /// Bytes allocated and not yet freed.
    pub fn current(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    /// The largest [`Self::current`] since the last [`Self::reset`].
    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    /// Start a new peak window at the current count.
    pub fn reset(&self) {
        self.peak.store(self.current(), Ordering::Relaxed);
    }

    fn grow(&self, bytes: usize) {
        let now = self.current.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.peak.fetch_max(now, Ordering::Relaxed);
    }

    fn shrink(&self, bytes: usize) {
        self.current.fetch_sub(bytes, Ordering::Relaxed);
    }
}

// SAFETY: every call forwards to `inner` unchanged; the wrapper only counts the
// sizes of the blocks `inner` actually handed out or took back.
unsafe impl<A: GlobalAlloc> GlobalAlloc for PeakAlloc<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.grow(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc_zeroed(layout) };
        if !ptr.is_null() {
            self.grow(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.shrink(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { self.inner.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            if new_size >= layout.size() {
                self.grow(new_size - layout.size());
            } else {
                self.shrink(layout.size() - new_size);
            }
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A private counter over `System`: the test binary's own allocations do
    // not pass through it, so the counts are exact.
    fn counter() -> PeakAlloc {
        PeakAlloc::system()
    }

    #[test]
    fn peak_alloc_tracks_alloc_and_free() {
        let a = counter();
        let small = Layout::from_size_align(100, 8).unwrap();
        let big = Layout::from_size_align(1000, 8).unwrap();
        unsafe {
            let p = a.alloc(small);
            let q = a.alloc_zeroed(big);
            assert_eq!((a.current(), a.peak()), (1100, 1100));
            a.dealloc(q, big);
            assert_eq!((a.current(), a.peak()), (100, 1100));
            a.dealloc(p, small);
        }
        assert_eq!((a.current(), a.peak()), (0, 1100));
    }

    #[test]
    fn peak_alloc_tracks_realloc() {
        let a = counter();
        let layout = Layout::from_size_align(64, 8).unwrap();
        unsafe {
            let p = a.alloc(layout);
            let p = a.realloc(p, layout, 4096);
            assert_eq!((a.current(), a.peak()), (4096, 4096));
            let grown = Layout::from_size_align(4096, 8).unwrap();
            let p = a.realloc(p, grown, 512);
            assert_eq!((a.current(), a.peak()), (512, 4096));
            a.dealloc(p, Layout::from_size_align(512, 8).unwrap());
        }
        assert_eq!(a.current(), 0);
    }

    #[test]
    fn peak_alloc_reset_sets_peak_to_current() {
        let a = counter();
        let layout = Layout::from_size_align(256, 8).unwrap();
        unsafe {
            let p = a.alloc(layout);
            let q = a.alloc(layout);
            a.dealloc(q, layout);
            assert_eq!(a.peak(), 512);
            a.reset();
            assert_eq!((a.current(), a.peak()), (256, 256));
            let r = a.alloc(layout);
            assert_eq!(a.peak(), 512);
            a.dealloc(r, layout);
            a.dealloc(p, layout);
        }
        a.reset();
        assert_eq!(a.peak(), 0);
    }
}
