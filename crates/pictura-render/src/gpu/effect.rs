//! Layer-effect pass dispatch: replay a cached `EffectPass` on the GPU.

use pictura_core::{Document, Layer};

use super::backend::{Gpu, SrcLayout};
use super::NO_ADJ;
use crate::layer_effects::{EffectColor, EffectPass};

impl Gpu {
    /// Replay `pass` into the canvas as one packed source-over dispatch.
    ///
    /// ponytail: the pass colour and alpha are quantized to the packed 8-bit
    /// word. The CPU keeps both in `f32`; if a family exceeds the ±1 LSB parity
    /// budget, the upgrade is an `f32` alpha binding rather than a tolerance
    /// change.
    pub(super) fn composite_effect_pass(
        &self,
        canvas: &wgpu::Buffer,
        pass: &EffectPass,
        layer_opacity: u8,
        doc: &Document,
    ) {
        let region = self.region();
        let left = pass.rect.left.max(region.x0 as i32);
        let top = pass.rect.top.max(region.y0 as i32);
        let right = pass.rect.right.min((region.x0 + region.w) as i32);
        let bottom = pass.rect.bottom.min((region.y0 + region.h) as i32);
        if right <= left || bottom <= top {
            return;
        }
        let w = (right - left) as usize;
        let h = (bottom - top) as usize;
        let n = w * h;
        let mut words = vec![0u32; n];
        let pw = pass.rect.width().max(0) as usize;
        let plane = pw * pass.rect.height().max(0) as usize;
        for y in top..bottom {
            for x in left..right {
                let pi = (y - pass.rect.top) as usize * pw + (x - pass.rect.left) as usize;
                let alpha = pass.alpha.get(pi).copied().unwrap_or(0.0);
                if alpha <= 0.0 {
                    continue;
                }
                let color = match &pass.color {
                    EffectColor::Constant(color) => *color,
                    EffectColor::PerPixel(buffer) => [
                        buffer.data.get(pi).copied().unwrap_or(0) as f32 / 255.0,
                        buffer.data.get(plane + pi).copied().unwrap_or(0) as f32 / 255.0,
                        buffer.data.get(2 * plane + pi).copied().unwrap_or(0) as f32 / 255.0,
                    ],
                };
                let ri = (y - top) as usize * w + (x - left) as usize;
                words[ri] = pack_word(color, alpha);
            }
        }
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let src = self.upload_with("pictura-effect", n * 4, move |start, mut out| {
            out.copy_from_slice(&bytes[start..start + out.len()]);
        });
        let proxy = Layer {
            is_group: true,
            ..Default::default()
        };
        let mask = self.build_mask(&proxy, doc);
        let layout = SrcLayout {
            x0: left as u32,
            y0: top as u32,
            w: w as u32,
            h: h as u32,
            gray: false,
            packed: true,
        };
        self.dispatch(
            canvas,
            &src,
            &mask,
            layout,
            pass.mode,
            layer_opacity,
            255,
            false,
            NO_ADJ,
            None,
        );
    }
}

/// Pack straight RGBA into the little-endian word the shader unpacks.
pub(super) fn pack_word(color: [f32; 3], alpha: f32) -> u32 {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    q(color[0]) | (q(color[1]) << 8) | (q(color[2]) << 16) | (q(alpha) << 24)
}
