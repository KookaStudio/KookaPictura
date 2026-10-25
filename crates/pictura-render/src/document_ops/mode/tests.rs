use super::*;
use pictura_codec::{
    read_psd, web_safe_palette, write_psd, ColorReduction, Dither, PaletteOptions,
};
use pictura_core::{Layer, Samples};

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
        .channels
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.to_vec())
        .expect("channel")
}

fn indexed_options(reduction: ColorReduction, dither: Dither) -> PaletteOptions {
    PaletteOptions {
        reduction,
        colors: 256,
        dither,
        amount: 75,
        transparency: false,
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
        .channels
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
    assert_eq!(read.source_mode, Some(ColorMode::Cmyk));
    assert_eq!(layer_plane(&read, 0), layer_plane(&doc, 0));
    assert_eq!(layer_plane(&read, 2), layer_plane(&doc, 2));
    // The retained store is what was written, byte for byte.
    assert_eq!(read.source_planes, doc.source_planes);
}

#[test]
fn lab_conversion_saves_as_lab_and_reads_back_the_shown_rgb() {
    let mut doc = ramp(8, 4);
    convert_mode(&mut doc, ColorMode::Lab).expect("converts");
    let read = roundtrip(&doc);
    assert_eq!(read.source_mode, Some(ColorMode::Lab));
    assert_eq!(layer_plane(&read, 1), layer_plane(&doc, 1));
    // Converting again from Lab to RGB drops the Lab store: a plain RGB save.
    convert_mode(&mut doc, ColorMode::Rgb).unwrap();
    assert_eq!(doc.source_mode, None);
    assert_eq!(roundtrip(&doc).source_mode, None);
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
    assert_eq!(read.source_mode, Some(ColorMode::Indexed));
    assert_eq!(read.source_palette, doc.source_palette);
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
        let _ = doc.layers.push(Layer {
            name: "group".into(),
            is_group: true,
            ..Default::default()
        });
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
    assert_eq!(read.source_mode, Some(ColorMode::Bitmap));
    assert_eq!(read.composite.data, doc.composite.data);
}

#[test]
fn bitmap_dithers_keep_the_mean_tone() {
    let gray = vec![64u8; 64 * 64];
    for method in [BitmapMethod::Pattern, BitmapMethod::Diffusion] {
        let whites = flat::bitmap_whites(&gray, 64, method);
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
    assert_eq!(doc.source_mode, None);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(layer_plane(&doc, 0), vec![0, 255]);
}

#[test]
fn eight_to_sixteen_widens_and_saves_sixteen_bit() {
    let mut doc = ramp(4, 4);
    convert_bit_depth(&mut doc, BitDepth::Sixteen).expect("converts");
    assert_eq!(document_bit_depth(&doc), BitDepth::Sixteen);
    let read = roundtrip(&doc);
    assert_eq!(read.source_depth, Some(BitDepth::Sixteen));
    let Some(Samples::U16(v)) = read.source_planes.map(|s| s.samples) else {
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
    assert_eq!(roundtrip(&doc).source_depth, Some(BitDepth::ThirtyTwo));
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
    assert_eq!(doc.source_planes, None);
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
    assert_eq!(store.planes[0].1, Samples::U16(vec![19595]));
}

#[test]
fn sixteen_bit_cmyk_saves_cmyk_at_sixteen_bits() {
    let mut doc = ramp(4, 4);
    convert_mode(&mut doc, ColorMode::Cmyk).unwrap();
    convert_bit_depth(&mut doc, BitDepth::Sixteen).unwrap();
    let read = roundtrip(&doc);
    assert_eq!(read.source_mode, Some(ColorMode::Cmyk));
    assert_eq!(read.source_depth, Some(BitDepth::Sixteen));
    assert!(
        !can_convert_mode(&doc, ColorMode::Indexed),
        "Indexed needs 8-bit"
    );
    convert_bit_depth(&mut doc, BitDepth::Eight).unwrap();
    let read = roundtrip(&doc);
    assert_eq!(
        (read.source_mode, read.source_depth),
        (Some(ColorMode::Cmyk), None)
    );
}

#[test]
fn the_matrix_dims_unsupported_targets_and_refusals_do_not_mutate() {
    let mut doc = ramp(2, 2);
    assert!(!can_convert_mode(&doc, ColorMode::Rgb), "current mode");
    assert!(!can_convert_mode(&doc, ColorMode::Duotone));
    assert!(!can_convert_mode(&doc, ColorMode::Multichannel));
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
fn save_view_trims_the_display_alpha_only_for_a_source_mode() {
    let doc = ramp(2, 2);
    assert!(matches!(save_view(&doc), Cow::Borrowed(_)));
    let mut cmyk = doc.clone();
    convert_mode(&mut cmyk, ColorMode::Cmyk).unwrap();
    assert_eq!(cmyk.composite.channels, 4);
    assert_eq!(save_view(&cmyk).composite.channels, 3);
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
    let store = doc.source_planes.as_ref().expect("store");
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
    let store = doc.source_planes.as_ref().expect("store");
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
        },
    )
    .expect("reads");
    assert_eq!(read.source_mode, Some(ColorMode::Cmyk));
    assert_eq!(read.source_planes, doc.source_planes);
    for (a, b) in shown.iter().zip(&read.composite.data[..6]) {
        assert!(
            a.abs_diff(*b) <= 8,
            "{shown:?} -> {:?}",
            &read.composite.data[..6]
        );
    }
}
