//! Deterministic non-cryptographic content digest used as a cache identity.

/// A 64-bit FNV-1a digest of `bytes`.
///
/// This is a cache identity, not a security hash: equal digests are treated as
/// equal content, so a collision can only stale a cache entry, never corrupt a
/// document.
pub fn content_digest(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
