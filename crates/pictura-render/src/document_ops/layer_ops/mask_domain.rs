//! A finite stored mask and the canvas window through which it is edited.

use pictura_core::{memory, Document, Layer, LayerMask, Plane, PsdRect, Samples, SourceChannels};

use super::paths::resolve_path_mut;

pub(super) fn canvas(doc: &Document) -> Option<PsdRect> {
    let (right, bottom) = (
        i32::try_from(doc.width).ok()?,
        i32::try_from(doc.height).ok()?,
    );
    (right > 0 && bottom > 0).then_some(PsdRect {
        top: 0,
        left: 0,
        bottom,
        right,
    })
}

pub(super) fn dimensions(rect: PsdRect) -> Option<(usize, usize)> {
    let w = i64::from(rect.right) - i64::from(rect.left);
    let h = i64::from(rect.bottom) - i64::from(rect.top);
    if w < 0 || h < 0 || w > i64::from(i32::MAX) || h > i64::from(i32::MAX) {
        return None;
    }
    Some((usize::try_from(w).ok()?, usize::try_from(h).ok()?))
}

pub(super) fn pixels(rect: PsdRect) -> Option<usize> {
    let (w, h) = dimensions(rect)?;
    w.checked_mul(h)
}

pub(super) fn translated(rect: PsdRect, dx: i32, dy: i32) -> Option<PsdRect> {
    Some(PsdRect {
        left: rect.left.checked_add(dx)?,
        right: rect.right.checked_add(dx)?,
        top: rect.top.checked_add(dy)?,
        bottom: rect.bottom.checked_add(dy)?,
    })
}

fn expanded(rect: PsdRect, pad: i32) -> Option<PsdRect> {
    Some(PsdRect {
        left: rect.left.checked_sub(pad)?,
        top: rect.top.checked_sub(pad)?,
        right: rect.right.checked_add(pad)?,
        bottom: rect.bottom.checked_add(pad)?,
    })
}

pub(super) fn stored(mask: &LayerMask, x: i32, y: i32) -> u8 {
    let Some(data) = &mask.data else { return 255 };
    let Some((w, h)) = dimensions(mask.rect) else {
        return mask.default_color;
    };
    let mx = i64::from(x) - i64::from(mask.rect.left);
    let my = i64::from(y) - i64::from(mask.rect.top);
    if mx < 0 || my < 0 || mx >= w as i64 || my >= h as i64 {
        return mask.default_color;
    }
    data.sample(my as usize * w + mx as usize)
        .unwrap_or(mask.default_color)
}

pub(super) fn reserve(bytes: usize) -> Option<memory::Reservation<'static>> {
    memory::governor()
        .reserve(u64::try_from(bytes).ok()?, "layer mask")
        .ok()
}

pub(super) fn allocate(n: usize, value: u8) -> Option<Plane<u8>> {
    Some(Plane::build(n, |data| data.fill(value)))
}

pub(super) struct Prepared {
    pub mask: LayerMask,
    pub source: Option<SourceChannels>,
    _held: memory::Reservation<'static>,
}

impl Prepared {
    pub fn publish(self, layer: &mut Layer) {
        layer.mask = Some(self.mask);
        layer.source_channels = self.source;
        layer.raw_channels.retain(|c| c.id != -2);
    }

    pub fn write(&mut self, x: i32, y: i32, value: u8) -> Option<()> {
        let (w, _) = dimensions(self.mask.rect)?;
        let i = usize::try_from(i64::from(y) - i64::from(self.mask.rect.top))
            .ok()?
            .checked_mul(w)?
            .checked_add(usize::try_from(i64::from(x) - i64::from(self.mask.rect.left)).ok()?)?;
        let data = self.mask.data.as_mut()?;
        if data.sample(i)? == value {
            return Some(());
        }
        let row = (y - self.mask.rect.top) as usize;
        let col = (x - self.mask.rect.left) as usize;
        let (xs, ys) = (col..col + 1, row..row + 1);
        data.write_rect_from(xs.clone(), ys.clone(), w, &[value]);
        if let Some(samples) = self.source.as_mut().and_then(|s| {
            s.planes
                .iter_mut()
                .find(|(id, _)| *id == -2)
                .map(|(_, p)| p)
        }) {
            match samples {
                Samples::U8(p) => p.write_rect_from(xs, ys, w, &[value]),
                Samples::U16(p) => p.write_rect_from(xs, ys, w, &[u16::from(value) * 257]),
                Samples::F32(p) => p.write_rect_from(xs, ys, w, &[f32::from(value) / 255.0]),
            }
        }
        Some(())
    }
}

pub(super) fn prepare(layer: &Layer, canvas: PsdRect, required: PsdRect) -> Option<Prepared> {
    let old = layer.mask.as_ref()?;
    let old_n = pixels(old.rect)?;
    if old.data.as_ref().is_some_and(|data| data.len() != old_n) {
        return None;
    }
    let halo = if old.data.is_none() || old.default_color != 0 {
        crate::mask_feather_reach(old)
    } else {
        0
    };
    let captured = expanded(canvas, halo)?;
    let mut rect = captured.union(required);
    if old_n > 0 {
        rect = rect.union(old.rect);
    }
    let n = pixels(rect)?;
    let native = layer
        .source_channels
        .as_ref()
        .and_then(|source| {
            source
                .planes
                .iter()
                .find(|(id, _)| *id == -2)
                .map(|(_, p)| p)
        })
        .filter(|samples| old.data.is_some() && samples.len() == old_n);
    let sample_bytes = native.map_or(0, |p| match p {
        Samples::U8(_) => 1,
        Samples::U16(_) => 2,
        Samples::F32(_) => 4,
    });
    // Native Vec-to-plane conversion and copy-on-write bands can overlap.
    let held = reserve(n.checked_mul(2usize.checked_mul(1 + sample_bytes)?)?)?;
    let mut mask = old.clone();
    mask.default_color = 0;
    mask.rect = rect;
    if rect != old.rect || old.data.is_none() {
        let fill = if old.data.is_none() {
            255
        } else {
            old.default_color
        };
        let mut data = allocate(n, 0)?;
        let (w, _) = dimensions(rect)?;
        for area in [Some(captured), old.data.is_none().then_some(old.rect)]
            .into_iter()
            .flatten()
        {
            if let Some(hit) = area.intersect(rect) {
                // The new plane is flat until it is tiled below.
                let flat = data.as_mut_slice();
                for y in hit.top..hit.bottom {
                    let at = (y - rect.top) as usize * w;
                    let (a, b) = (
                        (hit.left - rect.left) as usize,
                        (hit.right - rect.left) as usize,
                    );
                    flat.get_mut(at + a..at + b)?.fill(fill);
                }
            }
        }
        if let Some(source) = old.data.as_ref() {
            let (ow, oh) = dimensions(old.rect)?;
            let (w, _) = dimensions(rect)?;
            let flat = data.as_mut_slice();
            for y in 0..oh {
                let sy = old.rect.top + y as i32;
                let at = (sy - rect.top) as usize * w + (old.rect.left - rect.left) as usize;
                source.read_into(y * ow, flat.get_mut(at..at + ow)?);
            }
        }
        mask.data = Some(data);
    }
    let mut source = layer.source_channels.clone();
    if let Some(store) = source.as_mut() {
        if let Some(retained) = native {
            if rect != old.rect {
                let padded = pad_native(retained, old.rect, rect, captured, old.default_color)?;
                store.planes.iter_mut().find(|(id, _)| *id == -2)?.1 = padded;
            }
        } else {
            store.planes.retain(|(id, _)| *id != -2);
        }
    }
    // The 8-bit mask is tiled so an edit forks only the tiles it touches.
    // ponytail: the retained native mask stays flat, as every native store does.
    let (width, height) = dimensions(rect)?;
    mask.data.as_mut()?.tile(width, height);
    Some(Prepared {
        mask,
        source,
        _held: held,
    })
}

fn pad_native(
    samples: &Samples,
    old: PsdRect,
    rect: PsdRect,
    captured: PsdRect,
    fill: u8,
) -> Option<Samples> {
    let n = pixels(rect)?;
    let (w, _) = dimensions(rect)?;
    let (ow, oh) = dimensions(old)?;
    macro_rules! padded {
        ($plane:expr,$zero:expr,$value:expr,$variant:ident) => {{
            let mut out = Vec::new();
            out.try_reserve_exact(n).ok()?;
            out.resize(n, $zero);
            for y in captured.top..captured.bottom {
                let begin = (y - rect.top) as usize * w + (captured.left - rect.left) as usize;
                out[begin..begin + captured.width() as usize].fill($value);
            }
            for y in 0..oh {
                let begin =
                    (y + (old.top - rect.top) as usize) * w + (old.left - rect.left) as usize;
                $plane.read_into(y * ow, &mut out[begin..begin + ow]);
            }
            Samples::$variant(out.into())
        }};
    }
    Some(match samples {
        Samples::U8(p) => padded!(p, 0, fill, U8),
        Samples::U16(p) => padded!(p, 0, u16::from(fill) * 257, U16),
        Samples::F32(p) => padded!(p, 0.0, f32::from(fill) / 255.0, F32),
    })
}

/// Translate only a raster mask, preserving samples beyond the canvas.
pub fn translate_layer_mask(doc: &mut Document, path: &str, dx: i32, dy: i32) -> bool {
    if dx == 0 && dy == 0 {
        return false;
    }
    let Some(canvas) = canvas(doc) else {
        return false;
    };
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(old) = layer.mask.as_ref() else {
        return false;
    };
    let Some(n) = pixels(old.rect) else {
        return false;
    };
    if old.data.as_ref().is_some_and(|data| data.len() != n) {
        return false;
    }
    if translated(old.rect, dx, dy).is_none() {
        return false;
    }
    if old.default_color == 0 && old.data.is_some() {
        let Some(rect) = translated(old.rect, dx, dy) else {
            return false;
        };
        layer.mask.as_mut().unwrap().rect = rect;
        return true;
    }
    let Some(mut pending) = prepare(layer, canvas, canvas) else {
        return false;
    };
    let Some(rect) = translated(pending.mask.rect, dx, dy) else {
        return false;
    };
    pending.mask.rect = rect;
    pending.publish(layer);
    true
}

/// Capture a legacy exterior before an owning layer carries its mask along.
pub(crate) fn normalize_layer_mask(layer: &mut Layer, canvas: PsdRect) -> bool {
    let Some(mask) = layer.mask.as_ref() else {
        return true;
    };
    let Some(n) = pixels(mask.rect) else {
        return false;
    };
    if mask.data.as_ref().is_some_and(|data| data.len() != n) {
        return false;
    }
    if mask.default_color == 0 && mask.data.is_some() {
        return true;
    }
    let Some(pending) = prepare(layer, canvas, canvas) else {
        return false;
    };
    pending.publish(layer);
    true
}
