//! Refcounted copy-on-write pixel planes.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// One image plane behind a refcount.
///
/// Cloning the owning document is a refcount bump, and a write forks only the
/// plane it touches, so a stroke or a history snapshot stops paying for pixels
/// that have not changed. A plane that is uniquely held keeps its allocation
/// and mutates in place exactly as a `Vec` did; only a shared plane copies on
/// first write.
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
    data: Arc<[T]>,
    stamp: u64,
    ident: AtomicU64,
}

impl<T> Plane<T> {
    fn wrap(data: Arc<[T]>) -> Self {
        Self {
            data,
            stamp: 0,
            ident: AtomicU64::new(0),
        }
    }
}

impl<T> Clone for Plane<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            stamp: self.stamp,
            ident: AtomicU64::new(self.ident.load(Ordering::Relaxed)),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Plane<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Plane").field(&self.data).finish()
    }
}

impl<T: PartialEq> PartialEq for Plane<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: Eq> Eq for Plane<T> {}

impl<T> Deref for Plane<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.data
    }
}

impl<T: Clone> DerefMut for Plane<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.stamp = 0;
        *self.ident.get_mut() = 0;
        Arc::make_mut(&mut self.data)
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

impl<T> Plane<T> {
    /// The plane's samples, exactly as a `Vec::as_slice` would hand them over.
    pub fn as_slice(&self) -> &[T] {
        &self.data
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

    /// Whether `self` and `other` are one allocation.
    pub fn shares(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
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
    /// The plane's samples, mutable; a shared plane forks first.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// Append `more`, mirroring `Vec::extend_from_slice`.
    ///
    /// ponytail: a slice behind a refcount cannot grow in place, so this
    /// rebuilds; the callers build small buffers.
    pub fn extend_from_slice(&mut self, more: &[T])
    where
        T: Clone,
    {
        let mut data = self.data.to_vec();
        data.extend_from_slice(more);
        *self = Self::wrap(Arc::from(data));
    }

    /// Extend from an iterator, mirroring `Vec::extend`.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I)
    where
        T: Clone,
    {
        let mut data = self.data.to_vec();
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
            let mut data = self.data.to_vec();
            data.truncate(len);
            *self = Self::wrap(Arc::from(data));
        }
    }
}

impl<T> AsRef<[T]> for Plane<T> {
    fn as_ref(&self) -> &[T] {
        &self.data
    }
}

#[cfg(test)]
pub(crate) fn shares(a: &Plane<u8>, b: &Plane<u8>) -> bool {
    a.shares(b)
}

impl<'a, T> IntoIterator for &'a Plane<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for Plane<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.as_ref() == other.as_slice()
    }
}

impl<T: PartialEq> PartialEq<[T]> for Plane<T> {
    fn eq(&self, other: &[T]) -> bool {
        self.as_ref() == other
    }
}

impl<T: PartialEq, const N: usize> PartialEq<[T; N]> for Plane<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        self.as_ref() == other
    }
}

impl<T: PartialEq> PartialEq<&[T]> for Plane<T> {
    fn eq(&self, other: &&[T]) -> bool {
        self.as_ref() == *other
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
}
