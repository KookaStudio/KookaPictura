//! Growing a stroke's target layer (#1208).

use super::*;
use pictura_core::layer_bounds::padded_plane;

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
        self.scratch = padded_plane(&self.scratch, old_w, old_h, new_w, new_h, dx, dy);
        self.applied = padded_plane(&self.applied, old_w, old_h, new_w, new_h, dx, dy);
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
                DabEngine::ToneNative(brush) => {
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
