//! Uploads kept on the device between composites.
//!
//! A layer whose pixels did not change — a blend, opacity or visibility edit,
//! or simply the next composite — would otherwise be assembled and uploaded
//! again every time. A source upload is keyed by the plane stamps, the layer
//! rect, the document size, and the grayscale flag, so a region composite can
//! reuse the whole-document upload a previous full composite kept; a region
//! build that misses is not retained. Coverage is retained only for a
//! whole-document composite of a layer with an enabled data-carrying raster
//! mask; a vector-mask fill carries no stamp, so its coverage is rebuilt for
//! each composite. An upload keyed by an unstamped plane is never cached.

use std::collections::{HashMap, VecDeque};

use pictura_core::{Layer, PsdRect};

use super::assemble::{mask_has_data, mask_influence_rect, Region};

/// A rectangle as `(left, top, right, bottom)`.
type Edges = (i32, i32, i32, i32);

/// What an upload was built from; equal keys mean byte-identical uploads.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) enum UploadKey {
    Source {
        /// `(stamp, len)` of channels 0, 1, 2 and -1, `None` where absent.
        planes: [Option<(u64, usize)>; 4],
        /// The raw layer rect, which determines the row mapping.
        rect: Edges,
        /// The document the upload is clipped to.
        doc: (u32, u32),
        gray: bool,
    },
    Mask {
        region: (u32, u32, u32, u32),
        influence: Edges,
        /// The sampled raster mask's `(stamp, rect, default, density,
        /// feather bits)`.
        sampled: (u64, Edges, u8, u8, u64),
    },
    Raster {
        /// A content hash for a cached CPU raster (a type raster, a smart
        /// source, or a fill). `0` is never cached.
        content: u64,
        rect: Edges,
        doc: (u32, u32),
        gray: bool,
    },
}

fn rect_key(r: PsdRect) -> Edges {
    (r.left, r.top, r.right, r.bottom)
}

fn region_key(r: Region) -> (u32, u32, u32, u32) {
    (r.x0, r.y0, r.w, r.h)
}

/// The key of `layer`'s whole-document source upload over `doc`, or `None` when
/// a plane it reads carries no stamp.
pub(super) fn source_key(layer: &Layer, doc: (u32, u32), gray: bool) -> Option<UploadKey> {
    let mut planes = [None; 4];
    for (slot, id) in planes.iter_mut().zip([0i16, 1, 2, -1]) {
        if let Some(channel) = layer.channels.iter().find(|c| c.id == id) {
            let stamp = channel.data.stamp();
            if stamp == 0 {
                return None;
            }
            *slot = Some((stamp, channel.data.len()));
        }
    }
    Some(UploadKey::Source {
        planes,
        rect: rect_key(layer.rect),
        doc,
        gray,
    })
}

/// The key of `layer`'s coverage upload over `region`, or `None` when no
/// enabled data-carrying mask is sampled, a vector mask carries a fill, or the
/// mask plane is unstamped. Constant coverage is rebuilt row-wise and never
/// cached.
pub(super) fn mask_key(region: Region, layer: &Layer) -> Option<UploadKey> {
    if layer.vector_mask.as_ref().is_some_and(|v| v.has_fill()) || !mask_has_data(layer) {
        return None;
    }
    let mask = layer.mask.as_ref()?;
    let stamp = mask.data.as_ref()?.stamp();
    if stamp == 0 {
        return None;
    }
    Some(UploadKey::Mask {
        region: region_key(region),
        influence: mask_influence_rect(region, layer),
        sampled: (
            stamp,
            rect_key(mask.rect),
            mask.default_color,
            mask.density,
            mask.feather_bits,
        ),
    })
}

/// Bytes the kept uploads may hold together.
///
/// ponytail: one budget for every device; on an integrated GPU it is system
/// RAM. Two gigabytes keeps a 267-megapixel layer and its coverage resident;
/// size it from the adapter's memory if that matters.
const BUDGET: u64 = 2 << 30;

/// The most uploads the store may hold, bounding tiny-document entries that
/// would otherwise only be stopped by the byte budget.
const MAX_ENTRIES: usize = 1024;

/// Kept uploads in insertion order, oldest first.
pub(super) struct Resident<V = wgpu::Buffer> {
    entries: HashMap<UploadKey, (V, u64)>,
    order: VecDeque<UploadKey>,
    bytes: u64,
}

impl<V> Default for Resident<V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
        }
    }
}

impl<V: Clone> Resident<V> {
    /// The upload kept under `key`, if any. Does not change the store.
    pub(super) fn get(&self, key: &UploadKey) -> Option<V> {
        self.entries.get(key).map(|(value, _)| value.clone())
    }

    /// Keep `value` under `key`, dropping the oldest uploads past the byte
    /// budget or the entry cap (never the one just kept), and return the
    /// post-eviction `(entries, bytes)`. An upload larger than the budget is
    /// used by the composite but not retained.
    pub(super) fn put(&mut self, key: UploadKey, value: V, size: u64) -> (usize, u64) {
        if size > BUDGET {
            return (self.entries.len(), self.bytes);
        }
        if let Some((_, old)) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(old);
            self.order.retain(|k| *k != key);
        }
        self.bytes = self.bytes.saturating_add(size);
        self.entries.insert(key.clone(), (value, size));
        self.order.push_back(key);
        while (self.bytes > BUDGET || self.entries.len() > MAX_ENTRIES) && self.entries.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some((_, size)) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(size);
            }
        }
        (self.entries.len(), self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{fresh_stamp, Channel, LayerMask};

    fn region() -> Region {
        Region {
            x0: 0,
            y0: 0,
            w: 4,
            h: 2,
        }
    }

    fn layer() -> Layer {
        Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 4,
            },
            channels: (-1..3)
                .map(|id| Channel {
                    id,
                    data: vec![id as u8; 8].into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn stamp_all(layer: &mut Layer) {
        for c in &mut layer.channels {
            c.data.set_stamp(fresh_stamp());
        }
    }

    fn key(i: u64) -> UploadKey {
        UploadKey::Source {
            planes: [Some((i, 0)), None, None, None],
            rect: (0, 0, 4, 2),
            doc: (4, 2),
            gray: false,
        }
    }

    #[test]
    fn an_unstamped_plane_is_never_cached() {
        let mut l = layer();
        assert_eq!(source_key(&l, (4, 2), false), None);
        stamp_all(&mut l);
        l.channels[2].data.as_mut_slice()[0] = 9;
        assert_eq!(
            source_key(&l, (4, 2), false),
            None,
            "a write clears the stamp"
        );
    }

    #[test]
    fn a_source_key_follows_its_planes_and_geometry() {
        let mut l = layer();
        stamp_all(&mut l);
        let key = source_key(&l, (4, 2), false).expect("stamped");
        assert_eq!(source_key(&l.clone(), (4, 2), false), Some(key.clone()));
        assert_ne!(
            source_key(&l, (8, 2), false),
            Some(key.clone()),
            "document size is in the key"
        );
        assert_ne!(source_key(&l, (4, 2), true), Some(key.clone()));
        let mut moved = l.clone();
        moved.rect.left += 1;
        assert_ne!(
            source_key(&moved, (4, 2), false),
            Some(key.clone()),
            "the raw layer rect is in the key"
        );
        l.channels[0].data.set_stamp(fresh_stamp());
        assert_ne!(
            source_key(&l, (4, 2), false),
            Some(key),
            "new bytes, new key"
        );
    }

    #[test]
    fn a_mask_key_needs_a_stamped_plane_only_when_it_samples_one() {
        let mut l = layer();
        assert_eq!(
            mask_key(region(), &l),
            None,
            "constant coverage is rebuilt, not cached"
        );
        l.mask = Some(LayerMask {
            rect: l.rect,
            data: Some(vec![7u8; 8].into()),
            ..Default::default()
        });
        assert_eq!(
            mask_key(region(), &l),
            None,
            "an unstamped mask is not cached"
        );
        let stamp = fresh_stamp();
        l.mask
            .as_mut()
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .set_stamp(stamp);
        let key = mask_key(region(), &l).expect("stamped mask");
        assert_ne!(
            mask_key(Region { x0: 1, ..region() }, &l),
            Some(key.clone()),
            "region is in the key"
        );
        let mut moved_layer = l.clone();
        moved_layer.rect.left += 1;
        assert_ne!(
            mask_key(region(), &moved_layer),
            Some(key.clone()),
            "the influence rect is in the key"
        );
        let mut moved_mask = l.clone();
        moved_mask.mask.as_mut().unwrap().rect.left += 1;
        assert_ne!(
            mask_key(region(), &moved_mask),
            Some(key.clone()),
            "the mask rect is in the key"
        );
        l.mask.as_mut().unwrap().default_color = 255;
        assert_ne!(mask_key(region(), &l), Some(key));
        let key = mask_key(region(), &l);
        l.mask.as_mut().unwrap().density = 128;
        assert_ne!(mask_key(region(), &l), key, "density is in the key");
        let key = mask_key(region(), &l);
        l.mask.as_mut().unwrap().set_feather(2.0);
        assert_ne!(mask_key(region(), &l), key, "feather is in the key");
        l.mask.as_mut().unwrap().disabled = true;
        assert_eq!(
            mask_key(region(), &l),
            None,
            "a disabled mask is constant coverage"
        );
    }

    #[test]
    fn get_returns_a_kept_upload() {
        let mut r: Resident<u64> = Resident::default();
        r.put(key(1), 42, 8);
        assert_eq!(r.get(&key(1)), Some(42));
        assert_eq!(r.get(&key(2)), None);
    }

    #[test]
    fn get_leaves_the_store_unchanged() {
        let mut r: Resident<u64> = Resident::default();
        r.put(key(1), 42, 8);
        let before = (r.entries.len(), r.bytes);
        assert_eq!(r.get(&key(1)), Some(42));
        assert_eq!((r.entries.len(), r.bytes), before);
    }

    #[test]
    fn put_evicts_past_the_byte_budget() {
        let mut r: Resident<u64> = Resident::default();
        let half = BUDGET / 2 + 1;
        assert_eq!(r.put(key(1), 1, half), (1, half));
        assert_eq!(r.put(key(2), 2, half), (1, half));
        assert_eq!(r.get(&key(1)), None);
        assert_eq!(r.get(&key(2)), Some(2));
    }

    #[test]
    fn put_refuses_an_upload_over_the_budget() {
        let mut r: Resident<u64> = Resident::default();
        assert_eq!(r.put(key(1), 1, BUDGET + 1), (0, 0));
        assert_eq!(r.get(&key(1)), None);
    }

    #[test]
    fn put_keeps_the_entry_count_bounded() {
        let mut r: Resident<u64> = Resident::default();
        for i in 0..(MAX_ENTRIES as u64 + 10) {
            r.put(key(i), i, 1);
        }
        assert_eq!(r.entries.len(), MAX_ENTRIES);
        assert_eq!(r.get(&key(0)), None);
        assert_eq!(
            r.get(&key(MAX_ENTRIES as u64 + 9)),
            Some(MAX_ENTRIES as u64 + 9)
        );
    }

    #[test]
    fn put_replacing_a_key_keeps_one_entry() {
        let mut r: Resident<u64> = Resident::default();
        r.put(key(1), 1, 8);
        r.put(key(1), 2, 4);
        assert_eq!((r.entries.len(), r.bytes), (1, 4));
        assert_eq!(r.order.len(), 1, "a replace leaves one queue entry");
        assert_eq!(r.get(&key(1)), Some(2));
    }

    #[test]
    fn put_reports_post_eviction_stats() {
        let mut r: Resident<u64> = Resident::default();
        let half = BUDGET / 2 + 1;
        assert_eq!(r.put(key(1), 1, half), (1, half));
        assert_eq!(r.put(key(2), 2, half), (1, half));
        assert_eq!(r.put(key(3), 3, 4), (2, half + 4));
    }
}
