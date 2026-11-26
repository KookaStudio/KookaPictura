//! Indexed document conversion and index-preserving color table edits.

use pictura_codec::{
    indexed_metadata, indexed_working_buffer, set_indexed_metadata, ColorTable, PaletteOptions,
};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerParts, LayerStack, PixelBuffer, PsdRect,
    Samples, SourceChannels, SourcePlanes,
};
use pictura_ops::OpsError;

use super::{can_convert_mode, planes};

fn invalid(message: &str) -> OpsError {
    OpsError::InvalidParams(message.into())
}

fn source_rgba(doc: &Document, transparency: bool) -> Option<Vec<u8>> {
    let (w, h) = (doc.width, doc.height);
    let n = w as usize * h as usize;
    if !transparency && !doc.layers.is_empty() {
        let mut flattened = doc.clone();
        crate::flatten(&mut flattened).ok()?;
        let channels = flattened.layers[0].channels();
        let [r, g, b] = [0, 1, 2].map(|id| channels.iter().find(|channel| channel.id == id));
        let (r, g, b) = (r?, g?, b?);
        return Some(
            (0..n)
                .flat_map(|i| [r.data[i], g.data[i], b.data[i], 255])
                .collect(),
        );
    }
    let rendered;
    let buffer = if doc.layers.is_empty() {
        doc.composite.buffer()
    } else {
        rendered = crate::composite_rgba(doc);
        &rendered
    };
    let rgba = match buffer.channels {
        1 if buffer.data.len() == n => (0..n)
            .flat_map(|i| [buffer.data[i], buffer.data[i], buffer.data[i], 255])
            .collect(),
        2 if buffer.data.len() == 2 * n => (0..n)
            .flat_map(|i| {
                [
                    buffer.data[i],
                    buffer.data[i],
                    buffer.data[i],
                    buffer.data[n + i],
                ]
            })
            .collect(),
        channels @ (3 | 4) if buffer.data.len() == usize::from(channels) * n => (0..n)
            .flat_map(|i| {
                [
                    buffer.data[i],
                    buffer.data[n + i],
                    buffer.data[2 * n + i],
                    if channels == 4 {
                        buffer.data[3 * n + i]
                    } else {
                        255
                    },
                ]
            })
            .collect(),
        _ => return None,
    };
    Some(rgba)
}

pub fn indexed_exact_fits(doc: &Document, options: PaletteOptions, forced: &[[u8; 3]]) -> bool {
    can_convert_mode(doc, ColorMode::Indexed)
        && source_rgba(doc, options.transparency).is_some_and(|rgba| {
            pictura_codec::indexed_document_exact_fits(
                &rgba, doc.width, doc.height, options, forced,
            )
        })
}

pub fn convert_to_indexed(doc: &mut Document, options: PaletteOptions) -> Result<(), OpsError> {
    convert_to_indexed_with_forced(doc, options, &[], false)
}

pub fn convert_to_indexed_with_forced(
    doc: &mut Document,
    options: PaletteOptions,
    forced: &[[u8; 3]],
    exact: bool,
) -> Result<(), OpsError> {
    if !can_convert_mode(doc, ColorMode::Indexed) || !doc.variables.protected_ids().is_empty() {
        return Err(OpsError::Unsupported(
            "cannot convert document to Indexed".into(),
        ));
    }
    let (width, height) = (doc.width, doc.height);
    let rgba = source_rgba(doc, options.transparency)
        .ok_or_else(|| invalid("Indexed source has no usable pixel data"))?;
    let indexed =
        pictura_codec::quantize_document_indexed(&rgba, width, height, options, forced, exact)
            .ok_or_else(|| {
                invalid("required colors and transparency exceed the indexed table capacity")
            })?;
    let table = ColorTable {
        colors: indexed
            .palette
            .iter()
            .map(|&[r, g, b, _]| [r, g, b])
            .collect(),
        transparent: indexed.transparent_index(),
    };
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    let indices = Samples::U8(indexed.indices.clone().into());
    let mut layer = pictura_codec::layer_from_parts(LayerParts {
        name: if table.transparent.is_some() {
            "Indexed Color"
        } else {
            "Background"
        }
        .into(),
        rect: canvas,
        visible: true,
        opacity: 255,
        fill: 255,
        background: table.transparent.is_none(),
        ..Default::default()
    });
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Eight,
        canvas,
        vec![(0, indices.clone())],
    ));

    planes::drop_source_state(doc);
    doc.source
        .set_native(None, None)
        .expect("valid 8-bit source");
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc.prune_variable_bindings();
    doc.mode = ColorMode::Rgb;
    doc.depth = BitDepth::Eight;
    doc.composite.set_signed_merged_alpha(false);
    doc.composite.set_merged_output(true);
    doc.source
        .replace_planes(Some(SourcePlanes {
            depth: BitDepth::Eight,
            width,
            height,
            samples: indices,
        }))
        .expect("valid indexed source");
    doc.source.set_working_profile(None);
    write_table(doc, &table, &indexed.indices, Some(&indexed.indices));
    Ok(())
}

/// Return the table only when it is safe to rebuild working pixels from the
/// corresponding retained indices.
pub fn indexed_color_table(doc: &Document) -> Option<ColorTable> {
    editable_indices(doc)?;
    let palette = doc.source.palette_ref()?;
    let (count, transparent) = indexed_metadata(doc);
    Some(ColorTable {
        colors: (0..count)
            .map(|i| [palette[i], palette[256 + i], palette[512 + i]])
            .collect(),
        transparent,
    })
}

pub fn indexed_color_table_minimum(doc: &Document) -> Option<usize> {
    let (composite, layer) = editable_indices(doc)?;
    Some(
        composite
            .iter()
            .chain(layer.into_iter().flatten())
            .copied()
            .max()
            .map_or(1, |i| usize::from(i) + 1),
    )
}

fn editable_indices(doc: &Document) -> Option<(&[u8], Option<&[u8]>)> {
    if !pictura_codec::gif_indexed_source_is_current(doc) {
        return None;
    }
    let n = doc.width as usize * doc.height as usize;
    let layer = match doc.layers.as_slice() {
        [] => None,
        [layer]
            if layer.visible
                && !layer.is_group()
                && layer.rect
                    == (PsdRect {
                        top: 0,
                        left: 0,
                        bottom: doc.height as i32,
                        right: doc.width as i32,
                    })
                && layer.opacity == 255
                && layer.fill == 255
                && layer.mask.is_none()
                && layer.vector_mask.is_none()
                && layer.blend == pictura_core::BlendMode::Normal =>
        {
            let source = layer.source_channels.as_ref()?;
            let (_, Samples::U8(indices)) = source.planes.iter().find(|(id, _)| *id == 0)? else {
                return None;
            };
            let indices = indices.as_slice().get(..n)?;
            let transparent = indexed_metadata(doc).1;
            if let Some(alpha) = layer.channels().iter().find(|channel| channel.id == -1) {
                if alpha.data.len() != n
                    || alpha
                        .data
                        .iter()
                        .zip(indices)
                        .any(|(&a, &i)| a != if Some(i) == transparent { 0 } else { 255 })
                {
                    return None;
                }
            }
            Some(indices)
        }
        _ => return None,
    };
    let composite = match doc.source.planes() {
        Some(source) if doc.composite.merged_output() => match &source.samples {
            Samples::U8(indices) => indices.as_slice().get(..n)?,
            _ => return None,
        },
        _ => layer?,
    };
    Some((composite, layer))
}

pub fn replace_indexed_color_table(
    doc: &mut Document,
    table: ColorTable,
) -> Result<bool, OpsError> {
    let old = indexed_color_table(doc).ok_or_else(|| invalid("no editable indexed table"))?;
    if old == table {
        return Ok(false);
    }
    if table.colors.is_empty()
        || table.colors.len() > 256
        || table
            .transparent
            .is_some_and(|i| usize::from(i) >= table.colors.len())
    {
        return Err(invalid("invalid indexed color table"));
    }
    let (composite, layer) =
        editable_indices(doc).ok_or_else(|| invalid("no editable indexed table"))?;
    if composite
        .iter()
        .chain(layer.into_iter().flatten())
        .any(|&i| usize::from(i) >= table.colors.len())
    {
        return Err(invalid("the color table omits a used index"));
    }
    let composite = composite.to_vec();
    let layer = layer.map(<[u8]>::to_vec);
    write_table(doc, &table, &composite, layer.as_deref());
    Ok(true)
}

fn write_table(doc: &mut Document, table: &ColorTable, composite: &[u8], layer: Option<&[u8]>) {
    let mut palette = [0; 768];
    for (i, &[r, g, b]) in table.colors.iter().enumerate() {
        palette[i] = r;
        palette[256 + i] = g;
        palette[512 + i] = b;
    }
    let (width, height) = (doc.width, doc.height);
    doc.composite
        .replace(indexed_working_buffer(
            width,
            height,
            composite,
            &palette,
            table.transparent,
        ))
        .expect("valid indexed composite");
    if let Some(indices) = layer {
        replace_layer_palette(
            &mut doc.layers[0],
            indices,
            &palette,
            table.transparent,
            width,
            height,
        );
    }
    doc.source
        .set_encoding(Some(ColorMode::Indexed), Some(palette))
        .expect("valid indexed table");
    set_indexed_metadata(doc, Some(table.colors.len() as u16), table.transparent);
}

fn replace_layer_palette(
    layer: &mut Layer,
    indices: &[u8],
    palette: &[u8; 768],
    transparent: Option<u8>,
    width: u32,
    height: u32,
) {
    let PixelBuffer { data, channels, .. } =
        indexed_working_buffer(width, height, indices, palette, transparent);
    let plane = indices.len();
    let mut updated: Vec<_> = [0, 1, 2, -1]
        .into_iter()
        .enumerate()
        .map(|(i, id)| Channel {
            id,
            data: if i < usize::from(channels) {
                data[i * plane..(i + 1) * plane].to_vec().into()
            } else {
                pictura_core::Plane::filled(width as usize, height as usize, 1, 255)
            },
        })
        .collect();
    updated.extend(
        layer
            .channels()
            .iter()
            .filter(|channel| !matches!(channel.id, 0..=2 | -1))
            .cloned(),
    );
    *layer.channels_mut() = updated;
    if transparent.is_some() {
        layer.background = false;
    }
}
