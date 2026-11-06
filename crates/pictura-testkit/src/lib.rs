//! Verification harness for Kooka Pictura.
//!
//! M0 scope: a deterministic golden-image comparison. The comparison model is
//! specified in `docs/11-cross-cutting/testing-strategy.md`: decode both sides
//! into one comparison space (planar, 8-bit is enough for M0) and apply a
//! per-operation-class tolerance.

/// Result of comparing two same-shaped buffers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// Total samples compared.
    pub samples: usize,
    /// Samples that differ by more than the tolerance.
    pub differing: usize,
    /// Largest absolute difference observed.
    pub max_delta: u16,
    /// Sum of absolute differences (for a mean metric).
    pub sum_delta: u64,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.differing == 0
    }

    pub fn mean_delta(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            self.sum_delta as f64 / self.samples as f64
        }
    }
}

/// Compare two byte buffers of equal length with an absolute per-sample
/// tolerance. Returns `Err` if lengths differ.
pub fn compare(a: &[u8], b: &[u8], tolerance: u8) -> Result<Diff, String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    let mut differing = 0usize;
    let mut max_delta = 0u16;
    let mut sum_delta = 0u64;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = x.abs_diff(*y);
        if d > tolerance {
            differing += 1;
        }
        max_delta = max_delta.max(d as u16);
        sum_delta += d as u64;
    }
    Ok(Diff {
        samples: a.len(),
        differing,
        max_delta,
        sum_delta,
    })
}

mod peak_alloc;
pub use peak_alloc::PeakAlloc;

/// Stable content hash for determinism checks.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    // FNV-1a: tiny, dependency-free, good enough for M0 determinism checks.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_boundary_passes_at_and_fails_above() {
        let tolerance = 3;
        let at = compare(&[10, 10], &[13, 7], tolerance).unwrap();
        assert_eq!(at.differing, 0);
        assert_eq!(at.max_delta, 3);

        let over = compare(&[10], &[14], tolerance).unwrap();
        assert_eq!(over.differing, 1);
        assert_eq!(over.max_delta, 4);
    }

    #[test]
    fn length_mismatch_is_an_error() {
        assert!(compare(&[0, 1, 2], &[0, 1], 0).is_err());
    }

    #[test]
    fn hash_is_stable_and_detects_change() {
        assert_eq!(hash_bytes(b"pictura"), hash_bytes(b"pictura"));
        assert_ne!(hash_bytes(b"pictura"), hash_bytes(b"picturb"));
    }

    #[test]
    fn fixed_seed_input_hashes_equal_twice() {
        // ponytail: tiny LCG stands in for the seeded RNG; swap for ChaCha8
        // when a real determinism module lands.
        let sample = || {
            let mut state = 0x1234_5678_9abc_def0u64;
            (0..64)
                .map(|_| {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (state >> 33) as u8
                })
                .collect::<Vec<u8>>()
        };
        assert_eq!(hash_bytes(&sample()), hash_bytes(&sample()));
    }
}
