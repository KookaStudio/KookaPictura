//! Growing a stroke's target layer (#1208).

use super::*;

impl Saved {
    /// Pad the saved planes when the layer grows (#1208). Saved tiles keep
    /// their pre-stroke bytes; old pixels no tile saved yet are still the
    /// layer's own, so they are copied from `layer` now.
    pub(super) fn grow(&mut self, layer: &Layer, ch: PlaneIndex, new_rect: PsdRect) {
        let old_rect = self.rect;
        let (old_w, old_h) = (self.width, self.height);
        let (new_w, new_h) = (new_rect.width().max(0), new_rect.height().max(0));
        if new_w == old_w && new_h == old_h {
            return;
        }
        let (dx, dy) = (
            (old_rect.left - new_rect.left).max(0),
            (old_rect.top - new_rect.top).max(0),
        );
        let old_tiles = std::mem::take(&mut self.tiles);
        let old_columns = self.columns;

        for (k, plane) in self.planes.iter_mut().enumerate() {
            let Some(plane) = plane else { continue };
            let mut data = vec![0u8; (new_w * new_h) as usize];
            for y in 0..old_h {
                let src = (y * old_w) as usize;
                let dst = ((dy + y) * new_w + dx) as usize;
                let n = old_w as usize;
                if src + n <= plane.len() && dst + n <= data.len() {
                    data[dst..dst + n].copy_from_slice(&plane[src..src + n]);
                }
            }
            if let Some(source) = ch[k].and_then(|i| layer.channels.get(i)) {
                let bytes = source.data.as_slice();
                for y in 0..old_h {
                    let ty = y / SAVED_TILE;
                    for x in 0..old_w {
                        let tx = x / SAVED_TILE;
                        if old_tiles[(ty * old_columns + tx) as usize] {
                            continue;
                        }
                        // The layer has already grown, so the old pixel sits at
                        // its grown local index, not its old local index.
                        let at = ((dy + y) * new_w + dx + x) as usize;
                        if let Some(&value) = bytes.get(at) {
                            if at < data.len() {
                                data[at] = value;
                            }
                        }
                    }
                }
            }
            *plane = data;
        }
        let columns = (new_w + SAVED_TILE - 1) / SAVED_TILE;
        let rows = (new_h + SAVED_TILE - 1) / SAVED_TILE;
        let mut tiles = vec![false; (columns * rows) as usize];
        let (old_right, old_bottom) = (old_rect.left + old_w, old_rect.top + old_h);
        for ty in 0..rows {
            for tx in 0..columns {
                let (left, top) = (
                    new_rect.left + tx * SAVED_TILE,
                    new_rect.top + ty * SAVED_TILE,
                );
                if left < old_right
                    && left + SAVED_TILE > old_rect.left
                    && top < old_bottom
                    && top + SAVED_TILE > old_rect.top
                {
                    tiles[(ty * columns + tx) as usize] = true;
                }
            }
        }
        self.columns = columns;
        self.tiles = tiles;
        self.rect = new_rect;
        self.width = new_w;
        self.height = new_h;
    }
}

impl Stroke {
    /// Grow the target layer so a sample at document `(x, y)` fits, padding the
    /// stroke's layer-local state with it (#1208). False when the grow is
    /// refused for memory.
    pub fn ensure_rect_for_sample(&mut self, doc: &mut Document, x: f32, y: f32) -> bool {
        let cfg = self.cfg;
        let reach = cfg.diameter.max(1) as f32 * (1.0 + cfg.scatter as f32 / 100.0) * 0.5 + 3.0;
        let box_ = PsdRect {
            top: (y - reach).floor() as i32,
            left: (x - reach).floor() as i32,
            bottom: (y + reach).ceil() as i32,
            right: (x + reach).ceil() as i32,
        };
        let Some(box_) = box_.clip_to(doc.width, doc.height) else {
            return true;
        };
        let Some(layer) = layer_at(doc, &self.layer_path) else {
            return false;
        };
        let old = layer.rect;
        let new = if old.is_empty() {
            box_
        } else {
            old.union(box_)
        };
        if new == old {
            return true;
        }
        let Some(layer) = layer_at_mut(doc, &self.layer_path) else {
            return false;
        };
        match pictura_core::layer_bounds::grow_layer_to(layer, new, "a brush stroke") {
            Ok(true) => {}
            Ok(false) => return true,
            Err(_) => return false,
        }
        let Some(layer) = layer_at(doc, &self.layer_path) else {
            return false;
        };
        self.remap(layer, old, new);
        true
    }

    /// Move the stroke's layer-local state from `old` to the grown `new`.
    fn remap(&mut self, layer: &Layer, old: PsdRect, new: PsdRect) {
        let (old_w, old_h) = (old.width().max(0) as usize, old.height().max(0) as usize);
        let (new_w, new_h) = (new.width().max(0) as usize, new.height().max(0) as usize);
        let (dx, dy) = (
            (old.left - new.left).max(0) as usize,
            (old.top - new.top).max(0) as usize,
        );
        pad_vec(&mut self.scratch, old_w, old_h, new_w, new_h, dx, dy);
        pad_vec(&mut self.applied, old_w, old_h, new_w, new_h, dx, dy);
        self.saved.grow(layer, self.ch, new);
        if let Some(per) = self.per_dab.as_mut() {
            per.pixels
                .grow(new_w as i32, new_h as i32, dx as i32, dy as i32);
            per.lazy = Some(LazyPixels::new(layer).0);
            match &mut per.engine {
                DabEngine::Replace(replacer) => {
                    replacer.grow(old_w, old_h, new_w, new_h, dx, dy);
                }
                DabEngine::Mixer(mixer) => {
                    mixer.grow(new_w as i32, new_h as i32, dx as i32, dy as i32);
                }
                DabEngine::ArtHistory(brush) => {
                    brush.grow(new_w as i32, new_h as i32, dx as i32, dy as i32);
                }
                DabEngine::BackgroundErase(_) => {}
                DabEngine::Focus(brush) => {
                    brush.grow(new_w as i32, new_h as i32, dx as i32, dy as i32);
                }
                DabEngine::Smudge(brush) => {
                    brush.grow(new_w as i32, new_h as i32, dx as i32, dy as i32);
                }
                DabEngine::Tone(brush) => {
                    brush.grow(old_w, old_h, new_w, new_h, dx, dy);
                }
            }
        }
        let shift = |dirty: Option<PsdRect>| {
            dirty.map(|d| PsdRect {
                top: d.top + dy as i32,
                left: d.left + dx as i32,
                bottom: d.bottom + dy as i32,
                right: d.right + dx as i32,
            })
        };
        self.dirty = shift(self.dirty);
        self.dab_dirty = shift(self.dab_dirty);
        self.placer.shift(dx as f32, dy as f32);
        self.rect = new;
    }
}

/// Pad a layer-local plane to `new_w`×`new_h`, placing the old pixels at
/// `(dx, dy)`. The new area takes `T::default()` (#1208).
fn pad_vec<T: Copy + Default>(
    data: &mut Vec<T>,
    old_w: usize,
    old_h: usize,
    new_w: usize,
    new_h: usize,
    dx: usize,
    dy: usize,
) {
    if new_w == old_w && new_h == old_h {
        return;
    }
    let mut out = vec![T::default(); new_w * new_h];
    for y in 0..old_h {
        let src = y * old_w;
        let dst = (dy + y) * new_w + dx;
        if src + old_w <= data.len() && dst + old_w <= out.len() {
            out[dst..dst + old_w].copy_from_slice(&data[src..src + old_w]);
        }
    }
    *data = out;
}
