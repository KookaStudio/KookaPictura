use super::*;

/// Fallback for an Indexed document's palette; unreachable because `palette_from` errors on a malformed Indexed palette before conversion.
const EMPTY_PALETTE: [u8; 768] = [0; 768];

/// Normalize a non-RGB document to the engine's working mode. Grayscale and RGB
/// pass through untouched; Bitmap/Indexed/CMYK/Lab become RGB, record the source
/// mode, and convert every color plane (composite and every layer, including a
/// grouped layer's descendants).
pub(super) fn normalize(
    mut doc: Document,
    header_mode: ColorMode,
    depth: u16,
    palette: Option<&[u8; 768]>,
    options: &OpenOptions,
    bitmap_composite_decoded: bool,
) -> Document {
    if matches!(header_mode, ColorMode::Grayscale | ColorMode::Rgb) {
        return doc;
    }
    if doc.source.multichannel_plates() {
        crate::multichannel::complete_plate_display(&mut doc);
        doc.composite
            .replace(crate::multichannel_preview(&doc))
            .expect("valid plate preview");
        doc.mode = ColorMode::Rgb;
        return doc;
    }
    if header_mode == ColorMode::Lab {
        crate::color_mode::retain_lab_layer_planes(&mut doc.layers, depth);
    }
    if header_mode == ColorMode::Cmyk {
        crate::color_mode::retain_cmyk_layer_planes(&mut doc.layers, depth);
    }
    if header_mode == ColorMode::Indexed {
        crate::color_mode::retain_indexed_layer_planes(&mut doc.layers, depth);
    }
    let palette = palette.unwrap_or(&EMPTY_PALETTE);

    let embedded_cmyk = if header_mode == ColorMode::Cmyk {
        crate::icc::cmyk_profile_from_resources(&doc)
    } else {
        None
    };
    let working_cmyk = options
        .working_cmyk
        .as_deref()
        .and_then(|bytes| Profile::from_icc(bytes).ok());
    let decode_profile = if options.policy == Policy::Off {
        None
    } else {
        embedded_cmyk.as_ref().or(working_cmyk.as_ref())
    };

    let indexed_clear = if header_mode == ColorMode::Indexed {
        crate::indexed_metadata(&doc).1
    } else {
        None
    };
    if !bitmap_composite_decoded {
        let buffer = doc.composite.buffer();
        let converted = if header_mode == ColorMode::Indexed {
            let plane = buffer.width as usize * buffer.height as usize;
            let indices = &buffer.data[..plane];
            let mut converted = crate::indexed_working_buffer(
                buffer.width, buffer.height, indices, palette, indexed_clear,
            );
            if buffer.channels > 1 {
                let alpha = &buffer.data[plane..2 * plane];
                let mut data = converted.data[..3 * plane].to_vec();
                data.extend(alpha.iter().zip(indices).map(|(&a, &i)| {
                    if Some(i) == indexed_clear { 0 } else { a }
                }));
                converted.channels = 4;
                converted.data = data.into();
            }
            converted
        } else {
            convert_pixels(buffer.clone(), header_mode, depth, Some(palette), decode_profile, options.conversion)
        };
        doc.composite.replace(converted).expect("valid composite");
    }
    for layer in &mut doc.layers {
        crate::color_mode::convert_layer_color_channels(
            layer,
            header_mode,
            palette,
            decode_profile,
            options.conversion,
        );
    }
    if let Some(clear) = indexed_clear {
        crate::indexed_metadata::restore_layer_transparency(&mut doc.layers, clear);
    }
    doc.mode = ColorMode::Rgb;
    doc.depth = BitDepth::Eight;
    doc.source
        .set_encoding(
            Some(header_mode),
            (header_mode == ColorMode::Indexed).then_some(*palette),
        )
        .expect("validated source encoding");
    if header_mode == ColorMode::Indexed {
        doc.color_mode_data.clear();
    }
    if header_mode == ColorMode::Cmyk {
        doc.source.set_working_profile(None);
        match options.policy {
            Policy::Off => {
                crate::icc::set_icc_resource(&mut doc, None);
                doc.source.set_original_profile(None);
            }
            Policy::Preserve => {
                doc.source.set_original_profile(None);
                // An untagged CMYK file is treated as the working CMYK; attach
                // the working profile so a save re-encodes consistently. An
                // embedded profile is kept byte-for-byte.
                if embedded_cmyk.is_none() {
                    if let Some(working) = working_cmyk.as_ref() {
                        let icc = working.to_icc();
                        crate::icc::set_icc_resource(&mut doc, Some(&icc));
                    }
                }
            }
            Policy::Convert => {
                let source_bytes = crate::icc::icc_resource_bytes(&doc);
                if let (Some(src), Some(dst)) = (embedded_cmyk.as_ref(), working_cmyk.as_ref()) {
                    crate::color_mode::convert_retained_cmyk(
                        &mut doc,
                        src,
                        dst,
                        options.conversion,
                    );
                    let icc = dst.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else if let Some(dst) = working_cmyk.as_ref() {
                    let icc = dst.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else if let Some(src) = embedded_cmyk.as_ref() {
                    let icc = src.to_icc();
                    crate::icc::set_icc_resource(&mut doc, Some(&icc));
                } else {
                    crate::icc::set_icc_resource(&mut doc, None);
                }
                doc.source
                    .set_original_profile(source_bytes.map(Into::into));
            }
        }
    }
    doc
}

/// Convert a composite's color planes to planar RGB.
pub(super) fn convert_pixels(
    buf: PixelBuffer,
    mode: ColorMode,
    depth: u16,
    palette: Option<&[u8; 768]>,
    cmyk_profile: Option<&Profile>,
    options: ConversionOptions,
) -> PixelBuffer {
    use crate::color_mode::*;
    let data = match mode {
        ColorMode::Bitmap if depth == 1 => {
            bitmap_rows_to_rgb(&buf.data, buf.width as usize, buf.height as usize)
        }
        ColorMode::Bitmap => gray_to_rgb(&buf.data),
        ColorMode::Indexed => indexed_to_rgb(&buf.data, palette.unwrap_or(&EMPTY_PALETTE)),
        ColorMode::Cmyk => cmyk_profile
            .and_then(|profile| cmyk_to_rgb_with_profile(&buf.data, profile, options))
            .unwrap_or_else(|| cmyk_to_rgb(&buf.data)),
        ColorMode::Lab => lab_to_rgb(&buf.data),
        ColorMode::Duotone => gray_to_rgb(&buf.data),
        ColorMode::Multichannel if buf.channels == 1 => gray_to_rgb(&buf.data),
        ColorMode::Multichannel => cmy_to_rgb(&buf.data),
        ColorMode::Grayscale | ColorMode::Rgb => buf.data.to_vec(),
    };
    PixelBuffer {
        width: buf.width,
        height: buf.height,
        channels: 3,
        data: data.into(),
    }
}
