//! GPU rasterization of a mask stroke's dabs.
//!
//! A mask is one coverage byte per pixel, so a mask dab runs the same
//! [`GpuStroke`] shader with `channels == 1`: it blends byte 0 only and leaves
//! the other channels untouched. The mask's own grayscale document is patched
//! through the exact stroke's `patch_working_layer`, then copied into the real
//! mask, mirroring the pixel GPU stroke but for the 1-channel mask plane.

use super::super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{Document, PsdRect};
use pictura_paint::StrokeSample;

/// The RGBA seed the GPU mask stroke uploads: each coverage byte replicated
/// across RGB at opaque alpha, so byte 0 carries the coverage the shader blends.
pub(super) fn mask_rgba_seed(document: &Document) -> Vec<u8> {
    let n = (document.width as usize) * (document.height as usize);
    let data = document
        .layers
        .first()
        .and_then(|layer| layer.channels.iter().find(|c| c.id == 0))
        .map(|c| c.data.as_slice());
    let mut seed = vec![0u8; n * 4];
    for i in 0..n {
        let v = data.and_then(|d| d.get(i)).copied().unwrap_or(0);
        seed[i * 4] = v;
        seed[i * 4 + 1] = v;
        seed[i * 4 + 2] = v;
        seed[i * 4 + 3] = 255;
    }
    seed
}

impl qobject::PictureView {
    /// One sample of a GPU mask stroke: place its dabs exactly as the exact
    /// stroke would, patch each dab's changed coverage into the mask document,
    /// copy the region back into the real mask, and present it. The commit needs
    /// no replay: the mask document already holds the GPU-authored coverage.
    pub(super) fn mask_gpu_dab(mut self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        let (lx, ly) = {
            let rust = self.rust();
            let Some(ctx) = rust.session.live.mask_stroke() else {
                return false;
            };
            (x - ctx.rect.left as f64, y - ctx.rect.top as f64)
        };
        let sample = StrokeSample {
            x: lx as f32,
            y: ly as f32,
            pressure: pressure as f32,
        };
        let mut dabs = Vec::new();
        let mut changed = false;
        {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let Some(placer) = rust.session.live.gpu_placer_mut() else {
                return false;
            };
            placer.feed(sample.x, sample.y, &mut dabs);
            for (cx, cy) in dabs {
                let dab = {
                    let Some(gpu) = rust.session.live.gpu_stroke_mut() else {
                        break;
                    };
                    gpu.dab(cx, cy).ok().flatten()
                };
                let Some((rect, stride, planes)) = dab else {
                    continue;
                };
                let patched = match rust.session.live.mask_stroke_mut() {
                    Some(ctx) => {
                        ctx.stroke
                            .patch_working_layer(&mut ctx.document, rect, stride, &planes)
                    }
                    None => false,
                };
                if patched {
                    changed = true;
                }
            }
        }
        if !changed {
            return false;
        }
        let (dirty, rect) = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let Some(ctx) = rust.session.live.mask_stroke_mut() else {
                return false;
            };
            (ctx.stroke.take_dirty(), ctx.rect)
        };
        let Some(dirty) = dirty else {
            return false;
        };
        {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let (Some(doc), Some(ctx)) =
                (rust.session.doc.as_mut(), rust.session.live.mask_stroke())
            else {
                return false;
            };
            pictura_render::write_mask_back(doc, &ctx.path, &ctx.document, Some(dirty));
        }
        let present = PsdRect {
            top: dirty.top + rect.top,
            left: dirty.left + rect.left,
            bottom: dirty.bottom + rect.top,
            right: dirty.right + rect.left,
        };
        if let Some(region) = self.as_mut().rust_mut().queue_present(present) {
            self.as_mut().refresh_region(region);
        }
        true
    }
}
