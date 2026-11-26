//! Separate the flattened source components into named spot plates.

use pictura_codec::{ChannelDisplayInfo, ConversionOptions, SPOT_CHANNEL_MODE};
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerStack, Samples, SourcePlanes};
use pictura_ops::OpsError;

use super::planes;

const CYAN: (&str, [u8; 3]) = ("Cyan", [0, 255, 255]);
const MAGENTA: (&str, [u8; 3]) = ("Magenta", [255, 0, 255]);
const YELLOW: (&str, [u8; 3]) = ("Yellow", [255, 255, 0]);
const BLACK: (&str, [u8; 3]) = ("Black", [0, 0, 0]);

fn source_inks(mode: ColorMode) -> &'static [(&'static str, [u8; 3])] {
    match mode {
        ColorMode::Rgb => &[CYAN, MAGENTA, YELLOW],
        ColorMode::Cmyk => &[CYAN, MAGENTA, YELLOW, BLACK],
        ColorMode::Grayscale => &[BLACK],
        // ponytail: lightness and opponent axes retain their samples; the
        // display inks approximate their appearance as separate spot plates.
        ColorMode::Lab => &[
            ("Lightness", [0, 0, 0]),
            ("a", [0, 160, 0]),
            ("b", [0, 0, 220]),
        ],
        _ => &[],
    }
}

pub(super) fn can_split(doc: &Document, source: ColorMode, depth: BitDepth) -> bool {
    let inks = source_inks(source);
    !inks.is_empty()
        && matches!(depth, BitDepth::Eight | BitDepth::Sixteen)
        && inks.len() + doc.channels.len() <= pictura_core::channels::MAX_CHANNELS as usize
        && doc.variables.protected_ids().is_empty()
}

pub(super) fn split(doc: &mut Document, options: ConversionOptions) -> Result<(), OpsError> {
    let source = doc.document_color_mode();
    let depth = doc.document_bit_depth();
    let (width, height) = (doc.width, doc.height);
    let pixels = width as usize * height as usize;
    if doc
        .channels
        .iter()
        .any(|channel| channel.data.len() != pixels)
    {
        return Err(OpsError::InvalidParams(
            "Multichannel extra plane length".into(),
        ));
    }

    let mut staged = doc.clone();
    let rgb = flattened_rgb(&mut staged)?;
    let inks = source_inks(source);
    let store = doc
        .source
        .planes()
        .as_ref()
        .filter(|store| store.width == width && store.height == height && store.depth == depth);
    let retained = store.and_then(|store| store.samples.slice(0..pixels * inks.len()));
    let profile = (source == ColorMode::Cmyk)
        .then(|| pictura_codec::document_cmyk_profile(doc))
        .flatten();
    let retained_components = retained.as_ref().map(Samples::narrow_to_u8);
    let separation = match (source, retained_components) {
        (ColorMode::Rgb, _) => rgb,
        (ColorMode::Grayscale, _) => rgb[..pixels].to_vec(),
        (ColorMode::Cmyk, Some(values))
            if profile
                .as_ref()
                .and_then(|profile| {
                    pictura_codec::cmyk_to_rgb_with_profile(&values, profile, options)
                })
                .unwrap_or_else(|| pictura_codec::cmyk_to_rgb(&values))
                == rgb =>
        {
            values
        }
        (ColorMode::Lab, Some(values)) if pictura_codec::lab_to_rgb(&values) == rgb => values,
        _ => planes::encode_source_mode(source, &rgb, profile.as_ref(), options).0,
    };

    let mut names: Vec<String> = inks.iter().map(|(name, _)| (*name).into()).collect();
    let existing_names = pictura_codec::alpha_names(doc);
    names.extend((0..doc.channels.len()).map(|i| {
        existing_names
            .get(i)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Alpha {}", i + 1))
    }));
    let mut display: Vec<_> = inks
        .iter()
        .map(|(_, [r, g, b])| ChannelDisplayInfo {
            color_space: 0,
            components: [*r, *g, *b, 0].map(|sample| u16::from(sample) * 257),
            opacity: 100,
            mode: SPOT_CHANNEL_MODE,
        })
        .collect();
    let mut existing_display = pictura_codec::display_info(doc);
    existing_display.resize(doc.channels.len(), pictura_codec::masked_display_info());
    display.extend(existing_display);

    let mut channels: Vec<_> = separation
        .chunks_exact(pixels)
        .map(|plane| Channel {
            id: 0,
            data: plane.to_vec().into(),
        })
        .collect();
    channels.append(&mut staged.channels);
    for (i, channel) in channels.iter_mut().enumerate() {
        channel.id = i as i16;
    }
    let merged_alpha = source == ColorMode::Rgb
        && doc.composite.signed_merged_alpha()
        && doc.composite.channels == 4;
    let native = if depth == BitDepth::Sixteen {
        let mut samples = Vec::with_capacity(channels.len() * pixels);
        for (i, channel) in channels.iter().enumerate() {
            let index = i + usize::from(merged_alpha && i >= inks.len());
            let old =
                store.and_then(|store| store.samples.slice(index * pixels..(index + 1) * pixels));
            append_native(&mut samples, &channel.data, old);
        }
        Some(SourcePlanes {
            depth,
            width,
            height,
            samples: Samples::U16(samples.into()),
        })
    } else {
        None
    };

    staged.layers = LayerStack::default();
    staged.mode = ColorMode::Rgb;
    staged.depth = BitDepth::Eight;
    staged.color_mode_data.clear();
    staged.source.set_multichannel_plates();
    staged.source.set_original_profile(None);
    pictura_codec::strip_document_icc(&mut staged);
    staged
        .source
        .set_native((depth == BitDepth::Sixteen).then_some(depth), native)
        .expect("matching plate sample depth");
    staged.channels = channels;
    pictura_codec::set_alpha_names(&mut staged, &names);
    pictura_codec::set_display_info(&mut staged, &display);
    let preview = pictura_codec::multichannel_preview(&staged);
    staged
        .composite
        .replace(preview)
        .expect("valid plate preview");
    staged.composite.set_signed_merged_alpha(false);
    staged.composite.set_merged_output(true);
    *doc = staged;
    Ok(())
}

fn flattened_rgb(doc: &mut Document) -> Result<Vec<u8>, OpsError> {
    let pixels = doc.width as usize * doc.height as usize;
    if !doc.layers.is_empty() {
        crate::flatten(doc).map_err(|error| OpsError::InvalidParams(format!("{error:?}")))?;
        let layer = &doc.layers[0];
        let mut rgb = vec![0; 3 * pixels];
        for (index, out) in rgb.chunks_exact_mut(pixels).enumerate() {
            layer.channels()[index].data.read_into(0, out);
        }
        return Ok(rgb);
    }
    let composite = &doc.composite;
    if !doc.has_stored_composite() || !matches!(composite.channels, 1 | 3 | 4) {
        return Err(OpsError::InvalidParams(
            "Multichannel needs a source image".into(),
        ));
    }
    let mut rgb = vec![0; 3 * pixels];
    for (index, out) in rgb.chunks_exact_mut(pixels).enumerate() {
        let plane = if composite.channels == 1 { 0 } else { index };
        composite.data.read_into(plane * pixels, out);
    }
    if composite.channels == 4 {
        let mut alpha = vec![0u8; pixels];
        composite.data.read_into(3 * pixels, &mut alpha);
        for out in rgb.chunks_exact_mut(pixels) {
            for (sample, &alpha) in out.iter_mut().zip(&alpha) {
                *sample = 255 - ((255 - u16::from(*sample)) * u16::from(alpha) / 255) as u8;
            }
        }
    }
    Ok(rgb)
}

fn append_native(samples: &mut Vec<u16>, current: &[u8], retained: Option<Samples>) {
    if let Some(Samples::U16(native)) = retained {
        if native
            .iter()
            .zip(current)
            .all(|(&value, &shown)| (value >> 8) as u8 == shown)
        {
            samples.extend(native.iter().copied());
            return;
        }
    }
    samples.extend(current.iter().map(|&sample| u16::from(sample) * 257));
}

pub(super) fn to_rgb(doc: &mut Document) -> Result<(), OpsError> {
    if !doc.has_stored_composite() || doc.composite.channels != 3 {
        return Err(OpsError::InvalidParams(
            "Multichannel has no RGB preview".into(),
        ));
    }
    let depth = doc.document_bit_depth();
    let pixels = doc.width as usize * doc.height as usize;
    let native = (depth == BitDepth::Sixteen).then(|| {
        let mut samples = Vec::with_capacity((3 + doc.channels.len()) * pixels);
        append_native(&mut samples, &doc.composite.data, None);
        for (i, channel) in doc.channels.iter().enumerate() {
            let retained = doc
                .source
                .planes()
                .as_ref()
                .and_then(|store| store.samples.slice(i * pixels..(i + 1) * pixels));
            append_native(&mut samples, &channel.data, retained);
        }
        SourcePlanes {
            depth,
            width: doc.width,
            height: doc.height,
            samples: Samples::U16(samples.into()),
        }
    });
    doc.source
        .set_encoding(None, None)
        .expect("RGB source encoding");
    doc.source
        .set_native((depth == BitDepth::Sixteen).then_some(depth), native)
        .expect("matching RGB sample depth");
    planes::background_from_composite(doc);
    Ok(())
}
