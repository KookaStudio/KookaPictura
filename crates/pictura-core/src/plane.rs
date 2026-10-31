//! Refcounted copy-on-write pixel planes.

use std::ops::{Deref, DerefMut, Range};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

/// Rows per band of a fractured plane.
pub const BAND_ROWS: usize = 64;

/// One band of a fractured plane: a range of the plane's base allocation, or a
/// forked copy of that range.
enum Band<T> {
    Shared(Range<usize>),
    Owned(Arc<[T]>),
}

impl<T> Clone for Band<T> {
    fn clone(&self) -> Self {
        match self {
            Band::Shared(range) => Band::Shared(range.clone()),
            Band::Owned(data) => Band::Owned(Arc::clone(data)),
        }
    }
}

struct Bands<T> {
    width: usize,
    bands: Vec<Band<T>>,
}

impl<T> Clone for Bands<T> {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            bands: self.bands.clone(),
        }
    }
}

impl<T> Bands<T> {
    fn diverged(&self) -> bool {
        self.bands.iter().any(|b| matches!(b, Band::Owned(_)))
    }
}

/// One image plane behind a refcount.
///
/// Cloning the owning document is a refcount bump, and a write forks only the
/// plane it touches, so a stroke or a history snapshot stops paying for pixels
/// that have not changed. A plane that is uniquely held keeps its allocation
/// and mutates in place exactly as a `Vec` did; only a shared plane copies on
/// first write.
///
/// A plane may also be **fractured** into 64-row bands: the plane keeps one
/// base allocation and a band view over it, so cloning the plane shares every
/// band and a band write forks at most that band's bytes. Fracturing copies no
/// pixels.
///
/// A plane also carries a **stamp**: two planes with the same nonzero stamp
/// hold the same bytes. Every mutable access clears it before a byte can
/// change and a clone keeps it, so the history can tell an untouched plane from
/// a written one without comparing them. Only [`Plane::set_stamp`] sets one.
///
/// An **ident** ([`Plane::ident`]) is the same promise made lazily through a
/// shared reference, for caches derived from the bytes: it is assigned on the
/// first query, kept by a clone, and cleared by every mutable access.
pub struct Plane<T = u8> {
    base: Arc<[T]>,
    bands: Option<Bands<T>>,
    mirror: OnceLock<Arc<[T]>>,
    stamp: u64,
    ident: AtomicU64,
}

impl<T> Plane<T> {
    fn wrap(data: Arc<[T]>) -> Self {
        Self {
            base: data,
            bands: None,
            mirror: OnceLock::new(),
            stamp: 0,
            ident: AtomicU64::new(0),
        }
    }

    fn diverged(&self) -> bool {
        self.bands.as_ref().is_some_and(Bands::diverged)
    }
}

impl<T> Clone for Plane<T> {
    fn clone(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            bands: self.bands.clone(),
            mirror: OnceLock::new(),
            stamp: self.stamp,
            ident: AtomicU64::new(self.ident.load(Ordering::Relaxed)),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Plane<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.bands {
            Some(bands) if bands.diverged() => f
                .debug_struct("Plane")
                .field("len", &self.base.len())
                .field("bands", &bands.bands.len())
                .finish(),
            _ => f.debug_tuple("Plane").field(&self.base).finish(),
        }
    }
}

/// Logical byte equality over the band iterators, so it needs no contiguous
/// slice (and no `T: Clone`).
fn logical_eq<T: PartialEq>(a: &Plane<T>, b: &Plane<T>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.shares(b) {
        return true;
    }
    let mut a = a.bands().flat_map(|band| band.iter());
    let mut b = b.bands().flat_map(|band| band.iter());
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if x == y => {}
            _ => return false,
        }
    }
}

fn eq_slice<T: PartialEq>(plane: &Plane<T>, other: &[T]) -> bool {
    plane.len() == other.len() && plane.bands().flat_map(|band| band.iter()).eq(other.iter())
}

impl<T: PartialEq> PartialEq for Plane<T> {
    fn eq(&self, other: &Self) -> bool {
        logical_eq(self, other)
    }
}

impl<T: Eq> Eq for Plane<T> {}

impl<T: Clone> Deref for Plane<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Clone> DerefMut for Plane<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Default for Plane<T> {
    fn default() -> Self {
        Self::wrap(Arc::from(Vec::new()))
    }
}

impl<T> From<Vec<T>> for Plane<T> {
    fn from(data: Vec<T>) -> Self {
        Self::wrap(Arc::from(data))
    }
}

/// A stamp no plane has carried before.
pub fn fresh_stamp() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

static MATERIALIZATIONS: AtomicU64 = AtomicU64::new(0);

/// How many times a diverged banded plane has been materialized into one
/// contiguous allocation.
pub fn materializations() -> u64 {
    MATERIALIZATIONS.load(Ordering::Relaxed)
}

/// Reset the materialization counter (tests and the profile).
pub fn reset_materializations() {
    MATERIALIZATIONS.store(0, Ordering::Relaxed);
}

fn materialize<T: Clone>(base: &[T], bands: &Bands<T>) -> Vec<T> {
    let mut out = Vec::with_capacity(base.len());
    for band in &bands.bands {
        match band {
            Band::Shared(range) => out.extend_from_slice(&base[range.clone()]),
            Band::Owned(data) => out.extend_from_slice(data),
        }
    }
    out
}

fn build_bands<T>(len: usize, width: usize) -> Bands<T> {
    if width == 0 || len == 0 {
        return Bands {
            width,
            bands: Vec::new(),
        };
    }
    let band_len = BAND_ROWS * width;
    let mut bands = Vec::with_capacity(len.div_ceil(band_len));
    let mut start = 0;
    while start < len {
        let end = (start + band_len).min(len);
        bands.push(Band::Shared(start..end));
        start = end;
    }
    Bands { width, bands }
}

impl<T> Plane<T> {
    /// The plane's logical samples as one contiguous slice.
    ///
    /// A fractured plane whose bands have not diverged reads its base
    /// allocation directly. Once a band has diverged the logical bytes are
    /// materialized once and cached until the next write.
    pub fn as_slice(&self) -> &[T]
    where
        T: Clone,
    {
        match &self.bands {
            Some(bands) if bands.diverged() => {
                let view = self.mirror.get_or_init(|| {
                    MATERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
                    Arc::from(materialize(&self.base, bands))
                });
                view
            }
            _ => self.base.as_ref(),
        }
    }

    /// The logical length; never materializes a banded plane.
    pub fn len(&self) -> usize {
        self.base.len()
    }

    /// Whether the plane holds no samples.
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// The plane's stamp; `0` when it has none.
    pub fn stamp(&self) -> u64 {
        self.stamp
    }

    /// A nonzero id no plane with different bytes has carried, assigned on
    /// first query. Two planes with the same ident hold the same bytes.
    pub fn ident(&self) -> u64 {
        let id = self.ident.load(Ordering::Relaxed);
        if id != 0 {
            return id;
        }
        let fresh = fresh_stamp();
        match self
            .ident
            .compare_exchange(0, fresh, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => fresh,
            Err(won) => won,
        }
    }

    /// Mark the plane's bytes with `stamp`. The caller vouches that every plane
    /// carrying the same stamp holds the same bytes.
    pub fn set_stamp(&mut self, stamp: u64) {
        self.stamp = stamp;
    }

    /// Whether `self` and `other` are one allocation with no diverged band.
    pub fn shares(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.base, &other.base) && !self.diverged() && !other.diverged()
    }

    /// The number of bands a fractured plane exposes; `1` for a flat plane.
    pub fn band_count(&self) -> usize {
        self.bands.as_ref().map_or(1, |b| b.bands.len())
    }

    /// The width a fractured plane was fractured at, or `None` while flat.
    pub fn width(&self) -> Option<usize> {
        self.bands.as_ref().map(|b| b.width)
    }

    /// Band `i`'s bytes, or `None` when there is no such band.
    pub fn band(&self, i: usize) -> Option<&[T]> {
        match &self.bands {
            None => (i == 0).then_some(self.base.as_ref()),
            Some(bands) => bands.bands.get(i).map(|band| match band {
                Band::Shared(range) => &self.base[range.clone()],
                Band::Owned(data) => data.as_ref(),
            }),
        }
    }

    /// Every band's bytes, in order.
    pub fn bands(&self) -> impl Iterator<Item = &[T]> + '_ {
        (0..self.band_count()).filter_map(move |i| self.band(i))
    }

    /// Row `y`'s `width` samples, or `None` when the row is out of range.
    ///
    /// The banded path uses the width recorded by `fracture`; `width` is only
    /// read while the plane is flat.
    pub fn row(&self, y: usize, width: usize) -> Option<&[T]> {
        if width == 0 {
            return None;
        }
        match &self.bands {
            None => {
                let start = y.checked_mul(width)?;
                self.base.get(start..start + width)
            }
            Some(bands) => {
                let w = bands.width;
                if w == 0 {
                    return None;
                }
                let band = self.band(y / BAND_ROWS)?;
                let off = (y % BAND_ROWS) * w;
                band.get(off..(off + w).min(band.len()))
            }
        }
    }

    /// The sample at logical index `i`, read from its band so that a diverged
    /// plane is not materialized; `None` past the end.
    pub fn sample(&self, i: usize) -> Option<T>
    where
        T: Copy,
    {
        match &self.bands {
            None => self.base.get(i).copied(),
            Some(bands) => {
                let band_len = BAND_ROWS.checked_mul(bands.width).filter(|&n| n > 0)?;
                self.band(i / band_len)?.get(i % band_len).copied()
            }
        }
    }

    /// The base allocation as `(pointer, length)`, for retained-byte accounting.
    pub fn base_allocation(&self) -> (usize, usize) {
        (self.base.as_ptr() as usize, self.base.len())
    }

    /// Each forked band allocation as `(pointer, length)`.
    pub fn owned_bands(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.bands
            .iter()
            .flat_map(|b| b.bands.iter())
            .filter_map(|band| match band {
                Band::Owned(data) => Some((data.as_ptr() as usize, data.len())),
                Band::Shared(_) => None,
            })
    }

    /// Bytes held by this plane's own allocations: the base plus any forked
    /// band. Does not count an allocation shared with another plane twice.
    pub fn allocated_bytes(&self) -> usize {
        self.base.len() + self.owned_bands().map(|(_, len)| len).sum::<usize>()
    }

    /// Whether band `i` is the same allocation as `other`'s band `i`.
    pub fn shares_band(&self, i: usize, other: &Self) -> bool {
        match (self.band_key(i), other.band_key(i)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    fn band_key(&self, i: usize) -> Option<(usize, usize, usize)> {
        match &self.bands {
            None => (i == 0).then(|| (self.base.as_ptr() as usize, 0, self.base.len())),
            Some(bands) => bands.bands.get(i).map(|band| match band {
                Band::Shared(range) => (self.base.as_ptr() as usize, range.start, range.end),
                Band::Owned(data) => (data.as_ptr() as usize, 0, data.len()),
            }),
        }
    }
}

impl Plane<u8> {
    /// A uniquely held plane of `len` bytes that `fill` writes in place.
    ///
    /// `Plane::from(Vec)` copies the vector into the refcounted allocation, and
    /// zero-filling a fresh one is a single-threaded pass; this takes zeroed
    /// pages straight from the allocator, so a caller filling a large plane in
    /// parallel pays for its own pass only.
    pub fn build(len: usize, fill: impl FnOnce(&mut [u8])) -> Self {
        let zeroed: Arc<[std::mem::MaybeUninit<u8>]> = Arc::new_zeroed_slice(len);
        // SAFETY: the allocation is zeroed, and every bit pattern, zero
        // included, is a valid `u8`.
        let mut data: Arc<[u8]> = unsafe { zeroed.assume_init() };
        fill(Arc::get_mut(&mut data).expect("a fresh allocation is unique"));
        Self::wrap(data)
    }
}

impl<T: Clone> Plane<T> {
    /// Make a materialized contiguous view the plane's base and rebuild the
    /// bands over it, so the next write keeps the plane banded without a second
    /// copy.
    fn adopt_mirror(&mut self) {
        let Some(view) = self.mirror.take() else {
            return;
        };
        self.base = view;
        if let Some(bands) = &self.bands {
            let width = bands.width;
            self.bands = Some(build_bands(self.base.len(), width));
        }
    }

    /// The plane's samples, mutable; a shared plane forks first and a fractured
    /// plane flattens into one contiguous allocation.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.stamp = 0;
        *self.ident.get_mut() = 0;
        if let Some(bands) = self.bands.take() {
            let logical: Arc<[T]> = if let Some(view) = self.mirror.take() {
                view
            } else if bands.diverged() {
                MATERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
                Arc::from(materialize(&self.base, &bands))
            } else {
                Arc::clone(&self.base)
            };
            self.base = logical;
        }
        self.mirror = OnceLock::new();
        Arc::make_mut(&mut self.base)
    }

    /// Fracture the plane into 64-row bands of `width`, copying no pixels.
    ///
    /// Re-fracturing at the same width is a no-op; a different width, or a
    /// plane whose bands have diverged, flattens once and rebuilds the bands.
    pub fn fracture(&mut self, width: usize) {
        if self.bands.as_ref().is_some_and(|b| b.width == width) {
            return;
        }
        self.adopt_mirror();
        if self.bands.as_ref().is_some_and(Bands::diverged) {
            MATERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
            let flat: Arc<[T]> = Arc::from(materialize(&self.base, self.bands.as_ref().unwrap()));
            self.base = flat;
        }
        self.mirror = OnceLock::new();
        self.bands = Some(build_bands(self.base.len(), width));
    }

    /// Band `i`'s samples, mutable; forks that band if it is not uniquely
    /// owned.
    pub fn write_band(&mut self, i: usize) -> Option<&mut [T]> {
        self.adopt_mirror();
        if i >= self.bands.as_ref()?.bands.len() {
            return None;
        }
        self.stamp = 0;
        *self.ident.get_mut() = 0;
        self.mirror = OnceLock::new();
        let bands = self.bands.as_mut()?;
        let base = &mut self.base;
        let band = bands.bands.get_mut(i)?;
        match band {
            Band::Owned(data) => {
                if Arc::get_mut(data).is_none() {
                    let copy: Arc<[T]> = Arc::from(data.to_vec());
                    *data = copy;
                }
                Arc::get_mut(data)
            }
            Band::Shared(range) => {
                if Arc::strong_count(base) == 1 {
                    let range = range.clone();
                    let base = Arc::get_mut(base).expect("a unique base");
                    Some(&mut base[range])
                } else {
                    let copy: Arc<[T]> = Arc::from(base[range.clone()].to_vec());
                    *band = Band::Owned(copy);
                    match band {
                        Band::Owned(data) => Arc::get_mut(data),
                        Band::Shared(_) => unreachable!("the band was just replaced"),
                    }
                }
            }
        }
    }

    /// The mutable slices of several bands, forking each one that is shared.
    ///
    /// The slices are disjoint, so a caller can split them across threads. A
    /// unique base keeps its bands shared and is flattened once in place, so
    /// every requested band is served. Returns `None` when `indices` is not
    /// strictly ascending, is out of range, or a shared-base fork cannot yield
    /// exactly one slice per index.
    pub fn write_bands(&mut self, indices: &[usize]) -> Option<Vec<&mut [T]>> {
        if indices.windows(2).any(|pair| pair[0] >= pair[1]) {
            return None;
        }
        for &i in indices {
            let _ = self.write_band(i);
        }
        let bands = self.bands.as_ref()?;
        if indices.iter().any(|&i| i >= bands.bands.len()) {
            return None;
        }
        let unique_base = indices
            .iter()
            .any(|&i| matches!(&bands.bands[i], Band::Shared(_)));
        if unique_base {
            let band_len = BAND_ROWS * bands.width;
            let flat = self.as_mut_slice();
            let total = flat.len();
            let mut rest = flat;
            let mut position = 0;
            let mut out = Vec::with_capacity(indices.len());
            for &i in indices {
                let start = i * band_len;
                let end = (start + band_len).min(total);
                if start < position || start >= end {
                    return None;
                }
                let (_, after_skip) = rest.split_at_mut(start - position);
                let (band, tail) = after_skip.split_at_mut(end - start);
                out.push(band);
                rest = tail;
                position = end;
            }
            return Some(out);
        }
        let bands = self.bands.as_mut()?;
        let mut out = Vec::with_capacity(indices.len());
        for (i, band) in bands.bands.iter_mut().enumerate() {
            if !indices.contains(&i) {
                continue;
            }
            match band {
                Band::Owned(data) => out.push(Arc::get_mut(data)?),
                Band::Shared(_) => return None,
            }
        }
        (out.len() == indices.len()).then_some(out)
    }

    /// Row `y`'s `width` samples, mutable; forks the band the row falls in.
    ///
    /// The banded path uses the width recorded by `fracture`; `width` is only
    /// read while the plane is flat.
    pub fn row_mut(&mut self, y: usize, width: usize) -> Option<&mut [T]> {
        if width == 0 {
            return None;
        }
        if self.bands.is_none() {
            let start = y.checked_mul(width)?;
            let end = start + width;
            let slice = self.as_mut_slice();
            if end <= slice.len() {
                Some(&mut slice[start..end])
            } else {
                None
            }
        } else {
            self.adopt_mirror();
            let (band, off, w) = {
                let bands = self.bands.as_ref()?;
                if bands.width == 0 {
                    return None;
                }
                (y / BAND_ROWS, (y % BAND_ROWS) * bands.width, bands.width)
            };
            let slice = self.write_band(band)?;
            let end = (off + w).min(slice.len());
            slice.get_mut(off..end)
        }
    }

    /// Append `more`, mirroring `Vec::extend_from_slice`.
    ///
    /// ponytail: a slice behind a refcount cannot grow in place, so this
    /// rebuilds; the callers build small buffers.
    pub fn extend_from_slice(&mut self, more: &[T]) {
        let mut data = self.as_slice().to_vec();
        data.extend_from_slice(more);
        *self = Self::wrap(Arc::from(data));
    }

    /// Extend from an iterator, mirroring `Vec::extend`.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut data = self.as_slice().to_vec();
        data.extend(iter);
        *self = Self::wrap(Arc::from(data));
    }

    /// Drop every sample, mirroring `Vec::clear`.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Shorten the plane to `len`, mirroring `Vec::truncate`.
    ///
    /// ponytail: a slice behind a refcount cannot shrink in place, so this
    /// rebuilds; the callers are the tests that build malformed buffers.
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            let mut data = self.as_slice().to_vec();
            data.truncate(len);
            *self = Self::wrap(Arc::from(data));
        }
    }
}

impl<T: Clone> AsRef<[T]> for Plane<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

#[cfg(test)]
pub(crate) fn shares(a: &Plane<u8>, b: &Plane<u8>) -> bool {
    a.shares(b)
}

impl<'a, T: Clone> IntoIterator for &'a Plane<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for Plane<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        eq_slice(self, other.as_slice())
    }
}

impl<T: PartialEq> PartialEq<[T]> for Plane<T> {
    fn eq(&self, other: &[T]) -> bool {
        eq_slice(self, other)
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for Plane<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        eq_slice(self, other)
    }
}

impl<T: PartialEq> PartialEq<&[T]> for Plane<T> {
    fn eq(&self, other: &&[T]) -> bool {
        eq_slice(self, other)
    }
}

impl<T> std::iter::FromIterator<T> for Plane<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::wrap(Arc::from(iter.into_iter().collect::<Vec<T>>()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
    }

    #[test]
    fn sample_reads_a_diverged_plane_without_materializing() {
        let width = 8;
        let bytes = pattern(width * BAND_ROWS * 3 - width / 2);
        let mut plane: Plane<u8> = bytes.clone().into();
        assert_eq!(plane.sample(5), Some(bytes[5]));
        plane.fracture(width);
        let shared = plane.clone();
        plane.write_band(1).expect("a band")[0] = bytes[0].wrapping_add(1);
        assert!(plane.diverged());

        let mut expected = bytes.clone();
        expected[width * BAND_ROWS] = bytes[0].wrapping_add(1);
        for (i, &byte) in expected.iter().enumerate() {
            assert_eq!(plane.sample(i), Some(byte), "sample {i}");
        }
        assert_eq!(plane.sample(bytes.len()), None);
        assert_eq!(
            shared.sample(width * BAND_ROWS),
            Some(bytes[width * BAND_ROWS])
        );
        assert!(plane.mirror.get().is_none(), "no contiguous view was built");
    }

    #[test]
    fn a_clone_shares_and_a_write_forks() {
        let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
        let b = a.clone();
        assert!(shares(&a, &b), "a clone must only bump the refcount");
        assert_eq!(a, b);

        a[0] = 9;
        assert!(!shares(&a, &b), "a write must fork the plane");
        assert_eq!(a, vec![9, 2, 3, 4]);
        assert_eq!(b, vec![1, 2, 3, 4], "the sharer keeps the old bytes");
    }

    #[test]
    fn an_ident_is_shared_by_copies_and_lost_by_any_change() {
        let mut a: Plane<u8> = vec![1, 2, 3].into();
        let id = a.ident();
        assert_ne!(id, 0);
        assert_eq!(a.ident(), id, "an ident is stable until a write");
        let b = a.clone();
        assert_eq!(b.ident(), id, "a clone keeps the ident");
        a.as_mut_slice()[0] = 9;
        assert_ne!(a.ident(), id, "a mutable access retires the ident");
        assert_eq!(b.ident(), id, "the untouched copy keeps it");
    }

    #[test]
    fn a_stamp_is_shared_by_copies_and_lost_by_any_change() {
        let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
        assert_eq!(a.stamp(), 0, "a new plane has no stamp");
        let stamp = fresh_stamp();
        a.set_stamp(stamp);
        let b = a.clone();
        assert_eq!(b.stamp(), stamp, "a clone keeps the stamp");
        assert_eq!(a, b);

        let changes: [fn(&mut Plane<u8>); 6] = [
            |p| p[0] = 9,
            |p| p.as_mut_slice()[1] = 9,
            |p| p.extend_from_slice(&[5]),
            |p| p.extend([5u8]),
            |p| p.truncate(1),
            |p| p.clear(),
        ];
        for change in changes {
            let mut c = b.clone();
            change(&mut c);
            assert_eq!(c.stamp(), 0, "every mutable access clears the stamp");
        }
        assert_ne!(fresh_stamp(), stamp);
    }

    #[test]
    fn a_built_plane_holds_what_its_fill_wrote() {
        let p: Plane<u8> = Plane::build(4, |s| s.copy_from_slice(&[4, 3, 2, 1]));
        assert_eq!(p, vec![4, 3, 2, 1]);
        assert_eq!(p.stamp(), 0);
    }

    #[test]
    fn a_unique_plane_writes_in_place() {
        let mut a: Plane<u8> = vec![1, 2, 3, 4].into();
        a.truncate(2);
        a.fill(7);
        assert_eq!(a, vec![7, 7]);
        assert_eq!(a.as_ref(), &[7u8, 7]);
    }

    #[test]
    fn planes_compare_their_bytes_against_every_operand_the_tests_use() {
        let p: Plane<u8> = vec![1, 2, 3].into();
        assert_eq!(p, vec![1, 2, 3]);
        assert_eq!(p, [1u8, 2, 3][..]);
        assert_eq!(p, &[1u8, 2, 3][..]);
        assert_eq!(&p, &[1u8, 2, 3][..]);
        assert_eq!(p, p.clone());
        assert_ne!(p, vec![1, 2]);
    }

    #[test]
    fn an_empty_plane_is_default_and_deref_coerces() {
        let p: Plane<u8> = Plane::default();
        assert_eq!(p.len(), 0);
        let s: &[u8] = &p;
        assert!(s.is_empty());
        let o: Option<Plane<u8>> = Some(vec![5u8].into());
        let d: Option<&[u8]> = o.as_deref();
        assert_eq!(d, Some(&[5u8][..]));
    }

    #[test]
    fn fracturing_a_plane_copies_no_pixels() {
        let bytes = pattern(4 * 128);
        let mut p: Plane<u8> = bytes.clone().into();
        let before = p.allocated_bytes();
        p.fracture(4);
        assert_eq!(p.band_count(), 2, "128 rows of 64-row bands");
        assert_eq!(p.allocated_bytes(), before, "fracturing allocates nothing");
        let joined: Vec<u8> = p.bands().flat_map(|b| b.iter().copied()).collect();
        assert_eq!(joined, bytes, "the bands cover the original allocation");
        assert_eq!(p.as_slice(), bytes.as_slice());
    }

    #[test]
    fn a_band_write_forks_only_its_band() {
        let bytes = pattern(4 * 128);
        let mut a: Plane<u8> = bytes.clone().into();
        a.fracture(4);
        let b = a.clone();
        assert!(a.shares(&b));
        assert!(a.shares_band(0, &b) && a.shares_band(1, &b));

        a.write_band(1).unwrap()[0] = 0xEE;
        assert!(a.shares_band(0, &b), "the untouched band stays shared");
        assert!(!a.shares_band(1, &b), "the written band forked");
        assert!(!a.shares(&b));
        assert_eq!(b.as_slice(), bytes.as_slice(), "the clone keeps its bytes");
        assert_eq!(a.band(0).unwrap(), &bytes[..256]);
        assert_eq!(a.band(1).unwrap()[0], 0xEE);
    }

    #[test]
    fn contiguous_and_band_reads_agree() {
        let bytes = pattern(4 * 128);
        let mut reference: Plane<u8> = bytes.clone().into();
        reference.fracture(4);
        let mut p = reference.clone();

        p.write_band(1).unwrap()[3] = 0xAA;
        reference.as_mut_slice()[256 + 3] = 0xAA;

        let contiguous = p.as_slice().to_vec();
        let banded: Vec<u8> = p.bands().flat_map(|b| b.iter().copied()).collect();
        assert_eq!(contiguous, banded, "contiguous and band reads agree");
        assert_eq!(contiguous, reference.as_slice(), "and match a flat plane");

        let before = materializations();
        let _ = p.as_slice();
        assert_eq!(
            materializations(),
            before,
            "a second contiguous read allocates nothing"
        );
    }

    #[test]
    fn fracture_preserves_every_byte() {
        let bytes = pattern(3 * 200);
        let mut fractured: Plane<u8> = bytes.clone().into();
        fractured.fracture(3);
        let mut flat: Plane<u8> = bytes.into();

        fractured.write_band(0).unwrap()[0] = 0x11;
        flat.as_mut_slice()[0] = 0x11;
        assert_eq!(fractured.as_slice(), flat.as_slice());
    }

    #[test]
    fn a_whole_plane_write_flattens() {
        let bytes = pattern(4 * 128);
        let mut p: Plane<u8> = bytes.clone().into();
        p.fracture(4);
        let holder = p.clone();

        p.as_mut_slice()[0] = 0x99;
        assert_eq!(p.width(), None, "the band view is replaced");
        assert_eq!(p.band_count(), 1);
        assert!(!p.shares(&holder));
        assert_eq!(holder.as_slice(), bytes.as_slice());
        assert_eq!(p[0], 0x99);

        let mut q: Plane<u8> = bytes.clone().into();
        q.fracture(4);
        let holder = q.clone();
        q[0] = 0x77;
        assert_eq!(q.width(), None);
        assert_eq!(holder[0], bytes[0]);
    }

    #[test]
    fn degenerate_planes_fracture_safely() {
        let mut empty: Plane<u8> = Plane::default();
        empty.fracture(4);
        assert_eq!(empty.band_count(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.as_slice(), &[] as &[u8]);

        let mut zero: Plane<u8> = vec![1, 2, 3].into();
        zero.fracture(0);
        assert_eq!(zero.band_count(), 0);
        assert_eq!(zero.len(), 3, "a width-zero plane keeps its bytes");
        assert_eq!(zero.as_slice(), &[1, 2, 3]);

        let bytes = pattern(400);
        let mut partial: Plane<u8> = bytes.clone().into();
        partial.fracture(3);
        assert_eq!(partial.band_count(), 3);
        assert_eq!(partial.band(2).unwrap().len(), 16, "a short last band");

        let before = partial.allocated_bytes();
        partial.fracture(3);
        assert_eq!(partial.allocated_bytes(), before, "same width is a no-op");

        let holder = partial.clone();
        partial.write_band(0).unwrap()[0] = 0x42;
        assert!(
            !partial.shares(&holder),
            "a shared base forks the written band"
        );
        let mark = materializations();
        partial.fracture(2);
        assert!(materializations() > mark, "a new width flattens once");
        assert_eq!(partial.band(0).unwrap()[0], 0x42);
        let joined: Vec<u8> = partial.bands().flat_map(|b| b.iter().copied()).collect();
        assert_eq!(joined.len(), 400);
    }

    #[test]
    fn banded_planes_keep_their_std_traits() {
        let bytes = pattern(4 * 128);
        let mut p: Plane<u8> = bytes.clone().into();
        p.fracture(4);
        assert_eq!(p, bytes.clone());
        assert_eq!(p.as_ref(), bytes.as_slice());
        assert!(format!("{p:?}").contains("Plane"));
        let seen: Vec<u8> = (&p).into_iter().copied().collect();
        assert_eq!(seen, bytes);
        let collected: Plane<u8> = bytes.iter().copied().collect();
        assert_eq!(collected, bytes);
        assert_eq!(Plane::from(bytes.clone()), p);
    }

    #[test]
    fn materialization_counter_is_public_and_counts() {
        let mut p: Plane<u8> = pattern(4 * 128).into();
        p.fracture(4);
        let holder = p.clone();
        let before = materializations();
        let _ = p.as_slice();
        assert_eq!(materializations(), before, "an undiverged plane is free");
        p.write_band(0).unwrap()[0] ^= 0xFF;
        assert!(!p.shares(&holder), "a shared base forks the written band");
        let _ = p.as_slice();
        assert!(materializations() > before, "a diverged plane materializes");
    }

    #[test]
    fn band_shares_reports_every_band() {
        let mut p: Plane<u8> = pattern(4 * 128).into();
        p.fracture(4);
        let q = p.clone();
        assert!(p.shares_band(0, &q) && p.shares_band(1, &q));
        p.write_band(0).unwrap()[0] ^= 1;
        assert!(!p.shares_band(0, &q));
        assert!(p.shares_band(1, &q));
    }

    #[test]
    fn write_bands_serves_a_unique_base() {
        let mut p: Plane<u8> = pattern(4 * 128).into();
        p.fracture(4);
        let mut slices = p.write_bands(&[0]).expect("a unique base serves its bands");
        assert_eq!(slices.len(), 1);
        slices[0][0] = 0xAB;
        assert_eq!(p.band(0).unwrap()[0], 0xAB, "the write landed");
        assert_eq!(p.as_slice()[0], 0xAB, "and the plane stays readable");
    }
}
