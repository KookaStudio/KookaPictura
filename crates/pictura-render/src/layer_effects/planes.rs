//! Document-space effect passes: build once, replay into any canvas rectangle.
//!
//! A pass carries the same per-pixel blend inputs the uncached effect loop
//! passed to [`crate::composite::blend_parts`], so replaying it through the same
//! call is byte-identical. Alpha is stored unscaled; the layer's opacity is
//! applied by `blend_parts` at replay time.

use pictura_core::{BlendMode, PixelBuffer, PsdRect};

use crate::composite::{blend_parts, Canvas};

pub(crate) enum EffectColor {
    Constant([f32; 3]),
    /// Three planes (R, G, B) over the pass rect.
    PerPixel(PixelBuffer),
}

pub(crate) struct EffectPass {
    pub(crate) rect: PsdRect,
    pub(crate) mode: BlendMode,
    pub(crate) color: EffectColor,
    pub(crate) alpha: Vec<f32>,
}

impl EffectPass {
    pub(crate) fn size(&self) -> usize {
        let color = match &self.color {
            EffectColor::Constant(_) => 0,
            EffectColor::PerPixel(buffer) => buffer.data.len(),
        };
        color + self.alpha.len() * std::mem::size_of::<f32>()
    }
}

pub(super) struct PassWriter {
    rect: PsdRect,
    mode: BlendMode,
    color: EffectColor,
    alpha: Vec<f32>,
}

impl PassWriter {
    pub(super) fn constant(rect: PsdRect, mode: BlendMode, color: [f32; 3]) -> Self {
        Self {
            rect,
            mode,
            color: EffectColor::Constant(color),
            alpha: vec![0.0; area(rect)],
        }
    }

    pub(super) fn per_pixel(rect: PsdRect, mode: BlendMode) -> Self {
        let width = rect.width().max(0) as u32;
        let height = rect.height().max(0) as u32;
        Self {
            rect,
            mode,
            color: EffectColor::PerPixel(PixelBuffer::new(width, height, 3)),
            alpha: vec![0.0; area(rect)],
        }
    }

    pub(super) fn emit(&mut self, x: i32, y: i32, color: [f32; 3], alpha: f32) {
        let i = index(self.rect, x, y);
        if i >= self.alpha.len() {
            return;
        }
        self.alpha[i] = alpha;
        if let EffectColor::PerPixel(buffer) = &mut self.color {
            let plane = buffer.pixel_count();
            buffer.data[i] = to_u8(color[0]);
            buffer.data[plane + i] = to_u8(color[1]);
            buffer.data[2 * plane + i] = to_u8(color[2]);
        }
    }

    pub(super) fn finish(self) -> EffectPass {
        EffectPass {
            rect: self.rect,
            mode: self.mode,
            color: self.color,
            alpha: self.alpha,
        }
    }
}

/// Replay `passes` into the canvas, clipped to the canvas rectangle.
pub(super) fn apply(canvas: &mut Canvas, passes: &[EffectPass]) {
    let (x0, y0) = (canvas.x0(), canvas.y0());
    let (x1, y1) = (canvas.x1(), canvas.y1());
    for pass in passes {
        let left = pass.rect.left.max(x0);
        let top = pass.rect.top.max(y0);
        let right = pass.rect.right.min(x1);
        let bottom = pass.rect.bottom.min(y1);
        if right <= left || bottom <= top {
            continue;
        }
        let width = pass.rect.width().max(0) as usize;
        let plane = pass.rect.width().max(0) as usize * pass.rect.height().max(0) as usize;
        for y in top..bottom {
            for x in left..right {
                let i = (y - pass.rect.top) as usize * width + (x - pass.rect.left) as usize;
                let alpha = pass.alpha[i];
                if alpha <= 0.0 {
                    continue;
                }
                let color = match &pass.color {
                    EffectColor::Constant(color) => *color,
                    EffectColor::PerPixel(buffer) => [
                        buffer.data[i] as f32 / 255.0,
                        buffer.data[plane + i] as f32 / 255.0,
                        buffer.data[2 * plane + i] as f32 / 255.0,
                    ],
                };
                blend_parts(canvas, x as usize, y as usize, color, alpha, pass.mode);
            }
        }
    }
}

fn area(rect: PsdRect) -> usize {
    rect.width().max(0) as usize * rect.height().max(0) as usize
}

fn index(rect: PsdRect, x: i32, y: i32) -> usize {
    (y - rect.top) as usize * rect.width().max(0) as usize + (x - rect.left) as usize
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
