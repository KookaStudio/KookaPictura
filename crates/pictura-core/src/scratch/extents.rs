//! The free space inside one scratch file: 4 KiB-aligned ranges, first fit,
//! neighbours coalesced on release.

use std::collections::BTreeMap;

/// Extent alignment in bytes.
pub(super) const ALIGN: u64 = 4096;

/// `len` rounded up to [`ALIGN`] (at least one block).
pub(super) fn aligned(len: u64) -> u64 {
    len.max(1).div_ceil(ALIGN) * ALIGN
}

/// Free ranges by offset.
#[derive(Default)]
pub(super) struct FreeMap {
    ranges: BTreeMap<u64, u64>,
    bytes: u64,
}

impl FreeMap {
    /// Take `len` aligned bytes from the first range that holds them.
    pub(super) fn take(&mut self, len: u64) -> Option<u64> {
        let (&offset, &have) = self.ranges.iter().find(|(_, &have)| have >= len)?;
        self.ranges.remove(&offset);
        if have > len {
            self.ranges.insert(offset + len, have - len);
        }
        self.bytes -= len;
        Some(offset)
    }

    /// Return `offset .. offset + len`, merging it with the ranges it touches.
    pub(super) fn give(&mut self, offset: u64, len: u64) {
        let (mut start, mut end) = (offset, offset + len);
        if let Some((&before, &blen)) = self.ranges.range(..offset).next_back() {
            if before + blen == start {
                self.ranges.remove(&before);
                start = before;
            }
        }
        if let Some(&alen) = self.ranges.get(&end) {
            self.ranges.remove(&end);
            end += alen;
        }
        self.ranges.insert(start, end - start);
        self.bytes += len;
    }

    pub(super) fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_ranges_coalesce_and_refill() {
        let mut map = FreeMap::default();
        map.give(0, 3 * ALIGN);
        let a = map.take(ALIGN).unwrap();
        let b = map.take(ALIGN).unwrap();
        let c = map.take(ALIGN).unwrap();
        assert_eq!((a, b, c), (0, ALIGN, 2 * ALIGN));
        assert_eq!(map.take(ALIGN), None);
        map.give(b, ALIGN);
        map.give(a, ALIGN);
        map.give(c, ALIGN);
        assert_eq!(map.ranges.len(), 1, "neighbours merge into one range");
        assert_eq!(map.take(3 * ALIGN), Some(0));
    }
}
