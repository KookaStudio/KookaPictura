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
pub fn available_memory() -> Option<u64> {
    pictura_core::memory::available_memory()
}

/// The host's physical memory in bytes, or `None` when it cannot be read.
pub fn total_memory() -> Option<u64> {
    pictura_core::memory::total_memory()
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
