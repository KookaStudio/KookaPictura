//! The native-depth fill: the same pass as `fill.rs`, run over a layer's
//! retained native store, so the pixels the compositor reads are the ones the
//! fill writes.
//!
//! A 16- or 32-bit layer holds its pixels twice: in the retained native
//! store the compositor reads, and in the 8-bit channels the writer narrows
//! them to. The fill writes the store and re-narrows the channels, so the two
//! agree and a save re-emits the native samples.

use crate::fill::{fill_planes, Alpha};
use crate::stroke::{layer_at, layer_at_mut, parse_layer_path};
use crate::{PaintMode, Rgba};
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, Document, Layer, PsdRect, Sample,
    Samples,
};

/// Where the layer's pixels live.
pub(crate) enum NativePlan {
    /// The retained store holds every colour plane: the native engine fills
    /// it, at the store's depth.
    Store(BitDepth),
    /// The retained store holds none of them: the layer's 8-bit channels are
    /// the pixels the compositor reads.
    Copy,
    /// The retained store holds some of them: the compositor reads the store
    /// for the planes it holds, so a fill of the 8-bit channels would reach
    /// only part of the layer. The fill is refused.
    Refused,
}

/// The store that holds the layer's pixels, when the document has one the
/// compositor reads.
pub(crate) fn native_plan(doc: &Document, path: &str, gray: bool) -> NativePlan {
    let Some(indices) = parse_layer_path(path) else {
        return NativePlan::Copy;
    };
    let Some(layer) = layer_at(doc, &indices) else {
        return NativePlan::Copy;
    };
    let Some(depth) = doc.source_depth else {
        return NativePlan::Copy;
    };
    let colors = if gray { 1 } else { 3 };
    let mut owned = 0;
    for id in 0..colors as i16 {
        if doc.authoritative_plane(layer, id).is_some() {
            owned += 1;
        }
    }
    match owned {
        0 => NativePlan::Copy,
        n if n == colors => NativePlan::Store(depth),
        _ => NativePlan::Refused,
    }
}

/// Fill the layer's native store at the store's own depth.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_native(
    doc: &mut Document,
    path: &str,
    depth: BitDepth,
    mode: PaintMode,
    selection: Option<&[u8]>,
    alpha: Alpha,
    gray: bool,
    source: impl Fn(i32, i32) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    match depth {
        BitDepth::Sixteen => fill_at::<u16>(doc, path, depth, mode, selection, alpha, gray, source),
        BitDepth::ThirtyTwo => {
            fill_at::<f32>(doc, path, depth, mode, selection, alpha, gray, source)
        }
        _ => None,
    }
}

/// Fill the layer's native store, whose planes are at the store's own width.
#[allow(clippy::too_many_arguments)]
fn fill_at<T: Sample + PartialEq + Send + Sync>(
    doc: &mut Document,
    path: &str,
    depth: BitDepth,
    mode: PaintMode,
    selection: Option<&[u8]>,
    alpha: Alpha,
    gray: bool,
    source: impl Fn(i32, i32) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    let canvas = (doc.width as i32, doc.height as i32);
    if selection.is_some_and(|s| s.len() < (canvas.0 * canvas.1) as usize) {
        return None;
    }
    let layer = layer_at_mut(doc, &parse_layer_path(path)?)?;
    if layer.is_group || layer.adjustment.is_some() || layer_pixel_locked(layer) {
        return None;
    }
    let keep_alpha = layer_transparency_locked(layer);
    if keep_alpha && mode == PaintMode::Clear {
        return None;
    }
    // A transparency lock is the user's instruction about the layer's
    // transparency, so it outranks the caller's policy.
    let policy = if keep_alpha { Alpha::Keep } else { alpha };
    let rect = layer.rect;
    let n = (rect.width().max(0) * rect.height().max(0)) as usize;
    // The store's planes, borrowed mutably: the fill holds them at once.
    let Layer {
        source_channels,
        channels,
        ..
    } = layer;
    let store = source_channels.as_mut()?;
    let mut r: Option<&mut [T]> = None;
    let mut g: Option<&mut [T]> = None;
    let mut b: Option<&mut [T]> = None;
    let mut a: Option<&mut [T]> = None;
    for (id, samples) in store.planes.iter_mut() {
        let plane = T::as_slice_mut(samples);
        match *id {
            0 if r.is_none() => r = plane,
            1 if g.is_none() => g = plane,
            2 if b.is_none() => b = plane,
            -1 if a.is_none() => a = plane,
            _ => {}
        }
    }
    let r = r?;
    if r.len() != n {
        return None;
    }
    let (g, b) = if gray {
        (None, None)
    } else {
        let (Some(g), Some(b)) = (g, b) else {
            return None;
        };
        if g.len() != n || b.len() != n {
            return None;
        }
        (Some(g), Some(b))
    };
    let store_alpha = a.is_some();
    if let Some(a) = a.as_ref() {
        if a.len() != n {
            return None;
        }
    }
    // The transparency plane: the store's own when it has one, else the
    // layer's 8-bit channel widened to the store's width.
    let mut widened = Samples::U8(Vec::new());
    let alpha_plane: Option<&mut [T]> = match a {
        Some(a) => Some(a),
        None => match channels.iter().find(|c| c.id == -1) {
            Some(channel) => {
                widened = Samples::widen_from_u8(channel.data.as_slice(), depth);
                T::as_slice_mut(&mut widened)
            }
            // No transparency plane anywhere: the layer is opaque.
            None => None,
        },
    };
    let dirty = fill_planes(
        r,
        g,
        b,
        alpha_plane,
        rect,
        canvas,
        mode,
        selection,
        policy,
        gray,
        &source,
        blend_native::<T>,
    );
    // The 8-bit channels are the store's narrowing, so the working copy and
    // the store agree and the writer emits the store's samples.
    for (id, samples) in store.planes.iter() {
        let Some(channel) = channels.iter_mut().find(|c| c.id == *id) else {
            continue;
        };
        let narrowed = samples.narrow_to_u8();
        if narrowed.len() == channel.data.len() {
            channel.data = narrowed.into();
        }
    }
    if !store_alpha {
        // The transparency came from the 8-bit channel; narrow it back.
        if let Some(channel) = channels.iter_mut().find(|c| c.id == -1) {
            let narrowed = widened.narrow_to_u8();
            if narrowed.len() == channel.data.len() {
                channel.data = narrowed.into();
            }
        }
    }
    dirty
}

/// The brush-mode blend at the planes' own width: the destination's samples
/// and the source colour, mixed in the unit domain.
fn blend_native<T: Sample>(
    mode: PaintMode,
    dst: (T, T, T, T),
    s: &Rgba,
    a: f32,
    roll: f32,
) -> Option<(T, T, T, T)> {
    let (dr, dg, db, da) = (
        dst.0.to_unit(),
        dst.1.to_unit(),
        dst.2.to_unit(),
        dst.3.to_unit(),
    );
    let (sr, sg, sb, sa) = (
        f64::from(s.r) / 255.0,
        f64::from(s.g) / 255.0,
        f64::from(s.b) / 255.0,
        f64::from(s.a) / 255.0,
    );
    let strength = f64::from(a);
    let dst = (dr, dg, db, da);
    let src = (sr, sg, sb, sa);
    let out = match mode {
        PaintMode::Normal => Some(normal_unit(dst, src, strength)),
        PaintMode::Multiply => {
            let m = |src: f64, dst: f64| (1.0 - da) * src + da * src * dst;
            Some(normal_unit(
                dst,
                (m(sr, dr), m(sg, dg), m(sb, db), sa),
                strength,
            ))
        }
        PaintMode::Dissolve => (roll < a).then_some(src),
        PaintMode::Behind => (da != 1.0).then_some(behind_unit(dst, src, strength)),
        PaintMode::Clear => Some((dr, dg, db, da * (1.0 - strength))),
    }?;
    Some((
        T::from_unit(out.0),
        T::from_unit(out.1),
        T::from_unit(out.2),
        T::from_unit(out.3),
    ))
}

/// The Normal brush mode in the unit domain: the source over the backdrop.
fn normal_unit(
    dst: (f64, f64, f64, f64),
    src: (f64, f64, f64, f64),
    strength: f64,
) -> (f64, f64, f64, f64) {
    let (dr, dg, db, da) = dst;
    let (sr, sg, sb, sa) = src;
    let src_a = sa * strength;
    let out_a = src_a + da * (1.0 - src_a);
    if out_a <= 0.0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mix = |src: f64, dst: f64| (src * src_a + dst * da * (1.0 - src_a)) / out_a;
    (mix(sr, dr), mix(sg, dg), mix(sb, db), out_a)
}

/// The Behind brush mode in the unit domain: the source under the backdrop.
///
/// The source is laid behind the backdrop, so the mix weights the backdrop by
/// its own alpha and the source by what is left of it.
fn behind_unit(
    dst: (f64, f64, f64, f64),
    src: (f64, f64, f64, f64),
    strength: f64,
) -> (f64, f64, f64, f64) {
    let (dr, dg, db, da) = dst;
    let (sr, sg, sb, sa) = src;
    let src_a = sa * strength;
    let out_a = da + src_a * (1.0 - da);
    if out_a <= 0.0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mix = |src: f64, dst: f64| (dst * da + src * src_a * (1.0 - da)) / out_a;
    (mix(sr, dr), mix(sg, dg), mix(sb, db), out_a)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::fill::fill_layer;
    use crate::fill::tests::RED;
    use pictura_core::{
        Channel, ColorMode, Layer, LayerStack, LockFlags, PsdRect, Samples, SourceChannels,
    };

    /// A `w × h` document whose single layer retains a 16-bit native store of
    /// the three colour planes, whose 8-bit channels are the narrowing of it.
    pub(crate) fn depth16_doc(
        w: i32,
        h: i32,
        red: Vec<u16>,
        green: Vec<u16>,
        blue: Vec<u16>,
    ) -> Document {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        };
        let planes = [red.clone(), green.clone(), blue.clone()];
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![Layer {
            rect,
            channels: planes
                .iter()
                .enumerate()
                .map(|(i, p)| Channel {
                    id: i as i16,
                    data: Samples::U16(p.clone()).narrow_to_u8().into(),
                })
                .collect(),
            source_channels: Some(SourceChannels::new(
                BitDepth::Sixteen,
                rect,
                vec![
                    (0, Samples::U16(red)),
                    (1, Samples::U16(green)),
                    (2, Samples::U16(blue)),
                ],
            )),
            ..Default::default()
        }]);
        doc.source_depth = Some(BitDepth::Sixteen);
        doc
    }

    /// A `w × h` document whose single layer retains a 32-bit native store of
    /// the three colour planes, whose 8-bit channels are the narrowing of it.
    fn depth32_doc(w: i32, h: i32, red: Vec<f32>, green: Vec<f32>, blue: Vec<f32>) -> Document {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        };
        let planes = [red.clone(), green.clone(), blue.clone()];
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![Layer {
            rect,
            channels: planes
                .iter()
                .enumerate()
                .map(|(i, p)| Channel {
                    id: i as i16,
                    data: Samples::F32(p.clone()).narrow_to_u8().into(),
                })
                .collect(),
            source_channels: Some(SourceChannels::new(
                BitDepth::ThirtyTwo,
                rect,
                vec![
                    (0, Samples::F32(red)),
                    (1, Samples::F32(green)),
                    (2, Samples::F32(blue)),
                ],
            )),
            ..Default::default()
        }]);
        doc.source_depth = Some(BitDepth::ThirtyTwo);
        doc
    }

    /// The store's colour plane for `id`, at 32 bits.
    fn plane32(doc: &Document, id: i16) -> Vec<f32> {
        let store = doc.layers[0].source_channels.as_ref().expect("a store");
        let (_, samples) = store
            .planes
            .iter()
            .find(|(cid, _)| *cid == id)
            .expect("the plane");
        let Samples::F32(values) = samples else {
            panic!("an f32 store")
        };
        values.clone()
    }

    /// The store's colour plane for `id`.
    fn plane(doc: &Document, id: i16) -> Vec<u16> {
        let store = doc.layers[0].source_channels.as_ref().expect("a store");
        let (_, samples) = store
            .planes
            .iter()
            .find(|(cid, _)| *cid == id)
            .expect("the plane");
        let Samples::U16(values) = samples else {
            panic!("a u16 store")
        };
        values.clone()
    }

    #[test]
    fn a_depth16_fill_writes_the_native_store() {
        let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        assert_eq!(plane(&doc, 0), vec![220 * 257; 4]);
        assert_eq!(plane(&doc, 1), vec![0; 4]);
        // The 8-bit channels are the store's narrowing.
        assert_eq!(doc.layers[0].channels[0].data.as_slice(), &[220; 4]);
    }

    #[test]
    fn a_depth16_fill_blends_at_the_stores_depth() {
        // A backdrop that is not 8-bit-aligned: 0x8000 narrows to 0x80.
        let backdrop = vec![32768u16; 4];
        // The same fill through the 8-bit path, over the narrowed backdrop:
        // the copy is taken before the native fill changes the layer.
        let mut eight = depth16_doc(2, 2, backdrop.clone(), backdrop.clone(), backdrop.clone());
        eight.layers[0].source_channels = None;
        fill_layer(
            &mut eight,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 0.5)),
        );
        let eight_red = eight.layers[0].channels[0].data[0];
        assert_eq!(eight_red, 174);
        let mut doc = depth16_doc(2, 2, backdrop.clone(), backdrop.clone(), backdrop);
        fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 0.5)),
        );
        assert_ne!(
            plane(&doc, 0)[0],
            eight_red as u16 * 257,
            "the native blend is the 8-bit widening"
        );
    }

    #[test]
    fn a_depth32_fill_writes_the_native_store() {
        let mut doc = depth32_doc(2, 2, vec![0.0; 4], vec![0.0; 4], vec![0.0; 4]);
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        let red = plane32(&doc, 0);
        assert!(
            (red[0] - 220.0 / 255.0).abs() < 1e-6,
            "the fill's samples: {red:?}"
        );
        // The 8-bit channels are the store's narrowing.
        assert_eq!(doc.layers[0].channels[0].data.as_slice(), &[220; 4]);
    }

    #[test]
    fn a_depth32_fill_blends_at_the_stores_depth() {
        // A backdrop that is not 8-bit-aligned: 0.5 is not a multiple of
        // 1/255, and its narrowing is 128.
        let backdrop = vec![0.5f32; 4];
        // The same fill through the 8-bit path, over the narrowed backdrop.
        let mut eight = depth32_doc(2, 2, backdrop.clone(), backdrop.clone(), backdrop.clone());
        eight.layers[0].source_channels = None;
        fill_layer(
            &mut eight,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 0.5)),
        );
        let eight_red = eight.layers[0].channels[0].data[0];
        assert_eq!(eight_red, 174);
        let mut doc = depth32_doc(2, 2, backdrop.clone(), backdrop.clone(), backdrop);
        fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 0.5)),
        );
        let native = plane32(&doc, 0)[0];
        assert!(
            (native - eight_red as f32 / 255.0).abs() > 1e-4,
            "the native blend is the 8-bit widening: {native}"
        );
    }

    #[test]
    fn a_native_fill_refuses_a_locked_layer() {
        let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
        doc.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_none(), "a locked layer was filled");
        assert_eq!(plane(&doc, 0), vec![0; 4]);
    }

    #[test]
    fn a_native_blend_matches_the_eight_bit_modes() {
        // A half-transparent backdrop, so Behind, Multiply and Clear differ
        // from Normal.
        let white = Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        for mode in [
            PaintMode::Normal,
            PaintMode::Behind,
            PaintMode::Clear,
            PaintMode::Multiply,
        ] {
            let mut eight = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
            eight.layers[0].channels.push(Channel {
                id: -1,
                data: vec![128; 4].into(),
            });
            eight.layers[0].source_channels = None;
            fill_layer(&mut eight, "0", mode, None, Alpha::Blend, |_, _| {
                Some((white, 1.0))
            });
            let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
            doc.layers[0].channels.push(Channel {
                id: -1,
                data: vec![128; 4].into(),
            });
            fill_layer(&mut doc, "0", mode, None, Alpha::Blend, |_, _| {
                Some((white, 1.0))
            });
            // The native colour is within one 8-bit step of the 8-bit path's.
            let eight_red = eight.layers[0].channels[0].data[0] as i32 * 257;
            let native_red = plane(&doc, 0)[0] as i32;
            assert!(
                (native_red - eight_red).abs() <= 257,
                "mode {mode:?}: the native blend is {native_red}, the 8-bit one {eight_red}"
            );
        }
    }

    #[test]
    fn a_native_dissolve_picks_the_same_pixels_as_the_eight_bit_path() {
        // The dissolve's roll is the same noise in both paths, over the same
        // coordinates, so the same pixels are picked.
        let see_through = Rgba {
            r: 220,
            g: 0,
            b: 0,
            a: 128,
        };
        let white = vec![65535u16; 16];
        let mut eight = depth16_doc(4, 4, white.clone(), white.clone(), white.clone());
        eight.layers[0].source_channels = None;
        fill_layer(
            &mut eight,
            "0",
            PaintMode::Dissolve,
            None,
            Alpha::Blend,
            |_, _| Some((see_through, 1.0)),
        );
        let mut doc = depth16_doc(4, 4, white.clone(), white.clone(), white);
        fill_layer(
            &mut doc,
            "0",
            PaintMode::Dissolve,
            None,
            Alpha::Blend,
            |_, _| Some((see_through, 1.0)),
        );
        let eight_red = eight.layers[0].channels[0].data.as_slice();
        let native_red = plane(&doc, 0);
        for (i, &value) in native_red.iter().enumerate() {
            assert_eq!(
                value != 65535,
                eight_red[i] != 255,
                "pixel {i}: the dissolve picked a different one"
            );
        }
    }

    #[test]
    fn the_depth16_mirror_is_the_stores_narrowing() {
        let mut doc = depth16_doc(2, 2, vec![1000; 4], vec![2000; 4], vec![3000; 4]);
        fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        let store = doc.layers[0].source_channels.as_ref().expect("a store");
        for (id, samples) in &store.planes {
            let channel = doc.layers[0]
                .channels
                .iter()
                .find(|c| c.id == *id)
                .expect("the channel");
            assert_eq!(channel.data.as_slice(), samples.narrow_to_u8());
        }
    }

    #[test]
    fn a_store_without_a_transparency_plane_still_fills() {
        let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
        // The layer has an 8-bit transparency the store does not carry.
        doc.layers[0].channels.push(Channel {
            id: -1,
            data: vec![128; 4].into(),
        });
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        // The colour came from the store.
        assert_eq!(plane(&doc, 0), vec![220 * 257; 4]);
        // The transparency came from the 8-bit channel, and the store has none.
        assert_eq!(doc.layers[0].channels[3].data.as_slice(), &[255; 4]);
        assert!(!doc.layers[0]
            .source_channels
            .as_ref()
            .expect("a store")
            .planes
            .iter()
            .any(|(id, _)| *id == -1));
    }

    #[test]
    fn a_mismatched_store_falls_back_to_the_eight_bit_path() {
        let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
        // The store's rectangle does not match the layer's.
        let mut store = doc.layers[0].source_channels.take().expect("a store");
        store.rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        };
        doc.layers[0].source_channels = Some(store);
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        // The 8-bit channels took the colour; the store did not.
        assert_eq!(doc.layers[0].channels[0].data.as_slice(), &[220; 4]);
        assert_eq!(plane(&doc, 0), vec![0; 4]);
    }

    #[test]
    fn a_keep_policy_keeps_the_destination_alpha_at_depth_16() {
        let mut doc = depth16_doc(2, 1, vec![0; 2], vec![0; 2], vec![0; 2]);
        // The layer's 8-bit transparency: one half transparent pixel and one
        // empty one. The store carries no transparency plane of its own.
        doc.layers[0].channels.push(Channel {
            id: -1,
            data: vec![128, 0].into(),
        });
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Keep,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        // The half transparent pixel kept its alpha; the empty one was not
        // filled.
        assert_eq!(doc.layers[0].channels[3].data.as_slice(), &[128, 0]);
    }

    #[test]
    fn a_replace_policy_takes_the_sources_alpha_at_depth_16() {
        let mut doc = depth16_doc(2, 1, vec![0; 2], vec![0; 2], vec![0; 2]);
        doc.layers[0].channels.push(Channel {
            id: -1,
            data: vec![255; 2].into(),
        });
        let see_through = Rgba {
            r: 220,
            g: 0,
            b: 0,
            a: 0,
        };
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Replace,
            |_, _| Some((see_through, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        // The source's alpha is the pixel's alpha, at the store's width.
        assert_eq!(doc.layers[0].channels[3].data.as_slice(), &[0, 0]);
    }

    #[test]
    fn a_converted_mode_document_fills_the_eight_bit_path() {
        let mut doc = depth16_doc(2, 2, vec![0; 4], vec![0; 4], vec![0; 4]);
        // A converted colour mode's store holds source-mode planes, not the
        // working copy the compositor reads.
        doc.source_mode = Some(ColorMode::Cmyk);
        let dirty = fill_layer(
            &mut doc,
            "0",
            PaintMode::Normal,
            None,
            Alpha::Blend,
            |_, _| Some((RED, 1.0)),
        );
        assert!(dirty.is_some(), "the fill changed the layer");
        assert_eq!(doc.layers[0].channels[0].data.as_slice(), &[220; 4]);
        assert_eq!(plane(&doc, 0), vec![0; 4]);
    }
}
