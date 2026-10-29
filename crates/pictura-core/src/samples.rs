//! Native-depth sample storage and the shared narrowing/widening rules
//! (`psd-bit-depth`). Split from `lib.rs` for the file-size cap.

use crate::{BitDepth, Plane};

/// A planar buffer of document-depth samples: `u8`, `u16`, or `f32` per channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    U8(Plane<u8>),
    U16(Plane<u16>),
    F32(Plane<f32>),
}

impl Samples {
    /// The bit depth this store holds.
    pub fn depth(&self) -> BitDepth {
        match self {
            Samples::U8(_) => BitDepth::Eight,
            Samples::U16(_) => BitDepth::Sixteen,
            Samples::F32(_) => BitDepth::ThirtyTwo,
        }
    }

    /// Number of samples (not pixels).
    pub fn len(&self) -> usize {
        match self {
            Samples::U8(v) => v.len(),
            Samples::U16(v) => v.len(),
            Samples::F32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Decode a PSD native byte image into samples: big-endian `u16` at depth 16,
    /// IEEE-754 big-endian `f32` at depth 32; depth 1 and 8 bytes stay bytes.
    pub fn from_bytes(bytes: &[u8], depth: BitDepth) -> Samples {
        match depth {
            BitDepth::Sixteen => Samples::U16(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| u16::from_be_bytes(*b))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            BitDepth::ThirtyTwo => Samples::F32(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_be_bytes(*b))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            _ => Samples::U8(bytes.to_vec().into()),
        }
    }

    /// Encode samples back to the PSD native byte image; the inverse of
    /// [`Samples::from_bytes`].
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Samples::U8(v) => v.to_vec(),
            Samples::U16(v) => v.iter().flat_map(|s| s.to_be_bytes()).collect(),
            Samples::F32(v) => v.iter().flat_map(|s| s.to_be_bytes()).collect(),
        }
    }

    /// Narrow to 8-bit: depth 16 keeps the high byte, depth 32 is
    /// `clamp(trunc(v * 256), 0, 255)`, depth 1/8 is the bytes unchanged.
    pub fn narrow_to_u8(&self) -> Vec<u8> {
        match self {
            Samples::U8(v) => v.to_vec(),
            Samples::U16(v) => v.iter().map(|&s| (s >> 8) as u8).collect(),
            Samples::F32(v) => v
                .iter()
                .map(|&s| (s * 256.0).trunc().clamp(0.0, 255.0) as u8)
                .collect(),
        }
    }

    /// Quantize to 16-bit: depth 8 widens `v * 257`, depth 16 is unchanged, and
    /// depth 32 clamps to `[0, 1]` then rounds `v * 65535` (NaN maps to `0`).
    pub fn to_u16(&self) -> Vec<u16> {
        match self {
            Samples::U8(v) => v.iter().map(|&s| s as u16 * 257).collect(),
            Samples::U16(v) => v.to_vec(),
            Samples::F32(v) => v
                .iter()
                .map(|&s| (s.clamp(0.0, 1.0) * 65535.0).round() as u16)
                .collect(),
        }
    }

    /// Widen an 8-bit plane to `depth`: `v * 257` at depth 16, `v / 255` scaled
    /// into `[0, 1]` at depth 32, the bytes unchanged at depth 1/8.
    pub fn widen_from_u8(bytes: &[u8], depth: BitDepth) -> Samples {
        match depth {
            BitDepth::Sixteen => Samples::U16(
                bytes
                    .iter()
                    .map(|&v| v as u16 * 257)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            BitDepth::ThirtyTwo => Samples::F32(
                bytes
                    .iter()
                    .map(|&v| v as f32 / 255.0)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            _ => Samples::U8(bytes.to_vec().into()),
        }
    }

    /// A cloned sub-range, or `None` when it is out of bounds. Views one channel
    /// plane of a flat multi-plane store.
    pub fn slice(&self, range: std::ops::Range<usize>) -> Option<Samples> {
        if range.start > range.end || range.end > self.len() {
            return None;
        }
        Some(match self {
            Samples::U8(v) => Samples::U8(v[range].to_vec().into()),
            Samples::U16(v) => Samples::U16(v[range].to_vec().into()),
            Samples::F32(v) => Samples::F32(v[range].to_vec().into()),
        })
    }

    /// The raw bytes when the store is 8-bit, including the packed depth-1
    /// Bitmap plane; `None` for a native `u16`/`f32` store.
    pub fn as_u8(&self) -> Option<&[u8]> {
        match self {
            Samples::U8(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

/// A scalar sample that converts to and from the `[0, 1]` unit domain.
///
/// `to_unit` is total: an `f32` below `0.0` or above `1.0` clamps into the
/// interval and `NaN` maps to `0.0`, so a finite store cannot feed a kernel a
/// non-finite value. `from_unit` rounds at the store's own scale (`255` for
/// `u8`, `65535` for `u16`); an `f32` store keeps the unit value as-is so a
/// value above `1.0` stays representable.
pub trait Sample: Copy {
    fn to_unit(self) -> f64;
    fn from_unit(v: f64) -> Self;

    /// The store's samples as a mutable slice, when the store holds samples of
    /// this width; `None` for a store of another width.
    ///
    /// A tool that edits a store holds one plane per channel at once, so it
    /// needs the typed view; the default refuses rather than guessing.
    fn as_slice_mut(_store: &mut Samples) -> Option<&mut [Self]> {
        None
    }
}

impl Sample for u8 {
    fn to_unit(self) -> f64 {
        self as f64 / 255.0
    }

    fn from_unit(v: f64) -> Self {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    fn as_slice_mut(store: &mut Samples) -> Option<&mut [u8]> {
        match store {
            Samples::U8(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }
}

impl Sample for u16 {
    fn to_unit(self) -> f64 {
        self as f64 / 65535.0
    }

    fn from_unit(v: f64) -> Self {
        (v.clamp(0.0, 1.0) * 65535.0).round() as u16
    }

    fn as_slice_mut(store: &mut Samples) -> Option<&mut [u16]> {
        match store {
            Samples::U16(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }
}

impl Sample for f32 {
    fn to_unit(self) -> f64 {
        let v = self as f64;
        if v.is_nan() {
            0.0
        } else {
            v.clamp(0.0, 1.0)
        }
    }

    fn from_unit(v: f64) -> Self {
        v as f32
    }

    fn as_slice_mut(store: &mut Samples) -> Option<&mut [f32]> {
        match store {
            Samples::F32(v) => Some(v.as_mut_slice()),
            _ => None,
        }
    }
}

impl Samples {
    /// Map each pixel's three color planes (R, G, B) through `f` in the unit
    /// domain, leaving any alpha plane and trailing extra channels untouched.
    ///
    /// `width * height * channels` must fit the store; `channels` is 3 or 4.
    pub fn map_color_planes(
        &mut self,
        width: usize,
        height: usize,
        channels: u8,
        mut f: impl FnMut([f64; 3]) -> [f64; 3],
    ) {
        let n = width * height;
        match self {
            Samples::U8(v) => map_pixels(v, n, channels, &mut f),
            Samples::U16(v) => map_pixels(v, n, channels, &mut f),
            Samples::F32(v) => map_pixels(v, n, channels, &mut f),
        }
    }
}

fn map_pixels<T: Sample>(
    data: &mut [T],
    n: usize,
    channels: u8,
    f: &mut impl FnMut([f64; 3]) -> [f64; 3],
) {
    debug_assert!(channels == 3 || channels == 4);
    let (r, rest) = data.split_at_mut(n);
    let (g, rest) = rest.split_at_mut(n);
    let (b, _) = rest.split_at_mut(n);
    for i in 0..n {
        let out = f([r[i].to_unit(), g[i].to_unit(), b[i].to_unit()]);
        r[i] = T::from_unit(out[0]);
        g[i] = T::from_unit(out[1]);
        b[i] = T::from_unit(out[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_units_convert_at_the_store_scale() {
        assert_eq!(0u8.to_unit(), 0.0);
        assert_eq!(255u8.to_unit(), 1.0);
        assert_eq!(u8::from_unit(0.5), 128);
        assert_eq!(32768u16.to_unit(), 32768.0 / 65535.0);
        assert_eq!(u16::from_unit(1.0), 65535);
        assert_eq!(u16::from_unit(0.5), 32768);
    }

    #[test]
    fn f32_sample_clamps_and_maps_nan() {
        assert_eq!((-0.5f32).to_unit(), 0.0);
        assert_eq!(1.5f32.to_unit(), 1.0);
        assert_eq!(f32::NAN.to_unit(), 0.0);
        assert_eq!(0.25f32.to_unit(), 0.25);
        assert_eq!(f32::from_unit(1.5), 1.5);
    }

    #[test]
    fn map_color_planes_leaves_alpha_untouched() {
        let mut store = Samples::U8(vec![1, 2, 3, 4, 5, 6, 7, 8].into());
        store.map_color_planes(2, 1, 4, |p| p.map(|v| 1.0 - v));
        let Samples::U8(v) = store else { panic!() };
        assert_eq!(v, vec![254, 253, 252, 251, 250, 249, 7, 8]);
    }

    #[test]
    fn narrow_u16_matches_the_byte_rule() {
        let store = Samples::U16(vec![0, 255, 256, 32768, 65535].into());
        assert_eq!(store.narrow_to_u8(), vec![0, 0, 1, 128, 255]);
    }

    #[test]
    fn narrow_f32_matches_the_clipped_rule() {
        let store = Samples::F32(vec![1.5, -0.5, 0.0, 0.5].into());
        assert_eq!(store.narrow_to_u8(), vec![255, 0, 0, 128]);
    }

    #[test]
    fn to_u16_quantizes_and_widens() {
        assert_eq!(
            Samples::F32(vec![0.0, 0.5, 1.0, 1.5, -0.5].into()).to_u16(),
            vec![0, 32768, 65535, 65535, 0],
            "clamp then round v * 65535"
        );
        assert_eq!(
            Samples::U8(vec![0, 1, 255].into()).to_u16(),
            vec![0, 257, 65535],
            "8-bit widens v * 257"
        );
        assert_eq!(
            Samples::U16(vec![5, 60000].into()).to_u16(),
            vec![5, 60000],
            "16-bit is unchanged"
        );
    }

    #[test]
    fn widen_u8_to_u16_is_257_times() {
        assert_eq!(
            Samples::widen_from_u8(&[0, 1, 255], BitDepth::Sixteen),
            Samples::U16(vec![0, 257, 65535].into())
        );
    }

    #[test]
    fn widen_u8_to_f32_scales_to_unit_range() {
        assert_eq!(
            Samples::widen_from_u8(&[0, 1, 255], BitDepth::ThirtyTwo),
            Samples::F32(vec![0.0, 1.0 / 255.0, 1.0].into())
        );
    }

    #[test]
    fn len_depth_and_empty_follow_the_variant() {
        assert_eq!(Samples::U8(vec![0; 3].into()).depth(), BitDepth::Eight);
        assert_eq!(Samples::U16(vec![0; 3].into()).depth(), BitDepth::Sixteen);
        assert_eq!(
            Samples::F32(vec![0.0; 3].into()).depth(),
            BitDepth::ThirtyTwo
        );
        assert_eq!(Samples::U16(vec![0; 3].into()).len(), 3);
        assert!(Samples::U8(Vec::new().into()).is_empty());
        assert!(!Samples::F32(vec![0.0].into()).is_empty());
    }

    #[test]
    fn encode_inverts_decode_byte_exactly() {
        let u16_bytes: Vec<u8> = (0..40u16)
            .flat_map(|v| v.wrapping_mul(1031).to_be_bytes())
            .collect();
        assert_eq!(
            Samples::from_bytes(&u16_bytes, BitDepth::Sixteen).to_bytes(),
            u16_bytes
        );
        let f32_bytes: Vec<u8> = [0.0f32, 0.5, 1.0, -0.5, 1.5]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect();
        assert_eq!(
            Samples::from_bytes(&f32_bytes, BitDepth::ThirtyTwo).to_bytes(),
            f32_bytes
        );
    }

    #[test]
    fn a_sample_store_clone_shares_its_allocation() {
        let stores = [
            Samples::U8(vec![1, 2, 3].into()),
            Samples::U16(vec![1, 2, 3].into()),
            Samples::F32(vec![1.0, 2.0, 3.0].into()),
        ];
        for store in stores {
            let clone = store.clone();
            let shares = match (&store, &clone) {
                (Samples::U8(a), Samples::U8(b)) => a.shares(b),
                (Samples::U16(a), Samples::U16(b)) => a.shares(b),
                (Samples::F32(a), Samples::F32(b)) => a.shares(b),
                _ => panic!("clone changed the variant"),
            };
            assert!(shares, "a clone must only bump the refcount");
            assert_eq!(store, clone);
        }
    }

    #[test]
    fn a_shared_sample_store_forks_on_write() {
        let mut a = Samples::U16(vec![1, 2, 3].into());
        let b = a.clone();
        let Samples::U16(av) = &mut a else { panic!() };
        av.as_mut_slice()[0] = 9;
        let Samples::U16(bv) = &b else { panic!() };
        assert!(!av.shares(bv), "a write must fork the shared store");
        assert_eq!(a, Samples::U16(vec![9, 2, 3].into()));
        assert_eq!(b, Samples::U16(vec![1, 2, 3].into()));
    }

    #[test]
    fn a_unique_sample_store_writes_in_place() {
        let mut a = Samples::U16(vec![1, 2, 3].into());
        let Samples::U16(v) = &mut a else { panic!() };
        let before = v.as_slice().as_ptr();
        v.as_mut_slice()[0] = 9;
        assert_eq!(
            v.as_slice().as_ptr(),
            before,
            "a unique store keeps its allocation"
        );
    }

    #[test]
    fn slice_is_bounds_checked() {
        let store = Samples::U16(vec![1, 2, 3, 4].into());
        assert_eq!(store.slice(1..3), Some(Samples::U16(vec![2, 3].into())));
        assert_eq!(store.slice(2..5), None);
        assert_eq!(Samples::U8(vec![1, 2].into()).as_u8(), Some(&[1u8, 2][..]));
        assert_eq!(Samples::F32(vec![0.0].into()).as_u8(), None);
    }
}
