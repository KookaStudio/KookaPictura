//! The allocation budget an untrusted document decodes under (#844).
//!
//! A file's header, layer rects, and compressed streams are attacker-chosen, so
//! every plane is charged here before it is allocated: one allocation may not
//! exceed [`ImageBudget::max_alloc_bytes`], and one document may not claim more
//! than [`ImageBudget::max_document_bytes`] in all, which by default follows
//! the memory the host has available. A refusal is a typed error, never an
//! allocator abort.

use std::io::Read;

use crate::error::PsdError;
use crate::probe::ImageBudget;

/// When the host's available memory cannot be read, a document may claim this
/// many maximum-size allocations (16 GiB by default): enough for a
/// 16000²-class RGB document with about a dozen full-canvas layers.
const FALLBACK_ALLOCATIONS: u64 = 8;

/// The bytes one document may claim: three quarters of `available` (the rest
/// is headroom for the composite, history, and other processes), never less
/// than one `per_allocation` plane, or [`FALLBACK_ALLOCATIONS`] planes when
/// the host's memory is unknown.
pub(crate) fn document_bytes(available: Option<u64>, per_allocation: u64) -> u64 {
    match available {
        Some(bytes) => (bytes / 4 * 3).max(per_allocation),
        None => per_allocation.saturating_mul(FALLBACK_ALLOCATIONS),
    }
}

/// The memory the host can give a new allocation now without swapping, in
/// bytes, or `None` when it cannot be read.
pub(crate) fn available_memory() -> Option<u64> {
    host::available_memory()
}

#[cfg(target_os = "linux")]
mod host {
    /// `MemAvailable` from `/proc/meminfo`: the kernel's estimate of memory
    /// available to start new work, counting reclaimable cache.
    pub(super) fn available_memory() -> Option<u64> {
        super::mem_available(&std::fs::read_to_string("/proc/meminfo").ok()?)
    }
}

#[cfg(target_os = "macos")]
mod host {
    /// Free, inactive, and purgeable pages from the Mach VM statistics: the
    /// pages macOS can hand out without paging anything else out.
    #[allow(deprecated)] // libc points `mach_host_self` at the mach2 crate.
    pub(super) fn available_memory() -> Option<u64> {
        let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: `stats` is a zeroed `vm_statistics64` and `count` its size in
        // `integer_t` units, as `host_statistics64` requires.
        let status = unsafe {
            libc::host_statistics64(
                libc::mach_host_self(),
                libc::HOST_VM_INFO64,
                (&mut stats as *mut libc::vm_statistics64).cast(),
                &mut count,
            )
        };
        // SAFETY: `sysconf` has no preconditions.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if status != libc::KERN_SUCCESS || page <= 0 {
            return None;
        }
        let pages = u64::from(stats.free_count)
            + u64::from(stats.inactive_count)
            + u64::from(stats.purgeable_count);
        Some(pages * page as u64)
    }
}

#[cfg(windows)]
mod host {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    /// `ullAvailPhys`: physical memory available without paging.
    pub(super) fn available_memory() -> Option<u64> {
        // SAFETY: MEMORYSTATUSEX is plain data; `dwLength` is set as the API
        // requires before the call fills the rest.
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(status.ullAvailPhys)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod host {
    pub(super) fn available_memory() -> Option<u64> {
        None
    }
}

/// The `MemAvailable` line of a `/proc/meminfo` text, in bytes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn mem_available(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let mut fields = line["MemAvailable:".len()..].split_whitespace();
    let kib: u64 = fields.next()?.parse().ok()?;
    (fields.next() == Some("kB")).then(|| kib.saturating_mul(1024))
}

/// Running allocation budget for one document decode.
#[derive(Debug, Clone)]
pub(crate) struct DecodeBudget {
    per_allocation: u64,
    remaining: u64,
}

impl DecodeBudget {
    pub(crate) fn new(budget: ImageBudget) -> Self {
        Self {
            per_allocation: budget.max_alloc_bytes,
            remaining: budget.max_document_bytes,
        }
    }

    /// Charge one allocation of the product of `factors` bytes, returning that
    /// size. Overflow, an allocation over the per-allocation cap, or one that
    /// would exceed what the document has left are refused.
    pub(crate) fn charge(&mut self, what: &str, factors: &[usize]) -> Result<usize, PsdError> {
        let bytes = factors
            .iter()
            .try_fold(1u64, |n, &f| n.checked_mul(f as u64))
            .filter(|&n| n <= self.per_allocation && n <= self.remaining)
            .ok_or_else(|| {
                PsdError::Unsupported(format!("{what} {factors:?} exceeds the decode budget"))
            })?;
        self.remaining -= bytes;
        usize::try_from(bytes).map_err(|_| PsdError::Unsupported(format!("{what} too large")))
    }
}

impl Default for DecodeBudget {
    fn default() -> Self {
        Self::new(ImageBudget::default())
    }
}

/// Whether a `width` × `height` RGBA raster fits the default [`ImageBudget`].
pub(crate) fn rgba_within_budget(width: u32, height: u32) -> bool {
    let budget = ImageBudget::default();
    width <= budget.max_dimension
        && height <= budget.max_dimension
        && u64::from(width) * u64::from(height) * 4 <= budget.max_alloc_bytes
}

/// Read `reader` to its end, or `None` once it yields more than `limit` bytes.
/// A read error keeps what was read, as PDF readers do with truncated streams.
pub(crate) fn read_bounded(reader: impl Read, limit: u64) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let _ = reader.take(limit.saturating_add(1)).read_to_end(&mut out);
    (out.len() as u64 <= limit).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> DecodeBudget {
        DecodeBudget::new(ImageBudget {
            max_dimension: 1000,
            max_alloc_bytes: 100,
            max_document_bytes: 400,
        })
    }

    #[test]
    fn one_allocation_over_the_cap_is_refused() {
        let mut budget = small();
        assert_eq!(budget.charge("plane", &[10, 10]).unwrap(), 100);
        assert!(budget.charge("plane", &[101]).is_err());
    }

    #[test]
    fn the_document_total_is_bounded() {
        let mut budget = small();
        for _ in 0..4 {
            budget.charge("plane", &[100]).unwrap();
        }
        assert!(budget.charge("plane", &[1]).is_err());
    }

    const GIB: u64 = 1 << 30;

    #[test]
    fn the_document_limit_follows_available_memory() {
        // A large free host raises the limit; a busy one keeps one plane; an
        // unknown one keeps the fixed eight planes.
        assert_eq!(document_bytes(Some(48 * GIB), 2 * GIB), 36 * GIB);
        assert_eq!(document_bytes(Some(5 * GIB), 2 * GIB), 5 * GIB / 4 * 3);
        assert_eq!(document_bytes(Some(GIB), 2 * GIB), 2 * GIB);
        assert_eq!(document_bytes(None, 2 * GIB), 16 * GIB);
        assert!(ImageBudget::default().max_document_bytes >= 2 * GIB);
    }

    #[test]
    fn meminfo_available_is_parsed_in_bytes() {
        let text = "MemTotal:       32000000 kB\nMemFree:  100 kB\nMemAvailable:   22298680 kB\n";
        assert_eq!(mem_available(text), Some(22_298_680 * 1024));
        assert_eq!(mem_available("MemTotal: 5 kB\n"), None);
        assert_eq!(mem_available("MemAvailable: lots\n"), None);
    }

    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn the_host_reports_available_memory() {
        assert!(available_memory().is_some_and(|bytes| bytes > 0));
    }

    #[test]
    fn an_overflowing_product_is_refused() {
        let mut budget = DecodeBudget::default();
        assert!(budget
            .charge("plane", &[usize::MAX, usize::MAX, 4])
            .is_err());
    }

    #[test]
    fn a_bounded_read_refuses_more_than_the_limit() {
        assert_eq!(read_bounded(&b"abcd"[..], 4).as_deref(), Some(&b"abcd"[..]));
        assert_eq!(read_bounded(&b"abcde"[..], 4), None);
    }
}
