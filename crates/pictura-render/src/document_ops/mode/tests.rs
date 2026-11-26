use super::*;
use pictura_codec::{
    read_psd, web_safe_palette, write_psd, ColorReduction, Dither, PaletteOptions,
};
use pictura_core::{Channel, PixelBuffer, Samples};

/// A one-layer RGB document whose layer and (RGBA) composite hold `pixels`,
/// as the app keeps it.
fn rgb_doc(width: u32, height: u32, pixels: &[[u8; 3]]) -> Document {
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    Document::from_rgba("Layer 1", width, height, &rgba)
}

fn ramp(width: u32, height: u32) -> Document {
    let pixels: Vec<[u8; 3]> = (0..width * height)
        .map(|i| {
            let v = (i * 37 % 256) as u8;
            [v, v.wrapping_mul(3), 255 - v]
        })
        .collect();
    rgb_doc(width, height, &pixels)
}

fn roundtrip(doc: &Document) -> Document {
    read_psd(&write_psd(&save_view(doc)).expect("writes")).expect("reads")
}

fn layer_plane(doc: &Document, id: i16) -> Vec<u8> {
    doc.layers[0]
        .channels()
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.to_vec())
        .expect("channel")
}

#[test]
fn layer_only_native_depth_converts_with_absent_composite() {
    let mut doc = rgb_doc(1, 1, &[[40, 80, 120]]);
    let rect = doc.layers[0].rect;
    doc.layers[0].source_channels = Some(pictura_core::SourceChannels::new(
        BitDepth::Sixteen,
        rect,
        vec![
            (0, Samples::U16(vec![10281].into())),
            (1, Samples::U16(vec![20561].into())),
            (2, Samples::U16(vec![30841].into())),
        ],
    ));
    let child = doc.layers[0].clone();
    doc.layers = pictura_core::LayerStack::from_layers(vec![pictura_core::Layer::new(
        pictura_core::LayerContent::Group(pictura_core::LayerStack::from_layers(vec![child])),
    )]);
    doc.source
        .set_native(Some(BitDepth::Sixteen), None)
        .unwrap();
    doc.composite.detach_plane();
    convert_bit_depth(&mut doc, BitDepth::ThirtyTwo).unwrap();
    assert_eq!(doc.source.depth(), Some(BitDepth::ThirtyTwo));
    assert_eq!(
        doc.source.planes().as_ref().unwrap().depth,
        BitDepth::ThirtyTwo
    );
    let Samples::F32(values) = &doc.layers[0].children()[0]
        .source_channels
        .as_ref()
        .unwrap()
        .planes[0]
        .1
    else {
        panic!("expected float samples")
    };
    assert!((values[0] - 10281.0 / 65535.0).abs() < 1e-6);
}

fn indexed_options(reduction: ColorReduction, dither: Dither) -> PaletteOptions {
    PaletteOptions {
        reduction,
        colors: 256,
        dither,
        amount: 75,
        transparency: false,
        mapped_transparency: false,
        matte: [255, 255, 255],
    }
}

#[test]
fn grayscale_is_rounded_rec601_luma_on_layers_and_composite() {
    let mut doc = rgb_doc(2, 1, &[[255, 0, 0], [10, 200, 30]]);
    convert_mode(&mut doc, ColorMode::Grayscale).expect("converts");
    assert_eq!(doc.mode, ColorMode::Grayscale);
    assert_eq!(document_color_mode(&doc), ColorMode::Grayscale);
    // 0.299 * 255 = 76.2; 0.299*10 + 0.587*200 + 0.114*30 = 123.81.
    assert_eq!(layer_plane(&doc, 0), vec![76, 124]);
    assert!(doc.layers[0]
        .channels()
        .iter()
        .all(|c| c.id != 1 && c.id != 2));
    assert_eq!(doc.composite.channels, 1);
    assert_eq!(doc.composite.data.to_vec(), vec![76, 124]);
    assert_eq!(roundtrip(&doc).mode, ColorMode::Grayscale);
}

#[test]
fn rgb_from_grayscale_replicates_the_gray_plane() {
    let mut doc = rgb_doc(2, 1, &[[90, 90, 90], [10, 10, 10]]);
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    convert_mode(&mut doc, ColorMode::Rgb).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    for id in 0..3 {
        assert_eq!(layer_plane(&doc, id), vec![90, 10]);
    }
    assert_eq!(doc.composite.channels, 3);
}

#[test]
fn gcr_cmyk_generates_black_and_round_trips_within_one_level() {
    let rgb = [0u8, 255, 128, 0, 0, 64, 0, 0, 200];
    let cmyk = planes::rgb_to_cmyk_gcr(&rgb);
    // Pure black is no CMY ink and full K (stored 0); pure red is full M + Y
    // ink and no K.
    assert_eq!([cmyk[0], cmyk[3], cmyk[6], cmyk[9]], [255, 255, 255, 0]);
    assert_eq!([cmyk[1], cmyk[4], cmyk[7], cmyk[10]], [255, 0, 0, 255]);
    let back = pictura_codec::cmyk_to_rgb(&cmyk);
    for (a, b) in rgb.iter().zip(&back) {
        assert!(a.abs_diff(*b) <= 1, "{rgb:?} -> {back:?}");
    }
}

#[test]
fn cmyk_conversion_saves_as_cmyk_and_shows_what_it_saves() {
    let mut doc = ramp(8, 4);
    convert_mode(&mut doc, ColorMode::Cmyk).expect("converts");
    assert_eq!(document_color_mode(&doc), ColorMode::Cmyk);
    let read = roundtrip(&doc);
    assert_eq!(read.source.mode(), Some(ColorMode::Cmyk));
    assert_eq!(layer_plane(&read, 0), layer_plane(&doc, 0));
    assert_eq!(layer_plane(&read, 2), layer_plane(&doc, 2));
    // The retained store is what was written, byte for byte.
    assert_eq!(read.source.planes(), doc.source.planes());
}

#[test]
fn lab_conversion_saves_as_lab_and_reads_back_the_shown_rgb() {
    let mut doc = ramp(8, 4);
    convert_mode(&mut doc, ColorMode::Lab).expect("converts");
    let read = roundtrip(&doc);
    assert_eq!(read.source.mode(), Some(ColorMode::Lab));
    assert_eq!(layer_plane(&read, 1), layer_plane(&doc, 1));
    // Converting again from Lab to RGB drops the Lab store: a plain RGB save.
    convert_mode(&mut doc, ColorMode::Rgb).unwrap();
    assert_eq!(doc.source.mode(), None);
    assert_eq!(roundtrip(&doc).source.mode(), None);
}

#[test]
fn indexed_web_palette_contains_only_web_colors_and_saves_indexed() {
    let mut doc = ramp(16, 8);
    convert_to_indexed(
        &mut doc,
        indexed_options(ColorReduction::Restrictive, Dither::Diffusion),
    )
    .expect("converts");
    let web = web_safe_palette();
    let (r, g, b) = (
        layer_plane(&doc, 0),
        layer_plane(&doc, 1),
        layer_plane(&doc, 2),
    );
    for i in 0..r.len() {
        assert!(
            web.contains(&[r[i], g[i], b[i]]),
            "pixel {i} off the web palette"
        );
    }
    let read = roundtrip(&doc);
    assert_eq!(read.source.mode(), Some(ColorMode::Indexed));
    assert_eq!(read.source.palette(), doc.source.palette());
    assert_eq!(layer_plane(&read, 0), r);
}

#[test]
fn indexed_flattens_a_layered_document_and_discards_hidden_layers() {
    let mut doc = ramp(4, 4);
    let mut hidden = doc.layers[0].clone();
    hidden.name = "hidden".into();
    hidden.visible = false;
    let _ = doc.layers.push(hidden);
    {
        let _ = doc
            .layers
            .push(pictura_codec::layer_from_parts(pictura_core::LayerParts {
                name: "group".into(),
                is_group: true,
                ..Default::default()
            }));
    };
    convert_to_indexed(
        &mut doc,
        indexed_options(ColorReduction::Adaptive, Dither::None),
    )
    .unwrap();
    assert_eq!(doc.layers.len(), 1);
    assert!(doc.layers[0].background);
}

#[test]
fn exact_palette_is_offered_only_up_to_256_colors() {
    let few = rgb_doc(2, 2, &[[1, 2, 3], [1, 2, 3], [9, 9, 9], [0, 0, 0]]);
    assert!(indexed_exact_available(&few));
    let pixels: Vec<[u8; 3]> = (0..300u32).map(|i| [i as u8, (i >> 8) as u8, 0]).collect();
    assert!(!indexed_exact_available(&rgb_doc(300, 1, &pixels)));
}

#[test]
fn bitmap_threshold_is_exact_at_mid_gray_and_saves_as_bitmap() {
    let mut doc = rgb_doc(
        4,
        1,
        &[[0, 0, 0], [128, 128, 128], [129, 129, 129], [255, 255, 255]],
    );
    assert!(
        !can_convert_mode(&doc, ColorMode::Bitmap),
        "Bitmap needs Grayscale"
    );
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    convert_to_bitmap(&mut doc, BitmapMethod::Threshold).expect("converts");
    assert_eq!(document_color_mode(&doc), ColorMode::Bitmap);
    assert_eq!(document_bit_depth(&doc), BitDepth::One);
    assert!(doc.layers.is_empty());
    assert_eq!(&doc.composite.data[..4], &[0, 0, 255, 255]);
    let read = roundtrip(&doc);
    assert_eq!(read.source.mode(), Some(ColorMode::Bitmap));
    assert_eq!(read.composite.data, doc.composite.data);
}

#[test]
fn bitmap_conversion_clears_display_info() {
    let mut doc = rgb_doc(2, 1, &[[0, 0, 0], [255, 255, 255]]);
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    doc.channels.push(pictura_core::Channel {
        id: 0,
        data: vec![0; 2].into(),
    });
    pictura_codec::set_display_info(
        &mut doc,
        &[pictura_codec::ChannelDisplayInfo {
            mode: pictura_codec::SELECTED_AREAS_MODE,
            ..Default::default()
        }],
    );
    convert_to_bitmap(&mut doc, BitmapMethod::Threshold).expect("converts");
    assert!(pictura_codec::display_info(&doc).is_empty());
    doc.channels.push(pictura_core::Channel {
        id: 0,
        data: vec![0; 2].into(),
    });
    assert!(!pictura_codec::channel_selected_areas(&doc, 0));
}

#[test]
fn bitmap_dithers_keep_the_mean_tone() {
    let gray = vec![64u8; 64 * 64];
    for method in [BitmapMethod::Pattern, BitmapMethod::Diffusion] {
        let whites = flat::bitmap_whites(&gray, 64, method, 72.0);
        let share = whites.iter().filter(|&&w| w).count() as f64 / whites.len() as f64;
        assert!(
            (share - 0.25).abs() < 0.03,
            "{method:?} white share {share}"
        );
    }
}

#[test]
fn grayscale_from_bitmap_rebuilds_a_background_layer() {
    let mut doc = rgb_doc(2, 1, &[[0, 0, 0], [255, 255, 255]]);
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    convert_to_bitmap(&mut doc, BitmapMethod::Threshold).unwrap();
    assert!(
        !can_convert_mode(&doc, ColorMode::Rgb),
        "Bitmap leaves through Grayscale"
    );
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    assert_eq!(doc.source.mode(), None);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(layer_plane(&doc, 0), vec![0, 255]);
}

#[test]
fn eight_to_sixteen_widens_and_saves_sixteen_bit() {
    let mut doc = ramp(4, 4);
    convert_bit_depth(&mut doc, BitDepth::Sixteen).expect("converts");
    assert_eq!(document_bit_depth(&doc), BitDepth::Sixteen);
    let read = roundtrip(&doc);
    assert_eq!(read.source.depth(), Some(BitDepth::Sixteen));
    let Some(Samples::U16(v)) = read.source.planes().as_ref().map(|s| &s.samples) else {
        panic!("16-bit store");
    };
    assert_eq!(v[1], doc.composite.data[1] as u16 * 257);
}

#[test]
fn sixteen_to_thirty_two_and_back_down_to_eight() {
    let mut doc = ramp(4, 4);
    let before = layer_plane(&doc, 0);
    convert_bit_depth(&mut doc, BitDepth::Sixteen).unwrap();
    convert_bit_depth(&mut doc, BitDepth::ThirtyTwo).unwrap();
    assert_eq!(document_bit_depth(&doc), BitDepth::ThirtyTwo);
    assert_eq!(roundtrip(&doc).source.depth(), Some(BitDepth::ThirtyTwo));
    assert!(
        convert_bit_depth(&mut doc, BitDepth::Eight).is_err(),
        "32-bit needs HDR toning"
    );
    assert!(
        !can_convert_mode(&doc, ColorMode::Cmyk),
        "32-bit CMYK does not exist"
    );

    let mut doc = ramp(4, 4);
    convert_bit_depth(&mut doc, BitDepth::Sixteen).unwrap();
    convert_bit_depth(&mut doc, BitDepth::Eight).unwrap();
    assert_eq!(document_bit_depth(&doc), BitDepth::Eight);
    assert!(doc.source.planes().is_none());
    assert_eq!(layer_plane(&doc, 0), before);
}

#[test]
fn sixteen_bit_grayscale_is_computed_at_native_depth() {
    let mut doc = rgb_doc(1, 1, &[[255, 0, 0]]);
    convert_bit_depth(&mut doc, BitDepth::Sixteen).unwrap();
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    assert_eq!(document_bit_depth(&doc), BitDepth::Sixteen);
    let store = doc.layers[0].source_channels.as_ref().expect("store");
    assert_eq!(store.planes.len(), 1);
    // 0.299 * 65535 = 19594.965, not the 8-bit 76 * 257 = 19532.
    assert_eq!(store.planes[0].1, Samples::U16(vec![19595].into()));
}

#[test]
fn sixteen_bit_cmyk_saves_cmyk_at_sixteen_bits() {
    let mut doc = ramp(4, 4);
    convert_mode(&mut doc, ColorMode::Cmyk).unwrap();
    convert_bit_depth(&mut doc, BitDepth::Sixteen).unwrap();
    let read = roundtrip(&doc);
    assert_eq!(read.source.mode(), Some(ColorMode::Cmyk));
    assert_eq!(read.source.depth(), Some(BitDepth::Sixteen));
    assert!(
        !can_convert_mode(&doc, ColorMode::Indexed),
        "Indexed needs 8-bit"
    );
    convert_bit_depth(&mut doc, BitDepth::Eight).unwrap();
    let read = roundtrip(&doc);
    assert_eq!(
        (read.source.mode(), read.source.depth()),
        (Some(ColorMode::Cmyk), None)
    );
}

#[test]
fn the_matrix_dims_unsupported_targets_and_refusals_do_not_mutate() {
    let mut doc = ramp(2, 2);
    assert!(!can_convert_mode(&doc, ColorMode::Rgb), "current mode");
    assert!(!can_convert_mode(&doc, ColorMode::Duotone));
    assert!(can_convert_mode(&doc, ColorMode::Multichannel));
    assert!(!can_convert_mode(&doc, ColorMode::Bitmap));
    assert!(can_convert_mode(&doc, ColorMode::Indexed));
    assert!(!can_convert_depth(&doc, BitDepth::Eight), "current depth");
    let before = doc.clone();
    assert!(convert_mode(&mut doc, ColorMode::Bitmap).is_err());
    assert!(convert_to_bitmap(&mut doc, BitmapMethod::Threshold).is_err());
    assert_eq!(doc, before);
    convert_to_indexed(
        &mut doc,
        indexed_options(ColorReduction::Adaptive, Dither::None),
    )
    .unwrap();
    assert!(
        !can_convert_depth(&doc, BitDepth::Sixteen),
        "Indexed is 8-bit only"
    );
    assert!(can_convert_mode(&doc, ColorMode::Rgb));
}

#[test]
fn save_view_trims_the_display_alpha_for_a_source_mode_or_extra_channels() {
    let doc = ramp(2, 2);
    assert!(matches!(save_view(&doc), Cow::Borrowed(_)));
    let mut cmyk = doc.clone();
    convert_mode(&mut cmyk, ColorMode::Cmyk).unwrap();
    assert_eq!(cmyk.composite.channels, 4);
    assert_eq!(save_view(&cmyk).composite.channels, 3);
    let mut extra = doc.clone();
    extra.channels.push(Channel {
        id: 0,
        data: vec![0u8; 4].into(),
    });
    assert_eq!(save_view(&extra).composite.channels, 3);
    let rgb = PixelBuffer {
        width: 2,
        height: 2,
        channels: 3,
        data: vec![0u8; 3 * 4].into(),
    };
    let mut without_alpha = extra.clone();
    assert!(without_alpha
        .composite
        .replace(PixelBuffer {
            channels: 4,
            ..rgb.clone()
        })
        .is_err());
    assert_eq!(without_alpha, extra);
    without_alpha
        .composite
        .replace(rgb)
        .expect("valid RGB composite without alpha");
    assert!(matches!(save_view(&without_alpha), Cow::Borrowed(_)));
}

#[test]
fn an_extra_channel_saves_without_a_duplicate_alpha_plane() {
    let mut doc = ramp(2, 2);
    doc.composite
        .replace(PixelBuffer::new(0, 0, 4))
        .expect("valid composite");
    let alpha = [255u8, 0, 128, 200];
    doc.channels = vec![Channel {
        id: 0,
        data: alpha.to_vec().into(),
    }];
    let view = save_view(&doc);
    assert_eq!(view.composite.channels, 3);
    let plane = 4;
    let mut expected = vec![0u8; 3 * plane];
    view.composite.data.read_into(0, &mut expected);
    let bytes = write_psd(view.as_ref()).expect("writes");
    assert_eq!(
        u16::from_be_bytes(bytes[12..14].try_into().unwrap()),
        4,
        "three color channels plus the extra"
    );
    let back = read_psd(&bytes).expect("reads");
    assert_eq!(back.composite.channels, 3);
    assert_eq!(back.composite.data.to_vec(), expected);
    assert_eq!(back.channels.len(), 1);
    assert_eq!(back.channels[0].data.to_vec(), alpha);
}

/// The first three planes of `buffer`, read without materializing it.
fn rgb_planes(buffer: &PixelBuffer) -> Vec<u8> {
    let n = buffer.width as usize * buffer.height as usize;
    let mut rgb = vec![0u8; 3 * n];
    buffer.data.read_into(0, &mut rgb);
    rgb
}

// [sps_merged_tiled]
#[test]
fn save_view_keeps_the_merged_image_tiled() {
    let (w, h) = (301u32, 530u32);
    let extra = || Channel {
        id: 0,
        data: vec![7u8; (w * h) as usize].into(),
    };
    let mut layered = layered_without_composite(w, h);
    layered.channels.push(extra());
    let full = crate::composite_tiled(&layered);
    let view = pictura_core::assert_no_materialization(|| save_view(&layered));
    assert_eq!(view.composite.channels, 3);
    assert!(view.composite.data.is_tiled());
    assert_eq!(rgb_planes(&view.composite), rgb_planes(&full));

    let mut stored = ramp(w, h);
    stored.channels.push(extra());
    assert_eq!(stored.composite.channels, 4);
    let view = pictura_core::assert_no_materialization(|| save_view(&stored));
    assert_eq!(view.composite.channels, 3);
    assert!(view.composite.data.is_tiled());
    assert_eq!(rgb_planes(&view.composite), rgb_planes(&stored.composite));
}

#[test]
fn an_rgba_document_without_extra_channels_keeps_its_alpha_plane() {
    let rgba = [10u8, 20, 30, 255, 40, 50, 60, 0];
    let doc = Document::from_rgba("Layer 1", 2, 1, &rgba);
    let bytes = write_psd(&save_view(&doc)).expect("writes");
    assert_eq!(
        u16::from_be_bytes(bytes[12..14].try_into().unwrap()),
        4,
        "an RGBA composite without extras keeps four channels"
    );
    let back = read_psd(&bytes).expect("reads");
    assert_eq!(back.composite.channels, 3);
    assert_eq!(back.channels.len(), 1);
    assert_eq!(back.channels[0].data.to_vec(), vec![255, 0]);
}

#[test]
fn a_depth16_extra_channel_saves_without_a_duplicate_alpha_plane() {
    let mut doc = ramp(2, 2);
    doc.source
        .set_native(Some(BitDepth::Sixteen), doc.source.planes().clone())
        .expect("valid source depth");
    let plane = 4;
    let mut samples = Vec::with_capacity(4 * plane);
    for c in 0..3u16 {
        for i in 0..plane {
            samples.push((c * 40 + i as u16) * 257);
        }
    }
    let extra = [7u8, 8, 9, 10];
    samples.extend(extra.iter().map(|v| u16::from(*v) * 257));
    doc.source
        .replace_planes(Some(pictura_core::SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 2,
            height: 2,
            samples: Samples::U16(samples.into()),
        }))
        .expect("valid source planes");
    doc.channels = vec![Channel {
        id: 0,
        data: extra.to_vec().into(),
    }];
    let view = save_view(&doc);
    let mut expected = vec![0u8; 3 * plane];
    view.composite.data.read_into(0, &mut expected);
    let bytes = write_psd(view.as_ref()).expect("writes");
    assert_eq!(
        u16::from_be_bytes(bytes[12..14].try_into().unwrap()),
        4,
        "three color channels plus the extra"
    );
    assert_eq!(
        u16::from_be_bytes(bytes[22..24].try_into().unwrap()),
        16,
        "the retained depth is written"
    );
    let back = read_psd(&bytes).expect("reads");
    assert_eq!(back.source.depth(), Some(BitDepth::Sixteen));
    assert_eq!(back.composite.data.to_vec(), expected);
    assert_eq!(back.channels.len(), 1);
    assert_eq!(back.channels[0].data.to_vec(), extra);
}

#[test]
fn a_depth32_extra_channel_saves_without_a_duplicate_alpha_plane() {
    let mut doc = ramp(2, 2);
    doc.source
        .set_native(Some(BitDepth::ThirtyTwo), doc.source.planes().clone())
        .expect("valid source depth");
    let plane = 4;
    let color = [3u8, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36];
    let extra = [7u8, 8, 9, 10];
    let mut bytes = color.to_vec();
    bytes.extend_from_slice(&extra);
    doc.source
        .replace_planes(Some(pictura_core::SourcePlanes {
            depth: BitDepth::ThirtyTwo,
            width: 2,
            height: 2,
            samples: Samples::widen_from_u8(&bytes, BitDepth::ThirtyTwo),
        }))
        .expect("valid source planes");
    doc.channels = vec![Channel {
        id: 0,
        data: extra.to_vec().into(),
    }];
    let view = save_view(&doc);
    let mut expected = vec![0u8; 3 * plane];
    view.composite.data.read_into(0, &mut expected);
    let bytes = write_psd(view.as_ref()).expect("writes");
    assert_eq!(
        u16::from_be_bytes(bytes[12..14].try_into().unwrap()),
        4,
        "three color channels plus the extra"
    );
    assert_eq!(
        u16::from_be_bytes(bytes[22..24].try_into().unwrap()),
        32,
        "the retained depth is written"
    );
    let back = read_psd(&bytes).expect("reads");
    assert_eq!(back.source.depth(), Some(BitDepth::ThirtyTwo));
    assert_eq!(back.composite.data.to_vec(), expected);
    assert_eq!(back.channels.len(), 1);
    assert_eq!(back.channels[0].data.to_vec(), extra);
}

#[test]
fn mode_profile_uses_working_cmyk() {
    let mut doc = ramp(2, 1);
    let source = doc.composite.data[..6].to_vec();
    let profile = Profile::coated_cmyk();
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    convert_mode_with_profiles(&mut doc, ColorMode::Cmyk, Some(&profile), options)
        .expect("converts");
    let expected =
        pictura_codec::rgb_to_cmyk_with_profile(&source, &profile, options).expect("lcms2 encode");
    let store = doc.source.planes().as_ref().expect("store");
    let Samples::U8(values) = &store.samples else {
        panic!("8-bit store");
    };
    assert_eq!(values.as_slice(), expected.as_slice());
    let icc = pictura_codec::decode_image_resources(&doc)
        .into_iter()
        .find(|r| r.id == pictura_codec::ICC_PROFILE)
        .map(|r| r.data)
        .expect("1039");
    assert_eq!(icc, profile.to_icc());
}

#[test]
fn mode_profile_falls_back() {
    let mut doc = ramp(2, 1);
    let source = doc.composite.data[..6].to_vec();
    convert_mode(&mut doc, ColorMode::Cmyk).expect("converts");
    let expected = planes::rgb_to_cmyk_gcr(&source);
    let store = doc.source.planes().as_ref().expect("store");
    let Samples::U8(values) = &store.samples else {
        panic!("8-bit store");
    };
    assert_eq!(values.as_slice(), expected.as_slice());
    assert!(pictura_codec::decode_image_resources(&doc)
        .iter()
        .all(|r| r.id != pictura_codec::ICC_PROFILE));
}

#[test]
fn mode_profile_round_trip() {
    let mut doc = ramp(2, 1);
    let profile = Profile::coated_cmyk();
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    convert_mode_with_profiles(&mut doc, ColorMode::Cmyk, Some(&profile), options)
        .expect("converts");
    let shown = doc.composite.data[..6].to_vec();
    let written = write_psd(&save_view(&doc)).expect("writes");
    let read = pictura_codec::read_psd_with_options(
        &written,
        &pictura_codec::OpenOptions {
            policy: pictura_codec::Policy::Preserve,
            working_cmyk: Some(profile.to_icc()),
            conversion: options,
            skip_layered_merged: false,
            source: None,
        },
    )
    .expect("reads");
    assert_eq!(read.source.mode(), Some(ColorMode::Cmyk));
    assert_eq!(read.source.planes(), doc.source.planes());
    for (a, b) in shown.iter().zip(&read.composite.data[..6]) {
        assert!(
            a.abs_diff(*b) <= 8,
            "{shown:?} -> {:?}",
            &read.composite.data[..6]
        );
    }
}

/// A two-layer RGB document that keeps no stored composite (#1216), tall
/// enough for several streamed bands, with a half-opaque top layer.
fn layered_without_composite(width: u32, height: u32) -> Document {
    let mut doc = ramp(width, height);
    let mut top = doc.layers[0].clone();
    top.name = "Layer 2".into();
    top.opacity = 128;
    for channel in top.channels_mut() {
        let flipped: Vec<u8> = channel.data.to_vec().iter().rev().copied().collect();
        channel.data = flipped.into();
    }
    doc.layers.push(top).expect("a second layer");
    doc.composite
        .replace(PixelBuffer::new(0, 0, 4))
        .expect("valid composite");
    doc
}

/// The fields a conversion writes, as plain values: mode, composite channels
/// and samples, the source store's depth and bytes, and every layer plane.
type ConvertedState = (
    ColorMode,
    u8,
    Vec<u8>,
    Option<(BitDepth, Vec<u8>)>,
    Vec<Vec<u8>>,
);

fn converted_state(doc: &Document) -> ConvertedState {
    let store = doc.source.planes().as_ref().map(|store| {
        let bytes = match &store.samples {
            Samples::U8(v) => v.to_vec(),
            Samples::U16(v) => v.iter().flat_map(|s| s.to_le_bytes()).collect(),
            Samples::F32(v) => v.iter().flat_map(|s| s.to_le_bytes()).collect(),
        };
        (store.depth, bytes)
    });
    let layers = doc
        .layers
        .iter()
        .flat_map(|layer| layer.channels().iter().map(|c| c.data.to_vec()))
        .collect();
    (
        doc.mode,
        doc.composite.channels,
        doc.composite.data.to_vec(),
        store,
        layers,
    )
}

/// Grayscale, CMYK (GCR and a dithered profile), Lab and 16-bit conversions
/// that stream their RGB source equal the ones run from a stored composite
/// (#1274).
#[test]
fn streamed_conversions_match_the_whole_path() {
    let profile = Profile::coated_cmyk();
    let dithered = ConversionOptions::default();
    assert!(dithered.dither);
    type Convert = Box<dyn Fn(&mut Document)>;
    let conversions: Vec<(&str, Convert)> = vec![
        (
            "gray",
            Box::new(|d| convert_mode(d, ColorMode::Grayscale).unwrap()),
        ),
        (
            "cmyk",
            Box::new(|d| convert_mode(d, ColorMode::Cmyk).unwrap()),
        ),
        (
            "cmyk profile",
            Box::new(move |d| {
                convert_mode_with_profiles(d, ColorMode::Cmyk, Some(&profile), dithered).unwrap()
            }),
        ),
        (
            "lab",
            Box::new(|d| convert_mode(d, ColorMode::Lab).unwrap()),
        ),
        (
            "16",
            Box::new(|d| convert_bit_depth(d, BitDepth::Sixteen).unwrap()),
        ),
    ];
    for (name, convert) in &conversions {
        let mut streamed = layered_without_composite(301, 530);
        assert!(stream::streams_source(&streamed));
        let mut whole = streamed.clone();
        whole
            .composite
            .replace(crate::composite_tiled(&whole))
            .expect("valid composite");
        assert!(!stream::streams_source(&whole));
        convert(&mut streamed);
        convert(&mut whole);
        assert!(
            converted_state(&streamed) == converted_state(&whole),
            "{name}"
        );
    }
}

#[test]
fn multichannel_flattens_visible_rgb_and_preserves_extra() {
    let mut doc = rgb_doc(1, 1, &[[33, 112, 203]]);
    let mut hidden = rgb_doc(1, 1, &[[255, 5, 10]]).layers[0].clone();
    hidden.visible = false;
    doc.layers = pictura_core::LayerStack::from_layers(vec![doc.layers[0].clone(), hidden]);
    doc.channels.push(Channel {
        id: 0,
        data: vec![77].into(),
    });
    pictura_codec::set_alpha_names(&mut doc, &["Saved outline".into()]);

    convert_mode(&mut doc, ColorMode::Multichannel).unwrap();

    assert!(doc.layers.is_empty());
    assert!(doc.source.multichannel_plates());
    assert_eq!(
        pictura_codec::alpha_names(&doc),
        ["Cyan", "Magenta", "Yellow", "Saved outline"]
    );
    assert_eq!(pictura_codec::spot_channel_indices(&doc), [0, 1, 2]);
    assert_eq!(
        doc.channels.iter().map(|c| c.data[0]).collect::<Vec<_>>(),
        [33, 112, 203, 77]
    );
    assert_eq!(doc.composite.data, [33, 112, 203]);
}

#[test]
fn multichannel_preserves_source_component_order() {
    let cmyk = [71, 179, 215, 229];
    let lab = [153, 118, 159];
    let gray = [64];
    for (mode, plates, names) in [
        (
            ColorMode::Cmyk,
            cmyk.as_slice(),
            &["Cyan", "Magenta", "Yellow", "Black"][..],
        ),
        (ColorMode::Lab, lab.as_slice(), &["Lightness", "a", "b"][..]),
        (ColorMode::Grayscale, gray.as_slice(), &["Black"][..]),
    ] {
        let pixels = match mode {
            ColorMode::Cmyk => pictura_codec::cmyk_to_rgb(plates),
            ColorMode::Lab => pictura_codec::lab_to_rgb(plates),
            _ => plates.to_vec(),
        };
        let mut doc = Document::new(
            1,
            1,
            if mode == ColorMode::Grayscale {
                mode
            } else {
                ColorMode::Rgb
            },
            BitDepth::Eight,
        );
        doc.composite
            .replace(PixelBuffer {
                width: 1,
                height: 1,
                channels: pixels.len() as u8,
                data: pixels.into(),
            })
            .unwrap();
        if mode != ColorMode::Grayscale {
            doc.source.set_encoding(Some(mode), None).unwrap();
            doc.source
                .replace_planes(Some(pictura_core::SourcePlanes {
                    width: 1,
                    height: 1,
                    depth: BitDepth::Eight,
                    samples: Samples::U8(plates.to_vec().into()),
                }))
                .unwrap();
        }

        convert_mode(&mut doc, ColorMode::Multichannel).unwrap();

        assert_eq!(pictura_codec::alpha_names(&doc), names);
        assert_eq!(
            doc.channels.iter().map(|c| c.data[0]).collect::<Vec<_>>(),
            plates
        );
    }
}

#[test]
fn multichannel_native_plates_round_trip() {
    let values = [0x1234, 0x5678, 0x9abc, 0x0def];
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace(PixelBuffer {
            width: 1,
            height: 1,
            channels: 3,
            data: vec![0x12, 0x56, 0x9a].into(),
        })
        .unwrap();
    doc.channels.push(Channel {
        id: 0,
        data: vec![0x0d].into(),
    });
    pictura_codec::set_alpha_names(&mut doc, &["Saved".into()]);
    doc.source
        .set_native(
            Some(BitDepth::Sixteen),
            Some(pictura_core::SourcePlanes {
                width: 1,
                height: 1,
                depth: BitDepth::Sixteen,
                samples: Samples::U16(values.to_vec().into()),
            }),
        )
        .unwrap();

    convert_mode(&mut doc, ColorMode::Multichannel).unwrap();

    for (write, version) in [
        (write_psd as fn(&Document) -> _, 1),
        (pictura_codec::write_psb, 2),
    ] {
        let encoded = write(&doc).unwrap();
        assert_eq!(&encoded[4..6], &[0, version]);
        assert_eq!(&encoded[24..26], &[0, 7]);
        let reopened = read_psd(&encoded).unwrap();
        assert!(reopened.source.multichannel_plates());
        assert_eq!(reopened.document_bit_depth(), BitDepth::Sixteen);
        assert_eq!(
            pictura_codec::alpha_names(&reopened),
            ["Cyan", "Magenta", "Yellow", "Saved"]
        );
        assert_eq!(
            reopened.source.planes().as_ref().unwrap().samples.to_u16(),
            values
        );
        assert_eq!(reopened.composite, doc.composite);
    }
}

#[test]
fn multichannel_rgb_plate_polarity() {
    let pixels = [
        [0, 0, 0],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
    ];
    for depth in [BitDepth::Eight, BitDepth::Sixteen] {
        let mut doc = rgb_doc(5, 1, &pixels);
        if depth == BitDepth::Sixteen {
            convert_bit_depth(&mut doc, depth).unwrap();
        }
        convert_mode(&mut doc, ColorMode::Multichannel).unwrap();
        for (channel, plate) in doc.channels.iter().enumerate() {
            let expected: Vec<_> = pixels.iter().map(|pixel| pixel[channel]).collect();
            assert_eq!(plate.data, expected);
            if depth == BitDepth::Sixteen {
                let start = channel * pixels.len();
                let actual = doc
                    .source
                    .planes()
                    .as_ref()
                    .unwrap()
                    .samples
                    .slice(start..start + pixels.len())
                    .unwrap()
                    .to_u16();
                assert_eq!(
                    actual,
                    expected
                        .iter()
                        .map(|&sample| u16::from(sample) * 257)
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn multichannel_refuses_without_mutation() {
    let mut indexed = rgb_doc(1, 1, &[[20, 42, 180]]);
    convert_to_indexed(
        &mut indexed,
        indexed_options(ColorReduction::Adaptive, Dither::None),
    )
    .unwrap();
    let mut hdr = rgb_doc(1, 1, &[[20, 42, 180]]);
    convert_bit_depth(&mut hdr, BitDepth::ThirtyTwo).unwrap();
    let mut over_capacity = rgb_doc(1, 1, &[[20, 42, 180]]);
    over_capacity.channels = (0..54)
        .map(|id| Channel {
            id,
            data: vec![0].into(),
        })
        .collect();

    for mut doc in [indexed, hdr, over_capacity] {
        let original = doc.clone();
        assert!(!can_convert_mode(&doc, ColorMode::Multichannel));
        assert!(convert_mode(&mut doc, ColorMode::Multichannel).is_err());
        assert_eq!(doc, original);
    }
}

#[test]
fn multichannel_geometry_keeps_plates_layerless_and_savable() {
    for depth in [BitDepth::Eight, BitDepth::Sixteen] {
        let mut source = rgb_doc(
            2,
            2,
            &[[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]],
        );
        if depth == BitDepth::Sixteen {
            convert_bit_depth(&mut source, depth).unwrap();
        }
        convert_mode(&mut source, ColorMode::Multichannel).unwrap();
        if depth == BitDepth::Sixteen {
            let mut store = source.source.planes().clone().unwrap();
            let Samples::U16(ref mut samples) = store.samples else {
                unreachable!()
            };
            for value in samples.iter_mut() {
                *value = (*value & 0xff00) | 0x34;
            }
            source.source.replace_planes(Some(store)).unwrap();
        }

        let mut cropped = source.clone();
        assert!(crate::crop_document(&mut cropped, 1, 0, 1, 2));
        assert_eq!(cropped.channels[0].data, [40, 100]);
        let mut resized = source.clone();
        crate::resize_document(&mut resized, 4, 2, pictura_ops::Resample::Nearest).unwrap();
        assert_eq!(resized.channels[0].data, [10, 10, 40, 40, 70, 70, 100, 100]);
        let mut rotated = source.clone();
        crate::rotate_document(&mut rotated, 1).unwrap();
        assert_eq!(rotated.channels[0].data, [70, 10, 100, 40]);
        let mut canvas = source.clone();
        crate::resize_canvas_document(&mut canvas, 3, 2, pictura_ops::Anchor::TopLeft).unwrap();
        assert_eq!(canvas.channels[0].data, [10, 40, 255, 70, 100, 255]);
        let mut expanded_crop = source.clone();
        assert!(crate::crop_document_grow(&mut expanded_crop, -1, 0, 3, 2));
        assert_eq!(expanded_crop.channels[0].data, [255, 10, 40, 255, 70, 100]);

        for doc in [cropped, resized, rotated, canvas, expanded_crop] {
            assert!(doc.layers.is_empty());
            assert_eq!(
                doc.composite.buffer(),
                &pictura_codec::multichannel_preview(&doc)
            );
            let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
            assert_eq!(back.channels, doc.channels);
            assert_eq!(back.composite, doc.composite);
            assert_eq!(back.document_bit_depth(), depth);
            if depth == BitDepth::Sixteen {
                assert_eq!(back.source.planes(), doc.source.planes());
            }
        }
        let before = source.clone();
        assert!(!crate::rotate_document_in(&mut source, 17.0, (1.0, 1.0)));
        assert_eq!(source, before);
        assert!(crate::perspective_crop_refusal(&source).is_some());
    }
}

fn bitmap_gray_doc(width: u32, height: u32, gray: &[u8]) -> Document {
    let pixels: Vec<_> = gray.iter().map(|&value| [value; 3]).collect();
    let mut doc = rgb_doc(width, height, &pixels);
    convert_mode(&mut doc, ColorMode::Grayscale).unwrap();
    doc
}

#[test]
fn bitmap_halftone_rotates_and_repeats_at_document_resolution() {
    let mut gray = bitmap_gray_doc(16, 16, &[128; 256]);
    pictura_codec::set_document_resolution(
        &mut gray,
        pictura_codec::Resolution {
            ppi: 72.0,
            per_cm: false,
        },
    );
    let mut horizontal = gray.clone();
    convert_to_bitmap(
        &mut horizontal,
        BitmapMethod::HalftoneScreen(HalftoneScreen {
            frequency: 9.0,
            per_cm: false,
            angle: 0.0,
            shape: HalftoneShape::Line,
        }),
    )
    .unwrap();
    let mut vertical = gray;
    convert_to_bitmap(
        &mut vertical,
        BitmapMethod::HalftoneScreen(HalftoneScreen {
            frequency: 9.0,
            per_cm: false,
            angle: 90.0,
            shape: HalftoneShape::Line,
        }),
    )
    .unwrap();

    let stripe = [255, 255, 0, 0, 0, 0, 255, 255];
    let (rows, _) = horizontal.composite.data[..256].as_chunks::<16>();
    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row, &[stripe[y % 8]; 16]);
    }
    for row in vertical.composite.data[..256].as_chunks::<16>().0 {
        assert_eq!(&row[..8], &stripe);
        assert_eq!(&row[8..], &stripe);
    }
    assert_eq!(document_bit_depth(&vertical), BitDepth::One);
    assert!(vertical.layers.is_empty());
    assert_eq!(roundtrip(&vertical).composite.data, vertical.composite.data);
}

#[test]
fn bitmap_custom_pattern_tiles_from_canvas_origin() {
    let pattern = pictura_codec::PatternPixels {
        pattern_id: "test tile".into(),
        width: 2,
        height: 3,
        rgba: vec![
            32, 32, 32, 255, 224, 224, 224, 255, 224, 224, 224, 255, 32, 32, 32, 255, 32, 32, 32,
            255, 32, 32, 32, 255,
        ],
    };
    let mut doc = bitmap_gray_doc(5, 7, &[128; 35]);
    convert_to_bitmap(&mut doc, BitmapMethod::CustomPattern(Some(&pattern))).unwrap();
    let tile = [[255, 0], [0, 255], [255, 255]];
    for (y, row) in doc.composite.data[..35]
        .as_chunks::<5>()
        .0
        .iter()
        .enumerate()
    {
        for (x, &white) in row.iter().enumerate() {
            assert_eq!(white, tile[y % 3][x % 2]);
        }
    }
}

#[test]
fn bitmap_invalid_screen_or_pattern_does_not_flatten() {
    let pattern = pictura_codec::PatternPixels {
        pattern_id: "truncated".into(),
        width: 2,
        height: 3,
        rgba: vec![0; 2],
    };
    let mut doc = bitmap_gray_doc(4, 2, &[128; 8]);
    let mut hidden = doc.layers[0].clone();
    hidden.visible = false;
    hidden.background = false;
    doc.layers.push(hidden).unwrap();
    let original = doc.composite.data.clone();
    for method in [
        BitmapMethod::HalftoneScreen(HalftoneScreen {
            frequency: f64::INFINITY,
            per_cm: false,
            angle: 0.0,
            shape: HalftoneShape::Round,
        }),
        BitmapMethod::CustomPattern(Some(&pattern)),
        BitmapMethod::CustomPattern(None),
    ] {
        assert!(convert_to_bitmap(&mut doc, method).is_err());
        assert_eq!(document_color_mode(&doc), ColorMode::Grayscale);
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.composite.data, original);
    }
}

#[test]
fn bitmap_screen_units_shapes_rotation_and_endpoints() {
    let mut gray = [175; 32 * 32];
    gray[0] = 0;
    gray[1] = 255;
    let source = bitmap_gray_doc(32, 32, &gray);
    let render = |options| {
        let mut doc = source.clone();
        convert_to_bitmap(&mut doc, BitmapMethod::HalftoneScreen(options)).unwrap();
        doc.composite.data.clone()
    };
    let opts = HalftoneScreen {
        frequency: 10.125 * 2.54,
        per_cm: false,
        angle: 23.0,
        shape: HalftoneShape::Round,
    };
    let mut outputs = Vec::new();
    for shape in [
        HalftoneShape::Round,
        HalftoneShape::Diamond,
        HalftoneShape::Ellipse,
        HalftoneShape::Line,
        HalftoneShape::Square,
        HalftoneShape::Cross,
    ] {
        let options = HalftoneScreen { shape, ..opts };
        let output = render(options);
        assert_eq!(&output[..2], &[0, 255]);
        assert!(output[2..1024].contains(&0));
        assert!(output[2..1024].contains(&255));
        assert!(!outputs.contains(&output), "{shape:?}");
        assert_eq!(
            render(HalftoneScreen {
                frequency: 10.125,
                per_cm: true,
                ..options
            }),
            output
        );
        assert_ne!(
            render(HalftoneScreen {
                angle: 0.0,
                ..options
            }),
            output
        );
        outputs.push(output);
    }
}

#[test]
fn bitmap_screen_boundaries_and_nonfinite_are_checked() {
    let source = bitmap_gray_doc(2, 2, &[128; 4]);
    for (frequency, per_cm, angle, valid) in [
        (1.0, false, -180.0, true),
        (999.999, false, 180.0, true),
        (0.4, true, -180.0, true),
        (400.0, true, 180.0, true),
        (0.999, false, 0.0, false),
        (1000.0, false, 0.0, false),
        (0.399, true, 0.0, false),
        (400.01, true, 0.0, false),
        (9.0, false, -180.01, false),
        (9.0, false, 180.01, false),
        (9.0, false, f64::NAN, false),
        (f64::INFINITY, true, 0.0, false),
    ] {
        let mut doc = source.clone();
        assert_eq!(
            convert_to_bitmap(
                &mut doc,
                BitmapMethod::HalftoneScreen(HalftoneScreen {
                    frequency,
                    per_cm,
                    angle,
                    shape: HalftoneShape::Round,
                })
            )
            .is_ok(),
            valid
        );
        if !valid {
            assert_eq!(document_color_mode(&doc), ColorMode::Grayscale);
            assert_eq!(doc.composite.data, source.composite.data);
        }
    }
}

#[test]
fn bitmap_pattern_endpoints_and_missing_or_invalid_tile() {
    let mut doc = bitmap_gray_doc(4, 1, &[0, 255, 128, 128]);
    let tile = pictura_codec::PatternPixels {
        pattern_id: "extremes".into(),
        width: 2,
        height: 1,
        rgba: vec![0, 0, 0, 255, 0, 0, 0, 0],
    };
    let mut invalid = tile.clone();
    invalid.height = 0;
    for pattern in [None, Some(&invalid)] {
        assert!(convert_to_bitmap(&mut doc, BitmapMethod::CustomPattern(pattern)).is_err());
        assert_eq!(document_color_mode(&doc), ColorMode::Grayscale);
        assert_eq!(&doc.composite.data[..4], &[0, 255, 128, 128]);
    }
    convert_to_bitmap(&mut doc, BitmapMethod::CustomPattern(Some(&tile))).unwrap();
    assert_eq!(&doc.composite.data[..4], &[0, 255, 255, 0]);
}

#[test]
fn bitmap_screen_shapes_match_independent_area_ranks() {
    // Count each score directly instead of sorting or using the threshold table.
    fn score(shape: HalftoneShape, x: usize, y: usize) -> usize {
        match shape {
            HalftoneShape::Round => x * x + y * y,
            HalftoneShape::Diamond => x + y,
            HalftoneShape::Ellipse => x * x + 3 * y * y,
            HalftoneShape::Line => y,
            HalftoneShape::Square => x.max(y),
            HalftoneShape::Cross => x.max(y) + 3 * x.min(y),
        }
    }
    let shapes = [
        HalftoneShape::Round,
        HalftoneShape::Diamond,
        HalftoneShape::Ellipse,
        HalftoneShape::Line,
        HalftoneShape::Square,
        HalftoneShape::Cross,
    ];
    let points = [(0usize, 0usize), (1, 2), (3, 5), (7, 1), (11, 9), (15, 15)];
    for shape in shapes {
        for (angle, sin, cos) in [
            (0.0, 0.0_f64, 1.0_f64),
            (23.0, 0.39073112848927377, 0.9205048534524404),
        ] {
            for gray in [48u8, 96, 160, 208] {
                let result = flat::bitmap_whites(
                    &vec![gray; 16 * 16],
                    16,
                    BitmapMethod::HalftoneScreen(HalftoneScreen {
                        frequency: 9.0,
                        per_cm: false,
                        angle,
                        shape,
                    }),
                    72.0,
                );
                for (x, y) in points {
                    let px = x as f64 + 0.5;
                    let py = y as f64 + 0.5;
                    let u = (((px * cos + py * sin) / 8.0).rem_euclid(1.0) * 128.0) as usize;
                    let v = (((py * cos - px * sin) / 8.0).rem_euclid(1.0) * 128.0) as usize;
                    let distance = |i: usize| (2 * i + 1).abs_diff(128);
                    let target = score(shape, distance(u), distance(v));
                    let mut lower = 0usize;
                    let mut equal = 0usize;
                    for cy in 0..128 {
                        for cx in 0..128 {
                            let other = score(shape, distance(cx), distance(cy));
                            lower += usize::from(other < target);
                            equal += usize::from(other == target);
                        }
                    }
                    let threshold = 255 - ((2 * lower + equal) * 128 / (128 * 128)) as u8;
                    assert_eq!(
                        result[y * 16 + x],
                        gray > threshold,
                        "{shape:?} angle {angle} at ({x},{y}) gray {gray}"
                    );
                }
            }
        }
    }
}

fn indexed_transparent_source() -> Document {
    Document::from_rgba("Source", 2, 1, &[49, 23, 17, 255, 46, 23, 17, 0])
}

#[test]
fn forced_indexed_transparency_round_trips() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    convert_to_indexed_with_forced(&mut doc, options, &[[0; 3], [255; 3]], false).unwrap();
    let reopened = roundtrip(&doc);
    let table = indexed_color_table(&reopened).unwrap();
    assert!(table.colors.contains(&[0; 3]));
    assert!(table.colors.contains(&[255; 3]));
    assert_ne!(table.transparent, None);
    assert_eq!(layer_plane(&reopened, -1), vec![255, 0]);
}

#[test]
fn indexed_conversion_without_transparency_uses_white() {
    let mut doc = indexed_transparent_source();
    let options = indexed_options(ColorReduction::Adaptive, Dither::None);
    convert_to_indexed(&mut doc, options).unwrap();
    assert_eq!(layer_plane(&doc, 0), vec![49, 255]);
    assert_eq!(layer_plane(&doc, 1), vec![23, 255]);
    assert_eq!(layer_plane(&doc, -1), vec![255, 255]);
    assert_eq!(indexed_color_table(&doc).unwrap().transparent, None);
}

#[test]
fn forced_indexed_transparency_capacity_boundary() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=u8::MAX).map(|v| [v, 1, 2]).collect();
    let original = doc.clone();
    assert!(convert_to_indexed_with_forced(&mut doc, options, &forced, false).is_err());
    assert_eq!(doc, original);
    convert_to_indexed_with_forced(&mut doc, options, &forced[..255], false).unwrap();
    let table = indexed_color_table(&doc).unwrap();
    assert_eq!(table.transparent, Some(255));
    assert_eq!(&table.colors[..255], &forced[..255]);
}

#[test]
fn invalid_exact_request_is_atomic() {
    let mut doc = indexed_transparent_source();
    let before = doc.clone();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=254).map(|v| [v, 1, 2]).collect();
    assert!(!indexed_exact_fits(&doc, options, &forced));
    assert!(convert_to_indexed_with_forced(&mut doc, options, &forced, true).is_err());
    assert_eq!(doc, before);
}

#[test]
fn exact_indexed_conversion_fits_capacity() {
    let mut doc = indexed_transparent_source();
    let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
    options.transparency = true;
    let forced: Vec<_> = (0..=253).map(|v| [v, 1, 2]).collect();
    assert!(indexed_exact_fits(&doc, options, &forced));
    convert_to_indexed_with_forced(&mut doc, options, &forced, true).unwrap();
    let table = indexed_color_table(&doc).unwrap();
    assert_eq!(table.colors.len(), 256);
    assert_eq!(table.transparent, Some(255));
    assert!(table.colors.contains(&[49, 23, 17]));
    assert_eq!(layer_plane(&doc, 0)[0], 49);
}

#[test]
fn color_table_replacement_preserves_indices() {
    let mut doc = indexed_transparent_source();
    let options = indexed_options(ColorReduction::Adaptive, Dither::None);
    convert_to_indexed(&mut doc, options).unwrap();
    let original = doc.source.planes().clone();
    let mut table = indexed_color_table(&doc).unwrap();
    table.colors[0] = [10, 20, 30];
    table.transparent = Some(1);
    assert!(replace_indexed_color_table(&mut doc, table.clone()).unwrap());
    assert_eq!(doc.source.planes(), &original);
    assert_eq!(layer_plane(&doc, 0)[0], 10);
    assert_eq!(layer_plane(&doc, -1), vec![255, 0]);
    assert_eq!(indexed_color_table(&doc).unwrap(), table);
}

#[test]
fn flat_grayscale_alpha_converts_to_indexed_with_and_without_transparency() {
    let mut source = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    source.composite.replace(PixelBuffer {
        width: 2, height: 1, channels: 2, data: vec![40, 80, 255, 0].into(),
    }).unwrap();
    for transparency in [false, true] {
        let mut doc = source.clone();
        let mut options = indexed_options(ColorReduction::Adaptive, Dither::None);
        options.transparency = transparency;
        assert!(indexed_exact_fits(&doc, options, &[]));
        convert_to_indexed_with_forced(&mut doc, options, &[], true).unwrap();
        assert_eq!(layer_plane(&doc, 0)[0], 40);
        assert_eq!(layer_plane(&doc, -1), if transparency { vec![255, 0] } else { vec![255, 255] });
        if !transparency {
            assert_eq!(layer_plane(&doc, 0)[1], 255);
        }
    }
}
