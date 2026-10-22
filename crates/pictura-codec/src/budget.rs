//! The allocation budget an untrusted document decodes under (#844).
//!
//! A file's header, layer rects, and compressed streams are attacker-chosen, so
//! every plane is charged here before it is allocated: one allocation may not
//! exceed [`ImageBudget::max_alloc_bytes`], and one document may not claim more
//! than [`TOTAL_ALLOCATIONS`] of them in all. A refusal is a typed error, never
//! an allocator abort.

use std::io::Read;

use crate::error::PsdError;
use crate::probe::ImageBudget;

/// How many maximum-size allocations one document may claim in all.
///
/// Eight admits a 16000²-class RGB document with about a dozen full-canvas
/// layers (16 GiB by default).
///
/// ponytail: a fixed multiple of the per-allocation cap rather than a measure
/// of the machine's memory.
pub(crate) const TOTAL_ALLOCATIONS: u64 = 8;

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
            remaining: budget.max_alloc_bytes.saturating_mul(TOTAL_ALLOCATIONS),
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
        for _ in 0..TOTAL_ALLOCATIONS {
            budget.charge("plane", &[100]).unwrap();
        }
        assert!(budget.charge("plane", &[1]).is_err());
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
