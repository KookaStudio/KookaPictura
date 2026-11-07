//! Growing a pixel layer's rectangle (#1208).
//!
//! A pixel layer stores its planes at `layer.rect`. An edit that reaches
//! outside that rect grows it here: the new area is transparent (zero), the
//! bytes are reserved through the memory governor first, and the allocation is
//! fallible so a full host gets the typed refusal instead of an abort.

use crate::memory::{self, MemoryError};
use crate::{layer_pixel_locked, layer_transparency_locked, Layer, PsdRect, Samples};

/// Grow `layer`'s rect to `rect`, padding every plane with zeroes.
///
/// Returns `Ok(false)` when nothing needs to change (the rect is already a
/// superset, the layer is a group/adjustment, or it is locked), and the typed
/// memory error when the added bytes do not fit the governor's room or the
/// allocator refuses them. `purpose` names the edit in the refusal.
pub fn grow_layer_to(
    layer: &mut Layer,
    rect: PsdRect,
    purpose: &'static str,
) -> Result<bool, MemoryError> {
    let old = layer.rect;
    if rect == old || rect.is_empty() || layer.is_group || layer.adjustment.is_some() {
        return Ok(false);
    }
    if !old.is_empty() && old.union(rect) != rect {
        return Ok(false);
    }
    if layer_pixel_locked(layer) || layer_transparency_locked(layer) {
        return Ok(false);
    }
    let old_w = old.width().max(0) as usize;
    let old_h = old.height().max(0) as usize;
    let old_n = old_w * old_h;
    let new_w = rect.width() as usize;
    let new_h = rect.height() as usize;
    let new_n = new_w.checked_mul(new_h).ok_or(MemoryError::Insufficient {
        purpose,
        bytes: u64::MAX,
    })?;
    let (x0, y0) = if old.is_empty() {
        (0, 0)
    } else {
        (
            (old.left - rect.left).max(0) as usize,
            (old.top - rect.top).max(0) as usize,
        )
    };
    let source_planes = layer.source_channels.as_ref().map_or(0, |store| {
        if store.rect == old {
            store
                .planes
                .iter()
                .filter(|(_, samples)| samples.len() == old_n)
                .count()
        } else {
            0
        }
    });
    let planes = layer
        .channels
        .iter()
        .filter(|channel| channel.data.len() == old_n)
        .count()
        + source_planes;
    let sample_bytes = layer
        .source_channels
        .as_ref()
        .map_or(1, |store| match store.depth {
            crate::BitDepth::Sixteen => 2,
            crate::BitDepth::ThirtyTwo => 4,
            _ => 1,
        });
    let added = u64::try_from(new_n.saturating_sub(old_n))
        .unwrap_or(u64::MAX)
        .saturating_mul(planes as u64)
        .saturating_mul(sample_bytes as u64);
    let _held = memory::governor().reserve(added, purpose)?;

    for channel in &mut layer.channels {
        if channel.data.len() != old_n {
            continue;
        }
        channel.data = pad_u8(&channel.data, old_w, old_h, new_w, new_h, x0, y0, purpose)?.into();
    }
    if let Some(store) = layer.source_channels.as_mut() {
        if store.rect == old {
            for (id, samples) in &mut store.planes {
                if *id == -2 {
                    continue;
                }
                if samples.len() != old_n {
                    continue;
                }
                *samples = pad_samples(samples, old_w, old_h, new_w, new_h, x0, y0, purpose)?;
            }
            store.rect = rect;
        }
    }
    // Verbatim compressed streams for unmodeled channels cannot be padded; a
    // grown layer re-emits only the channels the engine models.
    // ponytail: a grow drops preserved raw channels instead of growing them.
    layer.raw_channels.clear();
    layer.rect = rect;
    Ok(true)
}

/// Crop `layer`'s planes back to `rect` (a subset of its current rect), the
/// inverse of [`grow_layer_to`]. The pixels outside `rect` are dropped.
pub fn shrink_layer_to(layer: &mut Layer, rect: PsdRect) {
    let old = layer.rect;
    if rect == old {
        return;
    }
    if rect.is_empty() {
        for channel in &mut layer.channels {
            channel.data = crate::Plane::default();
        }
        if let Some(store) = layer.source_channels.as_mut() {
            for (id, samples) in &mut store.planes {
                if *id == -2 {
                    continue;
                }
                *samples = match samples {
                    Samples::U8(_) => Samples::U8(Vec::new().into()),
                    Samples::U16(_) => Samples::U16(Vec::new().into()),
                    Samples::F32(_) => Samples::F32(Vec::new().into()),
                };
            }
            store.rect = rect;
        }
        layer.rect = rect;
        return;
    }
    let (old_w, old_h) = (old.width().max(0) as usize, old.height().max(0) as usize);
    let (new_w, new_h) = (rect.width() as usize, rect.height() as usize);
    let (x0, y0) = (
        (rect.left - old.left).max(0) as usize,
        (rect.top - old.top).max(0) as usize,
    );
    for channel in &mut layer.channels {
        if channel.data.len() != old_w * old_h {
            continue;
        }
        let mut out = vec![0u8; new_w * new_h];
        channel
            .data
            .read_rect_into(x0..x0 + new_w, y0..y0 + new_h, old_w, &mut out);
        channel.data = out.into();
    }
    if let Some(store) = layer.source_channels.as_mut() {
        if store.rect == old {
            for (id, samples) in &mut store.planes {
                if *id == -2 {
                    continue;
                }
                if samples.len() != old_w * old_h {
                    continue;
                }
                *samples = crop_samples(samples, old_w, new_w, new_h, x0, y0);
            }
            store.rect = rect;
        }
    }
    layer.rect = rect;
}

fn crop_samples(
    samples: &Samples,
    old_w: usize,
    new_w: usize,
    new_h: usize,
    x0: usize,
    y0: usize,
) -> Samples {
    fn crop<T: Copy + Default>(
        src: &[T],
        old_w: usize,
        new_w: usize,
        new_h: usize,
        x0: usize,
        y0: usize,
    ) -> Vec<T> {
        let mut out = vec![T::default(); new_w * new_h];
        for y in 0..new_h {
            let from = (y0 + y) * old_w + x0;
            let to = y * new_w;
            if from + new_w <= src.len() {
                out[to..to + new_w].copy_from_slice(&src[from..from + new_w]);
            }
        }
        out
    }
    match samples {
        Samples::U8(plane) => {
            Samples::U8(crop(plane.as_slice(), old_w, new_w, new_h, x0, y0).into())
        }
        Samples::U16(plane) => {
            Samples::U16(crop(plane.as_slice(), old_w, new_w, new_h, x0, y0).into())
        }
        Samples::F32(plane) => {
            Samples::F32(crop(plane.as_slice(), old_w, new_w, new_h, x0, y0).into())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pad_u8(
    data: &crate::Plane<u8>,
    old_w: usize,
    old_h: usize,
    new_w: usize,
    new_h: usize,
    x0: usize,
    y0: usize,
    purpose: &'static str,
) -> Result<Vec<u8>, MemoryError> {
    let mut out = allocate(new_w * new_h, purpose)?;
    out.resize(new_w * new_h, 0);
    copy_rows(&mut out, data, old_w, old_h, new_w, x0, y0);
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn pad_samples(
    samples: &Samples,
    old_w: usize,
    old_h: usize,
    new_w: usize,
    new_h: usize,
    x0: usize,
    y0: usize,
    purpose: &'static str,
) -> Result<Samples, MemoryError> {
    Ok(match samples {
        Samples::U8(plane) => {
            let mut out = allocate(new_w * new_h, purpose)?;
            out.resize(new_w * new_h, 0u8);
            copy_rows(&mut out, plane, old_w, old_h, new_w, x0, y0);
            Samples::U8(out.into())
        }
        Samples::U16(plane) => {
            let mut out = allocate(new_w * new_h, purpose)?;
            out.resize(new_w * new_h, 0u16);
            copy_rows(&mut out, plane, old_w, old_h, new_w, x0, y0);
            Samples::U16(out.into())
        }
        Samples::F32(plane) => {
            let mut out = allocate(new_w * new_h, purpose)?;
            out.resize(new_w * new_h, 0f32);
            copy_rows(&mut out, plane, old_w, old_h, new_w, x0, y0);
            Samples::F32(out.into())
        }
    })
}

fn allocate<T>(len: usize, purpose: &'static str) -> Result<Vec<T>, MemoryError> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| MemoryError::Insufficient {
            purpose,
            bytes: len as u64,
        })?;
    Ok(out)
}

fn copy_rows<T: Copy>(
    out: &mut [T],
    old: &crate::Plane<T>,
    old_w: usize,
    old_h: usize,
    new_w: usize,
    x0: usize,
    y0: usize,
) {
    for (y, row) in old.rows(0..old_h, old_w).enumerate() {
        let dst = (y0 + y) * new_w + x0;
        if row.len() == old_w && dst + old_w <= out.len() {
            out[dst..dst + old_w].copy_from_slice(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{self, HostReading};
    use crate::{BitDepth, Channel, LockFlags, Plane, SourceChannels};

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn layer_at(rect: PsdRect, w: usize, h: usize) -> Layer {
        let mut layer = Layer {
            rect,
            ..Layer::default()
        };
        for id in [0, 1, 2, -1] {
            layer.channels.push(Channel {
                id,
                data: Plane::from(vec![id as u8; w * h]),
            });
        }
        layer
    }

    #[test]
    fn grows_channels_and_retained_samples() {
        let mut layer = layer_at(rect(1, 1, 3, 3), 2, 2);
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            rect(1, 1, 3, 3),
            vec![
                (0, Samples::U16(vec![7u16; 4].into())),
                (-1, Samples::U16(vec![65535u16; 4].into())),
            ],
        ));
        assert!(grow_layer_to(&mut layer, rect(0, 0, 4, 4), "a test").unwrap());
        assert_eq!(layer.rect, rect(0, 0, 4, 4));
        for channel in &layer.channels {
            assert_eq!(channel.data.len(), 16);
            assert_eq!(channel.data[5], channel.id as u8, "old pixel at (1,1)");
            assert_eq!(channel.data[0], 0, "new area is transparent");
        }
        let store = layer.source_channels.expect("the store survives");
        assert_eq!(store.rect, rect(0, 0, 4, 4));
        let red = store
            .planes
            .iter()
            .find(|(id, _)| *id == 0)
            .expect("red plane")
            .1
            .clone();
        assert_eq!(red.len(), 16);
        assert_eq!(red.slice(5..6), Some(Samples::U16(vec![7u16].into())));
    }

    #[test]
    fn refuses_a_transparency_lock() {
        let mut layer = layer_at(rect(0, 0, 2, 2), 2, 2);
        layer.lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
        assert!(!grow_layer_to(&mut layer, rect(0, 0, 4, 4), "a test").unwrap());
        assert_eq!(layer.rect, rect(0, 0, 2, 2));
    }

    #[test]
    fn refuses_a_non_superset() {
        let mut layer = layer_at(rect(1, 1, 3, 3), 2, 2);
        assert!(!grow_layer_to(&mut layer, rect(2, 2, 4, 4), "a test").unwrap());
        assert_eq!(layer.rect, rect(1, 1, 3, 3));
    }

    #[test]
    fn refuses_when_the_governor_is_full() {
        memory::set_host_for_test(Some(HostReading {
            available: Some(0),
            ..HostReading::default()
        }));
        let mut layer = layer_at(rect(0, 0, 1, 1), 1, 1);
        let refused = grow_layer_to(&mut layer, rect(0, 0, 64, 64), "a test");
        memory::set_host_for_test(None);
        assert!(refused.is_err());
        assert_eq!(layer.rect, rect(0, 0, 1, 1));
    }

    #[test]
    fn drops_verbatim_raw_channels() {
        let mut layer = layer_at(rect(0, 0, 2, 2), 2, 2);
        layer.raw_channels.push(crate::RawChannel {
            id: 7,
            data: Plane::from(vec![0u8, 0, 0]),
        });
        assert!(grow_layer_to(&mut layer, rect(0, 0, 3, 3), "a test").unwrap());
        assert!(layer.raw_channels.is_empty());
    }
}
