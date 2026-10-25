//! Plane-level helpers shared by the `Image > Mode` conversions: the working
//! Grayscale <-> RGB reshape, the source-mode (CMYK/Lab) stores, and the
//! retained native-depth stores a save re-emits.

use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, PixelBuffer, PsdRect, Sample, Samples,
    SourceChannels, SourcePlanes,
};

/// Apply `$body` to the vector inside a [`Samples`], rebuilding the same variant.
macro_rules! map_samples {
    ($samples:expr, |$v:ident| $body:expr) => {
        match $samples {
            Samples::U8($v) => Samples::U8($body),
            Samples::U16($v) => Samples::U16($body),
            Samples::F32($v) => Samples::F32($body),
        }
    };
}

/// Rec. 601 luma of one 8-bit pixel, rounded.
///
/// ponytail: CS6 converts through the Gray working space (Dot Gain 20%); the
/// weights are not published (IMG-004 marks them inferred), so this is the
/// photorust Rec. 601 sum with rounding instead of truncation.
pub(super) fn luma_u8(r: u8, g: u8, b: u8) -> u8 {
    ((299 * r as u32 + 587 * g as u32 + 114 * b as u32 + 500) / 1000) as u8
}

fn luma_unit<T: Sample>(r: T, g: T, b: T) -> T {
    T::from_unit(0.299 * r.to_unit() + 0.587 * g.to_unit() + 0.114 * b.to_unit())
}

/// `[Y, extras..]` -> `[Y, Y, Y, extras..]` for a plane-major store.
fn gray_store_to_rgb<T: Copy>(v: &[T], plane: usize) -> Vec<T> {
    [&v[..plane], &v[..plane], v].concat()
}

/// `[R, G, B, extras..]` -> `[Y, extras..]` for a plane-major store.
fn rgb_store_to_gray<T: Sample>(v: &[T], plane: usize) -> Vec<T> {
    let mut out: Vec<T> = (0..plane)
        .map(|i| luma_unit(v[i], v[plane + i], v[2 * plane + i]))
        .collect();
    out.extend_from_slice(&v[3 * plane..]);
    out
}

/// Visit every non-group layer in the tree, depth first.
pub(super) fn for_each_layer(
    layers: &mut pictura_core::LayerStack,
    f: &mut impl FnMut(&mut Layer),
) {
    for layer in layers {
        if layer.is_group() {
            for_each_layer(&mut layer.children, f);
        } else {
            f(layer);
        }
    }
}

/// The positions in `layer.channels` of color ids `0..n`, when each is present
/// once with a common length; `None` for an adjustment, group, or other layout.
pub(super) fn color_positions(layer: &Layer, n: i16) -> Option<Vec<usize>> {
    let positions: Vec<usize> = (0..n)
        .map(|id| layer.channels.iter().position(|c| c.id == id))
        .collect::<Option<_>>()?;
    let len = layer.channels[positions[0]].data.len();
    positions
        .iter()
        .all(|&i| layer.channels[i].data.len() == len)
        .then_some(positions)
}

fn layer_store_plane(store: &SourceChannels, id: i16) -> Option<&Samples> {
    store
        .planes
        .iter()
        .find(|(channel, _)| *channel == id)
        .map(|(_, s)| s)
}

/// Forget the mode a read (or an earlier conversion) recorded, with every store
/// and palette encoded in it, so the document saves as its working mode.
pub(super) fn drop_source_state(doc: &mut Document) {
    if doc.source_mode.take().is_some() {
        doc.source_planes = None;
        doc.color_mode_data.clear();
        drop_layer_stores(doc);
    }
    doc.source_palette = None;
}

pub(super) fn drop_layer_stores(doc: &mut Document) {
    for_each_layer(&mut doc.layers, &mut |layer| layer.source_channels = None);
}

/// Give a flat (layerless) document a Background layer built from its
/// composite, so the converted image has pixels to edit.
pub(super) fn background_from_composite(doc: &mut Document) {
    let plane = doc.width as usize * doc.height as usize;
    let colors = doc.mode.color_channels() as usize;
    let mut channels: Vec<Channel> = (0..colors)
        .map(|c| Channel {
            id: c as i16,
            data: doc
                .composite
                .data
                .get(c * plane..(c + 1) * plane)
                .map_or_else(|| vec![255; plane], <[u8]>::to_vec)
                .into(),
        })
        .collect();
    channels.push(Channel {
        id: -1,
        data: vec![255; plane].into(),
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Background".to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        channels,
        background: true,
        ..Default::default()
    }]);
}

/// Reshape a working Grayscale document to working RGB by replicating the gray
/// plane: composite, layer color channels, and any retained native store.
pub(super) fn expand_gray(doc: &mut Document) {
    if doc.mode != ColorMode::Grayscale {
        return;
    }
    let plane = doc.width as usize * doc.height as usize;
    let gray = doc
        .composite
        .data
        .get(..plane)
        .map_or_else(|| vec![0; plane], <[u8]>::to_vec);
    doc.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 3,
        data: [&gray[..], &gray[..], &gray[..]].concat().into(),
    };
    if let Some(store) = doc.source_planes.as_mut() {
        if store.samples.len() >= plane {
            store.samples = map_samples!(&store.samples, |v| gray_store_to_rgb(v, plane));
        }
    }
    for_each_layer(&mut doc.layers, &mut |layer| {
        let Some(pos) = color_positions(layer, 1) else {
            return;
        };
        if layer.channels.iter().any(|c| c.id == 1 || c.id == 2) {
            return;
        }
        let gray = layer.channels[pos[0]].data.clone();
        layer.channels.insert(
            pos[0] + 1,
            Channel {
                id: 1,
                data: gray.clone(),
            },
        );
        layer
            .channels
            .insert(pos[0] + 2, Channel { id: 2, data: gray });
        if let Some(store) = layer.source_channels.as_mut() {
            if let Some(y) = layer_store_plane(store, 0).cloned() {
                store.planes.push((1, y.clone()));
                store.planes.push((2, y));
                store.planes.sort_by_key(|(id, _)| *id);
            }
        }
    });
    doc.mode = ColorMode::Rgb;
}

/// Reshape a working RGB document to working Grayscale through [`luma_u8`]:
/// composite, layer color channels, and any retained native store (whose luma
/// is computed at its own depth, so a 16/32-bit document keeps its precision).
pub(super) fn reduce_to_gray(doc: &mut Document) {
    if doc.mode != ColorMode::Rgb {
        return;
    }
    let plane = doc.width as usize * doc.height as usize;
    let rgb = &doc.composite.data;
    let gray: Vec<u8> = if rgb.len() >= 3 * plane {
        (0..plane)
            .map(|i| luma_u8(rgb[i], rgb[plane + i], rgb[2 * plane + i]))
            .collect()
    } else {
        vec![0; plane]
    };
    doc.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 1,
        data: gray.into(),
    };
    if let Some(store) = doc.source_planes.as_mut() {
        if store.samples.len() >= 3 * plane {
            store.samples = map_samples!(&store.samples, |v| rgb_store_to_gray(v, plane));
        }
    }
    for_each_layer(&mut doc.layers, &mut |layer| {
        let Some(pos) = color_positions(layer, 3) else {
            return;
        };
        let (r, g, b) = (
            &layer.channels[pos[0]].data,
            &layer.channels[pos[1]].data,
            &layer.channels[pos[2]].data,
        );
        let gray: Vec<u8> = (0..r.len()).map(|i| luma_u8(r[i], g[i], b[i])).collect();
        layer.channels[pos[0]].data = gray.into();
        layer.channels.retain(|c| c.id != 1 && c.id != 2);
        if let Some(store) = layer.source_channels.as_mut() {
            let planes: Option<Vec<Samples>> = (0..3)
                .map(|id| layer_store_plane(store, id).cloned())
                .collect();
            if let Some(planes) = planes {
                let n = planes[0].len();
                let flat = match (&planes[0], &planes[1], &planes[2]) {
                    (Samples::U8(r), Samples::U8(g), Samples::U8(b)) => {
                        Samples::U8(rgb_store_to_gray(&[&r[..], g, b].concat(), n))
                    }
                    (Samples::U16(r), Samples::U16(g), Samples::U16(b)) => {
                        Samples::U16(rgb_store_to_gray(&[&r[..], g, b].concat(), n))
                    }
                    (Samples::F32(r), Samples::F32(g), Samples::F32(b)) => {
                        Samples::F32(rgb_store_to_gray(&[&r[..], g, b].concat(), n))
                    }
                    _ => return,
                };
                store.planes.retain(|(id, _)| !(0..3).contains(id));
                store.planes.push((0, flat));
                store.planes.sort_by_key(|(id, _)| *id);
            }
        }
    });
    doc.mode = ColorMode::Grayscale;
    // An RGB working profile cannot describe gray numbers.
    doc.document_icc = None;
}

/// The 8-bit source-mode planes for working RGB `rgb` (plane-major), and the
/// working RGB they display as.
fn encode_source_mode(
    mode: ColorMode,
    rgb: &[u8],
    cmyk_profile: Option<&pictura_codec::Profile>,
    options: pictura_codec::ConversionOptions,
) -> (Vec<u8>, Vec<u8>) {
    let encoded = match mode {
        ColorMode::Cmyk => cmyk_profile
            .and_then(|profile| pictura_codec::rgb_to_cmyk_with_profile(rgb, profile, options))
            .unwrap_or_else(|| rgb_to_cmyk_gcr(rgb)),
        _ => pictura_codec::rgb_to_lab(rgb),
    };
    let shown = match mode {
        ColorMode::Cmyk => cmyk_profile
            .and_then(|profile| pictura_codec::cmyk_to_rgb_with_profile(&encoded, profile, options))
            .unwrap_or_else(|| pictura_codec::cmyk_to_rgb(&encoded)),
        _ => pictura_codec::lab_to_rgb(&encoded),
    };
    (encoded, shown)
}

/// RGB -> CMYK with full black generation (photorust's naive GCR: `K = 1 -
/// max(R, G, B)`, the remaining ink scaled by `1 / (1 - K)`), in the codec's
/// stored convention (255 = no ink). Pure black is `C = M = Y = 255, K = 0`.
///
/// ponytail: no ICC profile, UCR/GCR curve, ink limit, or gamut mapping; every
/// RGB color is representable, so out-of-gamut colors do not clip as in CS6.
pub(super) fn rgb_to_cmyk_gcr(rgb: &[u8]) -> Vec<u8> {
    let plane = rgb.len() / 3;
    let mut out = vec![255u8; plane * 4];
    for i in 0..plane {
        let (r, g, b) = (rgb[i], rgb[plane + i], rgb[2 * plane + i]);
        let max = r.max(g).max(b);
        out[3 * plane + i] = max;
        if max == 0 {
            continue;
        }
        let ink = |v: u8| ((v as u32 * 255 + max as u32 / 2) / max as u32) as u8;
        out[i] = ink(r);
        out[plane + i] = ink(g);
        out[2 * plane + i] = ink(b);
    }
    out
}

/// Encode a working RGB document in `mode` (CMYK or Lab): each color plane set
/// gains an 8-bit store in that mode and the working RGB becomes what the
/// store displays as, so a save re-emits the store and the canvas matches it.
pub(super) fn encode_document(
    doc: &mut Document,
    mode: ColorMode,
    cmyk_profile: Option<&pictura_codec::Profile>,
    options: pictura_codec::ConversionOptions,
) {
    let plane = doc.width as usize * doc.height as usize;
    if doc.composite.data.len() >= 3 * plane {
        let (encoded, shown) = encode_source_mode(
            mode,
            &doc.composite.data[..3 * plane],
            cmyk_profile,
            options,
        );
        let mut data = doc.composite.data.to_vec();
        data[..3 * plane].copy_from_slice(&shown);
        doc.composite.data = data.into();
        doc.source_planes = Some(SourcePlanes {
            depth: BitDepth::Eight,
            width: doc.width,
            height: doc.height,
            samples: Samples::U8(encoded),
        });
    }
    for_each_layer(&mut doc.layers, &mut |layer| {
        encode_layer(layer, mode, true, cmyk_profile, options)
    });
    doc.source_mode = Some(mode);
}

/// Store `layer`'s color planes encoded in `mode`; with `show`, also rewrite
/// its working RGB to what the encoding displays as.
fn encode_layer(
    layer: &mut Layer,
    mode: ColorMode,
    show: bool,
    cmyk_profile: Option<&pictura_codec::Profile>,
    options: pictura_codec::ConversionOptions,
) {
    let Some(pos) = color_positions(layer, 3) else {
        return;
    };
    let n = layer.channels[pos[0]].data.len();
    let rgb: Vec<u8> = pos
        .iter()
        .flat_map(|&i| layer.channels[i].data.iter().copied())
        .collect();
    let (encoded, shown) = encode_source_mode(mode, &rgb, cmyk_profile, options);
    if show {
        for (c, &i) in pos.iter().enumerate() {
            layer.channels[i].data = shown[c * n..(c + 1) * n].to_vec().into();
        }
    }
    let planes = (0..encoded.len() / n.max(1))
        .map(|c| (c as i16, Samples::U8(encoded[c * n..(c + 1) * n].to_vec())))
        .collect();
    layer.source_channels = Some(SourceChannels::new(BitDepth::Eight, layer.rect, planes));
}

/// Make sure the composite and every color layer carry an 8-bit store in the
/// output mode (the working planes for Grayscale/RGB, the encoded planes for
/// CMYK/Lab), building any that are missing. The step before widening.
pub(super) fn ensure_stores(doc: &mut Document) {
    let plane = doc.width as usize * doc.height as usize;
    match doc.source_mode {
        Some(mode @ (ColorMode::Cmyk | ColorMode::Lab)) => {
            if doc.source_planes.is_none() && doc.composite.data.len() >= 3 * plane {
                let (encoded, _) = encode_source_mode(
                    mode,
                    &doc.composite.data[..3 * plane],
                    None,
                    pictura_codec::ConversionOptions {
                        dither: false,
                        ..Default::default()
                    },
                );
                doc.source_planes = Some(SourcePlanes {
                    depth: BitDepth::Eight,
                    width: doc.width,
                    height: doc.height,
                    samples: Samples::U8(encoded),
                });
            }
            for_each_layer(&mut doc.layers, &mut |layer| {
                if layer.source_channels.is_none() {
                    encode_layer(
                        layer,
                        mode,
                        false,
                        None,
                        pictura_codec::ConversionOptions {
                            dither: false,
                            ..Default::default()
                        },
                    );
                }
            });
        }
        _ => {
            let colors = doc.mode.color_channels() as usize;
            if doc.source_planes.is_none() && doc.composite.data.len() >= colors * plane {
                doc.source_planes = Some(SourcePlanes {
                    depth: BitDepth::Eight,
                    width: doc.width,
                    height: doc.height,
                    samples: Samples::U8(doc.composite.data[..colors * plane].to_vec()),
                });
            }
            for_each_layer(&mut doc.layers, &mut |layer| {
                if layer.source_channels.is_some() {
                    return;
                }
                let Some(pos) = color_positions(layer, colors as i16) else {
                    return;
                };
                let planes = pos
                    .iter()
                    .map(|&i| {
                        let c = &layer.channels[i];
                        (c.id, Samples::U8(c.data.to_vec()))
                    })
                    .collect();
                layer.source_channels =
                    Some(SourceChannels::new(BitDepth::Eight, layer.rect, planes));
            });
        }
    }
}

/// Re-express every retained store at `depth`: an 8-bit store widens
/// (`v * 257` / `v / 255`), a 16-bit store becomes `f32` through the unit
/// domain, and a store narrows to 8-bit with the shared `narrow_to_u8` rule.
pub(super) fn restore_depth(doc: &mut Document, depth: BitDepth) {
    let convert = |samples: &Samples| -> Samples {
        match depth {
            BitDepth::Eight => Samples::U8(samples.narrow_to_u8()),
            BitDepth::Sixteen => Samples::U16(samples.to_u16()),
            BitDepth::ThirtyTwo => match samples {
                Samples::U8(v) => Samples::widen_from_u8(v, depth),
                Samples::U16(v) => Samples::F32(v.iter().map(|&s| s.to_unit() as f32).collect()),
                Samples::F32(v) => Samples::F32(v.clone()),
            },
            BitDepth::One => samples.clone(),
        }
    };
    if let Some(store) = doc.source_planes.as_mut() {
        if store.depth != BitDepth::One {
            store.samples = convert(&store.samples);
            store.depth = depth;
        }
    }
    for_each_layer(&mut doc.layers, &mut |layer| {
        if let Some(store) = layer.source_channels.as_mut() {
            for (_, samples) in &mut store.planes {
                *samples = convert(samples);
            }
            store.depth = depth;
        }
    });
}
