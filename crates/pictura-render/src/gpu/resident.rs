//! Uploads kept on the device between composites.
//!
//! A layer whose pixels did not change — a blend, opacity or visibility edit,
//! or simply the next composite — would otherwise be assembled and uploaded
//! again every time. An upload is keyed by the plane stamps it was built from:
//! a nonzero stamp names one set of bytes and any write clears it, so a key can
//! only match bytes identical to the cached ones. An unstamped plane is never
//! cached.

use pictura_core::{Layer, PsdRect};

use super::assemble::{mask_has_data, mask_influence_rect, Region};

/// A rectangle as `(left, top, right, bottom)`.
type Edges = (i32, i32, i32, i32);

/// What an upload was built from; equal keys mean byte-identical uploads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum UploadKey {
    Source {
        /// `(stamp, len)` of channels 0, 1, 2 and -1, `None` where absent.
        planes: [Option<(u64, usize)>; 4],
        rect: Edges,
        region: (u32, u32, u32, u32),
        gray: bool,
    },
    Mask {
        region: (u32, u32, u32, u32),
        influence: Edges,
        /// The sampled raster mask's `(stamp, rect, default, density,
        /// feather bits)`; `None` for the constant coverage of a mask with
        /// nothing to sample.
        sampled: Option<(u64, Edges, u8, u8, u64)>,
    },
}

fn rect_key(r: PsdRect) -> Edges {
    (r.left, r.top, r.right, r.bottom)
}

fn region_key(r: Region) -> (u32, u32, u32, u32) {
    (r.x0, r.y0, r.w, r.h)
}

/// The key of `layer`'s source upload over `region`, or `None` when a plane it
/// reads carries no stamp.
pub(super) fn source_key(region: Region, layer: &Layer, gray: bool) -> Option<UploadKey> {
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
        region: region_key(region),
        gray,
    })
}

/// The key of `layer`'s coverage upload over `region`, or `None` when it reads
/// an unstamped mask plane or a vector mask (which carries no stamp).
pub(super) fn mask_key(region: Region, layer: &Layer) -> Option<UploadKey> {
    if layer.vector_mask.as_ref().is_some_and(|v| v.has_fill()) {
        return None;
    }
    let sampled = if mask_has_data(layer) {
        let mask = layer.mask.as_ref()?;
        let stamp = mask.data.as_ref()?.stamp();
        if stamp == 0 {
            return None;
        }
        Some((
            stamp,
            rect_key(mask.rect),
            mask.default_color,
            mask.density,
            mask.feather_bits,
        ))
    } else {
        None
    };
    Some(UploadKey::Mask {
        region: region_key(region),
        influence: mask_influence_rect(region, layer),
        sampled,
    })
}

/// Bytes the kept uploads may hold together.
///
/// ponytail: one budget for every device; on an integrated GPU it is system
/// RAM. Two gigabytes keeps a 267-megapixel layer and its coverage resident;
/// size it from the adapter's memory if that matters.
const BUDGET: u64 = 2 << 30;

/// Kept uploads, least recently used first.
#[derive(Default)]
pub(super) struct Resident {
    entries: Vec<(UploadKey, wgpu::Buffer)>,
}

impl Resident {
    /// The upload kept under `key`, marked most recently used.
    pub(super) fn get(&mut self, key: &UploadKey) -> Option<wgpu::Buffer> {
        let at = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(at);
        let buffer = entry.1.clone();
        self.entries.push(entry);
        Some(buffer)
    }

    /// Keep `buffer` under `key`, dropping the least recently used uploads
    /// past the budget (never the one just kept).
    pub(super) fn put(&mut self, key: UploadKey, buffer: wgpu::Buffer) {
        self.entries.retain(|(k, _)| *k != key);
        self.entries.push((key, buffer));
        while self.entries.len() > 1
            && self.entries.iter().map(|(_, b)| b.size()).sum::<u64>() > BUDGET
        {
            self.entries.remove(0);
        }
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

    #[test]
    fn an_unstamped_plane_is_never_cached() {
        let mut l = layer();
        assert_eq!(source_key(region(), &l, false), None);
        stamp_all(&mut l);
        l.channels[2].data.as_mut_slice()[0] = 9;
        assert_eq!(
            source_key(region(), &l, false),
            None,
            "a write clears the stamp"
        );
    }

    #[test]
    fn a_source_key_follows_its_planes_and_geometry() {
        let mut l = layer();
        stamp_all(&mut l);
        let key = source_key(region(), &l, false).expect("stamped");
        assert_eq!(source_key(region(), &l.clone(), false), Some(key.clone()));
        let moved = Region { x0: 1, ..region() };
        assert_ne!(source_key(moved, &l, false), Some(key.clone()));
        assert_ne!(source_key(region(), &l, true), Some(key.clone()));
        l.channels[0].data.set_stamp(fresh_stamp());
        assert_ne!(
            source_key(region(), &l, false),
            Some(key),
            "new bytes, new key"
        );
    }

    #[test]
    fn a_mask_key_needs_a_stamped_plane_only_when_it_samples_one() {
        let mut l = layer();
        assert!(mask_key(region(), &l).is_some(), "constant coverage");
        l.mask = Some(LayerMask {
            rect: l.rect,
            data: Some(vec![7u8; 8].into()),
            ..Default::default()
        });
        assert_eq!(mask_key(region(), &l), None);
        let stamp = fresh_stamp();
        l.mask
            .as_mut()
            .unwrap()
            .data
            .as_mut()
            .unwrap()
            .set_stamp(stamp);
        let key = mask_key(region(), &l).expect("stamped mask");
        l.mask.as_mut().unwrap().default_color = 255;
        assert_ne!(mask_key(region(), &l), Some(key));
        let key = mask_key(region(), &l);
        l.mask.as_mut().unwrap().density = 128;
        assert_ne!(mask_key(region(), &l), key, "density is in the key");
        let key = mask_key(region(), &l);
        l.mask.as_mut().unwrap().set_feather(2.0);
        assert_ne!(mask_key(region(), &l), key, "feather is in the key");
    }
}
