//! The 16-bit tone path: the same Dodge, Burn, and Sponge model as
//! [`crate::tone`], run over the layer's retained native colour planes so the
//! pixels the compositor reads are the ones the stroke edits.
//!
//! The working model is 8-bit; a 16-bpc layer keeps its display and save pixels
//! in `Layer::source_channels`. This brush reads and writes those `u16` planes,
//! writes their narrowing into the per-dab RGBA image, and leaves the existing
//! `write_back`, dirty tracking, and history capture to carry the mirror.

use crate::healing::RgbaImage;
use crate::replace::dab_bounds;
use crate::tip::TipParams;
use crate::tone::{sponge, tone, Tone, ToneOptions};
use crate::StrokeConfig;
use pictura_core::{BitDepth, Coverage, Layer, PsdRect, Samples};

/// One 16-bit toning stroke's state, mirroring [`crate::tone::ToneBrush`].
pub(crate) struct NativeToneBrush {
    options: ToneOptions,
    gray: bool,
    /// Sized to the layer on the first dab.
    applied: Vec<f32>,
}

impl NativeToneBrush {
    pub(crate) fn new(options: ToneOptions, gray: bool) -> Self {
        NativeToneBrush {
            options,
            gray,
            applied: Vec::new(),
        }
    }

    /// Pad the applied-amount plane when the layer grows (#1208).
    pub(crate) fn grow(
        &mut self,
        old_w: usize,
        old_h: usize,
        new_w: usize,
        new_h: usize,
        dx: usize,
        dy: usize,
    ) {
        if self.applied.is_empty() {
            return;
        }
        let mut applied = vec![0f32; new_w * new_h];
        for y in 0..old_h {
            let src = y * old_w;
            let dst = (dy + y) * new_w + dx;
            if src + old_w <= self.applied.len() && dst + old_w <= applied.len() {
                applied[dst..dst + old_w].copy_from_slice(&self.applied[src..src + old_w]);
            }
        }
        self.applied = applied;
    }

    /// Tone one dab centred at layer-local `(cx, cy)`, writing the retained
    /// native planes and the narrowed working image. Returns the layer-local
    /// rectangle changed. Toning changes colour, never coverage.
    ///
    /// `ponytail:` a 16-bit dab runs on one thread; the 8-bit path keeps its
    /// banded raster. A parallel native split can follow when a 16-bpc dab is
    /// measured too slow.
    pub(crate) fn dab(
        &mut self,
        layer: &mut Layer,
        pixels: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
        clip: Option<&Coverage>,
    ) -> Option<PsdRect> {
        let amount = self.options.amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return None;
        }
        let region = dab_bounds(pixels, cfg, cx, cy)?;
        if self.applied.is_empty() {
            self.applied = vec![0.0; pixels.data.len()];
        }
        let w = pixels.width as usize;
        let n = pixels.data.len();
        let store = layer.source_channels.as_mut()?;
        if store.depth != BitDepth::Sixteen || store.rect != layer.rect {
            return None;
        }
        let mut r: Option<&mut [u16]> = None;
        let mut g: Option<&mut [u16]> = None;
        let mut b: Option<&mut [u16]> = None;
        let mut a: Option<&mut [u16]> = None;
        for (id, samples) in store.planes.iter_mut() {
            let Samples::U16(plane) = samples else {
                return None;
            };
            if plane.len() != n {
                continue;
            }
            let plane = plane.as_mut_slice();
            match *id {
                0 if r.is_none() => r = Some(plane),
                1 if g.is_none() => g = Some(plane),
                2 if b.is_none() => b = Some(plane),
                -1 if a.is_none() => a = Some(plane),
                _ => {}
            }
        }
        let r = r?;
        if !self.gray {
            g.as_ref()?;
            b.as_ref()?;
        }
        let alpha = a.as_deref();
        let tip = TipParams::new(cfg);
        let options = self.options;
        let mut dirty: Option<PsdRect> = None;
        for row in region.top as usize..region.bottom as usize {
            let dy = row as f32 + 0.5 - cy;
            let Some((xlo, xhi)) = tip.x_span(dy) else {
                continue;
            };
            let x0 = ((cx + xlo).floor() as i32).max(region.left);
            let x1 = ((cx + xhi).ceil() as i32).min(region.right - 1);
            let mut first = None;
            let mut last = 0;
            for x in x0..=x1 {
                let at = row * w + x as usize;
                let src_alpha = alpha.map_or(pixels.data[at][3] as u16, |a| a[at]);
                if src_alpha == 0 {
                    continue;
                }
                let cover = tip.coverage(x as f32 + 0.5 - cx, dy);
                let weight = if options.airbrush {
                    cover * amount
                } else {
                    if cover <= self.applied[at] {
                        continue;
                    }
                    (cover - self.applied[at]) * amount
                };
                if weight <= 0.0 {
                    continue;
                }
                let src = match (g.as_deref(), b.as_deref()) {
                    (Some(g), Some(b)) => [
                        f32::from(r[at]) / 65535.0,
                        f32::from(g[at]) / 65535.0,
                        f32::from(b[at]) / 65535.0,
                    ],
                    _ => [f32::from(r[at]) / 65535.0, 0.0, 0.0],
                };
                let lifted = match options.tone {
                    Tone::Dodge | Tone::Burn => tone(src, weight, &options),
                    Tone::Sponge => sponge(src, weight, &options),
                };
                let q = |v: f32| (v.clamp(0.0, 1.0) * 65535.0).round() as u16;
                let out = [q(lifted[0]), q(lifted[1]), q(lifted[2])];
                let clip_cover = clip.map_or(255, |c| {
                    c.sample(layer.rect.left + x, layer.rect.top + row as i32)
                });
                if clip_cover == 0 {
                    continue;
                }
                let old = [
                    r[at],
                    g.as_deref().map_or(0, |g| g[at]),
                    b.as_deref().map_or(0, |b| b[at]),
                ];
                let mixed = [
                    mix(out[0], old[0], clip_cover),
                    mix(out[1], old[1], clip_cover),
                    mix(out[2], old[2], clip_cover),
                ];
                if mixed == old {
                    continue;
                }
                r[at] = mixed[0];
                let narrowed = (mixed[0] >> 8) as u8;
                let mut px = pixels.data[at];
                px[0] = narrowed;
                if let (Some(g), Some(b)) = (g.as_deref_mut(), b.as_deref_mut()) {
                    g[at] = mixed[1];
                    b[at] = mixed[2];
                    px[1] = (mixed[1] >> 8) as u8;
                    px[2] = (mixed[2] >> 8) as u8;
                }
                pixels.data[at] = px;
                if !options.airbrush {
                    self.applied[at] = cover;
                }
                first.get_or_insert(x);
                last = x;
            }
            if let Some(first) = first {
                let row_box = PsdRect {
                    top: row as i32,
                    left: first,
                    bottom: row as i32 + 1,
                    right: last + 1,
                };
                dirty = Some(dirty.map_or(row_box, |d| d.union(row_box)));
            }
        }
        dirty
    }
}

/// `new` under `cover` (0–255) over `old`, at 16 bits.
fn mix(new: u16, old: u16, cover: u8) -> u16 {
    if cover == 255 {
        return new;
    }
    let t = u32::from(cover);
    let inv = 255 - t;
    ((u32::from(new) * t + u32::from(old) * inv + 127) / 255) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::tests::doc;
    use crate::fill_native::tests::depth16_doc;
    use crate::tone::{SpongeMode, Tone, ToneRange};
    use crate::{PaintError, Stroke, StrokeSample};
    use pictura_core::{
        BitDepth, Channel, ColorMode, Document, Layer, LayerContent, LayerParts, LayerStack,
        PsdRect, Samples, SourceChannels, SourcePlanes,
    };

    fn plane16(doc: &Document, id: i16) -> Vec<u16> {
        let store = doc.layers[0].source_channels.as_ref().expect("a store");
        let (_, samples) = store
            .planes
            .iter()
            .find(|(cid, _)| *cid == id)
            .expect("the plane");
        let Samples::U16(values) = samples else {
            panic!("a u16 store")
        };
        values.to_vec()
    }

    fn channel8(doc: &Document, id: i16) -> Vec<u8> {
        doc.layers[0]
            .channels()
            .iter()
            .find(|c| c.id == id)
            .expect("the channel")
            .data
            .to_vec()
    }

    fn stroke(d: &Document, options: ToneOptions) -> Document {
        let cfg = StrokeConfig {
            diameter: 20,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_tone(d, "0", cfg, options).expect("begin");
        let mut out = d.clone();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.0,
                y: 20.0,
                pressure: 1.0,
            },
        );
        stroke.finish();
        out
    }

    fn dodge() -> ToneOptions {
        ToneOptions {
            tone: Tone::Dodge,
            range: ToneRange::Shadows,
            protect_tones: false,
            amount: 1.0,
            ..ToneOptions::default()
        }
    }

    fn burn() -> ToneOptions {
        ToneOptions {
            tone: Tone::Burn,
            ..dodge()
        }
    }

    fn gray16(w: i32, h: i32, value: u16) -> Document {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        };
        let plane = vec![value; (w * h) as usize];
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Grayscale, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![Layer::from_parts(
            LayerParts {
                rect,
                channels: vec![Channel {
                    id: 0,
                    data: Samples::U16(plane.clone().into()).narrow_to_u8().into(),
                }],
                source_channels: Some(SourceChannels::new(
                    BitDepth::Sixteen,
                    rect,
                    vec![(0, Samples::U16(plane.into()))],
                )),
                ..Default::default()
            },
            LayerContent::Pixel,
        )]);
        doc.source
            .set_native(Some(BitDepth::Sixteen), doc.source.planes().clone())
            .expect("valid source depth");
        doc
    }

    /// A 16-bit RGB document whose layer covers `rect`, not the whole canvas.
    fn depth16_rect_doc(canvas: i32, rect: PsdRect, value: u16) -> Document {
        let plane = vec![value; (rect.width() * rect.height()) as usize];
        let mut doc = Document::new(
            canvas as u32,
            canvas as u32,
            ColorMode::Rgb,
            BitDepth::Eight,
        );
        doc.layers = LayerStack::from_layers(vec![Layer::from_parts(
            LayerParts {
                rect,
                channels: [0i16, 1, 2]
                    .into_iter()
                    .map(|id| Channel {
                        id,
                        data: Samples::U16(plane.clone().into()).narrow_to_u8().into(),
                    })
                    .collect(),
                source_channels: Some(SourceChannels::new(
                    BitDepth::Sixteen,
                    rect,
                    [0i16, 1, 2]
                        .into_iter()
                        .map(|id| (id, Samples::U16(plane.clone().into())))
                        .collect(),
                )),
                ..Default::default()
            },
            LayerContent::Pixel,
        )]);
        doc.source
            .set_native(Some(BitDepth::Sixteen), doc.source.planes().clone())
            .expect("valid source depth");
        doc
    }

    #[test]
    fn a_depth16_dodge_writes_the_native_store() {
        let d = depth16_doc(40, 40, vec![1000; 1600], vec![1000; 1600], vec![1000; 1600]);
        let out = stroke(&d, dodge());
        let native = plane16(&out, 0);
        let at = 20 * 40 + 20;
        assert!(native[at] > 1000, "the native red did not lift");
        assert_eq!(channel8(&out, 0)[at], (native[at] >> 8) as u8);
    }

    #[test]
    fn a_depth16_burn_writes_the_native_store() {
        let d = depth16_doc(
            40,
            40,
            vec![20000; 1600],
            vec![20000; 1600],
            vec![20000; 1600],
        );
        let out = stroke(&d, burn());
        let native = plane16(&out, 0);
        let at = 20 * 40 + 20;
        assert!(native[at] < 20000, "the native red did not fall");
        assert_eq!(channel8(&out, 0)[at], (native[at] >> 8) as u8);
    }

    #[test]
    fn a_depth16_sponge_writes_the_native_store() {
        let d = depth16_doc(
            40,
            40,
            vec![20000; 1600],
            vec![8000; 1600],
            vec![8000; 1600],
        );
        let options = ToneOptions {
            tone: Tone::Sponge,
            sponge: SpongeMode::Saturate,
            vibrance: false,
            amount: 1.0,
            ..ToneOptions::default()
        };
        let out = stroke(&d, options);
        let at = 20 * 40 + 20;
        let spread = plane16(&out, 0)[at] as i32 - plane16(&out, 2)[at] as i32;
        assert!(spread > 12000, "the native spread did not rise: {spread}");
        assert_eq!(channel8(&out, 0)[at], (plane16(&out, 0)[at] >> 8) as u8);
        assert_eq!(channel8(&out, 2)[at], (plane16(&out, 2)[at] >> 8) as u8);
    }

    #[test]
    fn a_depth16_grayscale_tone_uses_the_single_plane() {
        let d = gray16(40, 40, 10000);
        let out = stroke(&d, dodge());
        let at = 20 * 40 + 20;
        let store = out.layers[0].source_channels.as_ref().expect("a store");
        assert_eq!(store.planes.len(), 1, "a colour plane was added");
        let native = plane16(&out, 0);
        assert!(native[at] > 10000, "the grey plane did not lift");
        assert_eq!(channel8(&out, 0)[at], (native[at] >> 8) as u8);
    }

    #[test]
    fn a_depth16_tone_is_not_the_8bit_widening() {
        let deep = depth16_doc(40, 40, vec![1000; 1600], vec![1000; 1600], vec![1000; 1600]);
        let eight = doc(40, 40, [3, 3, 3, 255]);
        let at = 20 * 40 + 20;
        let native = plane16(&stroke(&deep, dodge()), 0)[at];
        let shallow = channel8(&stroke(&eight, dodge()), 0)[at] as u16 * 257;
        assert_ne!(native, shallow, "the native result is the 8-bit widening");
    }

    #[test]
    fn cancelling_a_depth16_tone_restores_the_native_store() {
        let mut d = depth16_doc(40, 40, vec![1000; 1600], vec![1000; 1600], vec![1000; 1600]);
        let before = plane16(&d, 0);
        let cfg = StrokeConfig {
            diameter: 20,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_tone(&d, "0", cfg, dodge()).expect("begin");
        stroke.sample(
            &mut d,
            StrokeSample {
                x: 20.0,
                y: 20.0,
                pressure: 1.0,
            },
        );
        stroke.cancel(&mut d);
        assert_eq!(plane16(&d, 0), before);
    }

    #[test]
    fn a_depth16_tone_grows_and_cancel_restores_the_native_store() {
        let rect = PsdRect {
            top: 10,
            left: 10,
            bottom: 30,
            right: 30,
        };
        let mut d = depth16_rect_doc(40, rect, 1000);
        let before = plane16(&d, 0);
        let cfg = StrokeConfig {
            diameter: 20,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_tone(&d, "0", cfg, dodge()).expect("begin");
        stroke.sample(
            &mut d,
            StrokeSample {
                x: 30.0,
                y: 20.0,
                pressure: 1.0,
            },
        );
        let grown = d.layers[0].rect;
        assert!(
            grown.right > rect.right,
            "the stroke did not grow the layer"
        );
        let store = d.layers[0].source_channels.as_ref().expect("a store");
        let Samples::U16(plane) = &store.planes[0].1 else {
            panic!("a u16 store")
        };
        let (dx, dy) = (
            (rect.left - grown.left) as usize,
            (rect.top - grown.top) as usize,
        );
        assert_eq!(
            plane[dy * grown.width() as usize + dx],
            1000,
            "the grow lost the original native sample"
        );
        stroke.cancel(&mut d);
        assert_eq!(d.layers[0].rect, rect);
        assert_eq!(plane16(&d, 0), before);
    }

    #[test]
    fn a_retained_32bit_store_refuses_toning() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.source
            .set_native(
                Some(BitDepth::ThirtyTwo),
                Some(SourcePlanes {
                    depth: BitDepth::ThirtyTwo,
                    width: 2,
                    height: 2,
                    samples: Samples::F32(vec![0.5; 12].into()),
                }),
            )
            .expect("valid source depth");
        let begun = Stroke::begin_tone(&doc, "0", StrokeConfig::default(), ToneOptions::default());
        assert!(matches!(begun, Err(PaintError::UnsupportedDepth)));
    }
}
