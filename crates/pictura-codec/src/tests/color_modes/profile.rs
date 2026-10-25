//! Profile-aware CMYK read, write, policy, and Convert tests.

use super::*;

fn with_icc(psd: Vec<u8>, icc: &[u8]) -> Vec<u8> {
    let color_len = u32::from_be_bytes(psd[26..30].try_into().unwrap()) as usize;
    let at = 30 + color_len;
    let section = encode_image_resources(&[frame_image_resource(ICC_PROFILE, "", icc)]);
    let mut out = psd[..at].to_vec();
    out.extend_from_slice(&(section.len() as u32).to_be_bytes());
    out.extend_from_slice(&section);
    out.extend_from_slice(&psd[at + 4..]);
    out
}

fn profile_options(policy: Policy) -> OpenOptions {
    OpenOptions {
        policy,
        working_cmyk: Some(Profile::coated_cmyk().to_icc()),
        conversion: ConversionOptions {
            dither: false,
            ..ConversionOptions::default()
        },
    }
}

fn cmyk_pixel_psd(icc: Option<&[u8]>) -> Vec<u8> {
    let psd = flat_psd(8, 4, 4, 1, 1, &[&[10], &[20], &[30], &[40]]);
    match icc {
        Some(icc) => with_icc(psd, icc),
        None => psd,
    }
}

fn expected_rgb(cmyk: &[u8]) -> Vec<u8> {
    pictura_color::convert_space(
        &Profile::coated_cmyk(),
        &Profile::srgb(),
        cmyk,
        Intent::RelativeColorimetric,
        true,
    )
    .expect("lcms2 conversion")
}

fn resource_1039(doc: &Document) -> Option<Vec<u8>> {
    decode_image_resources(doc)
        .into_iter()
        .find(|r| r.id == ICC_PROFILE)
        .map(|r| r.data)
}

#[test]
fn cmyk_profile_read_uses_embedded() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&profile));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    assert_eq!(doc.composite.data.to_vec(), expected_rgb(&[10, 20, 30, 40]));
    assert_eq!(resource_1039(&doc).as_deref(), Some(profile.as_slice()));
    assert!(doc.document_icc.is_none());
    assert!(doc.source_icc.is_none());
}

#[test]
fn cmyk_profile_read_uses_working() {
    let psd = cmyk_pixel_psd(None);
    let options = profile_options(Policy::Preserve);
    let doc = read_psd_with_options(&psd, &options).expect("reads");
    assert_eq!(doc.composite.data.to_vec(), expected_rgb(&[10, 20, 30, 40]));
    let tagged = resource_1039(&doc).expect("working profile");
    let parsed = Profile::from_icc(&tagged).expect("parses");
    assert_eq!(parsed.description(), Profile::coated_cmyk().description());
}

#[test]
fn cmyk_profile_read_falls_back() {
    let psd = cmyk_pixel_psd(None);
    let doc = read_psd(&psd).expect("reads");
    assert_eq!(doc.composite.data.to_vec(), cmyk_to_rgb(&[10, 20, 30, 40]));
    assert!(doc.image_resources.is_empty());
}

#[test]
fn cmyk_profile_policy_preserve() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&profile));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    assert_eq!(resource_1039(&doc).as_deref(), Some(profile.as_slice()));
    assert_eq!(doc.composite.data.to_vec(), expected_rgb(&[10, 20, 30, 40]));
}

#[test]
fn cmyk_profile_policy_convert() {
    let embedded = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&embedded));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Convert)).expect("reads");
    assert_eq!(doc.source_icc.as_deref(), Some(embedded.as_slice()));
    let tagged = resource_1039(&doc).expect("working profile");
    let parsed = Profile::from_icc(&tagged).expect("parses");
    assert_eq!(parsed.description(), Profile::coated_cmyk().description());
    let working = Profile::coated_cmyk();
    let stored = doc.source_planes.as_ref().expect("retained planes");
    let expected = pictura_color::convert_space(
        &working,
        &working,
        &[10, 20, 30, 40],
        Intent::RelativeColorimetric,
        true,
    )
    .expect("cmyk to cmyk");
    let Samples::U8(values) = &stored.samples else {
        panic!("8-bit store");
    };
    assert_eq!(values.as_slice(), expected.as_slice());
}

#[test]
fn cmyk_profile_policy_off() {
    let psd = cmyk_pixel_psd(Some(&Profile::coated_cmyk().to_icc()));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Off)).expect("reads");
    assert!(resource_1039(&doc).is_none());
    assert!(doc.document_icc.is_none());
    assert!(doc.source_icc.is_none());
    assert_eq!(doc.composite.data.to_vec(), cmyk_to_rgb(&[10, 20, 30, 40]));
}

#[test]
fn cmyk_profile_write_keeps_profile() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&profile));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    let written = write_psd(&doc).expect("writes");
    let out = read_psd_with_options(&written, &profile_options(Policy::Preserve)).expect("reads");
    assert_eq!(resource_1039(&out).as_deref(), Some(profile.as_slice()));
    assert_eq!(out.source_planes, doc.source_planes);
}

#[test]
fn cmyk_profile_write_reencodes() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&profile));
    let mut doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    doc.composite.data[..3].copy_from_slice(&[200, 50, 100]);
    let written = write_psd(&doc).expect("writes");
    let out = read_psd_with_options(&written, &profile_options(Policy::Preserve)).expect("reads");
    for (a, b) in [200u8, 50, 100].iter().zip(&out.composite.data[..3]) {
        assert!(a.abs_diff(*b) <= 8, "edit {a} became {b}");
    }
}

#[test]
fn cmyk_profile_fixture_matches_lcms2() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = layered_psd(
        4,
        4,
        &[&[10], &[20], &[30], &[40]],
        &[(0, &[10]), (1, &[20]), (2, &[30]), (3, &[40])],
    );
    let psd = with_icc(psd, &profile);
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    let expected = expected_rgb(&[10, 20, 30, 40]);
    assert_eq!(doc.composite.data.to_vec(), expected);
    for id in 0..3i16 {
        let channel = doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == id)
            .expect("layer channel");
        assert_eq!(channel.data.to_vec(), vec![expected[id as usize]]);
    }
}

#[test]
fn cmyk_profile_free_fixture_uses_fallback() {
    let psd = cmyk_pixel_psd(None);
    let doc = read_psd(&psd).expect("reads");
    assert_eq!(doc.composite.data.to_vec(), cmyk_to_rgb(&[10, 20, 30, 40]));
    assert_eq!(
        read_psd(&flat_psd(8, 4, 4, 1, 1, &[&[0], &[0], &[0], &[0]]))
            .unwrap()
            .composite
            .data
            .to_vec(),
        vec![0, 0, 0]
    );
    assert_eq!(
        read_psd(&flat_psd(8, 4, 4, 1, 1, &[&[255], &[255], &[255], &[255]]))
            .unwrap()
            .composite
            .data
            .to_vec(),
        vec![255, 255, 255]
    );
}

#[test]
fn convert_normalized_cmyk_keeps_working_profile() {
    let psd = cmyk_pixel_psd(Some(&Profile::coated_cmyk().to_icc()));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Convert)).expect("reads");
    let tagged = resource_1039(&doc).expect("working profile");
    assert_eq!(
        Profile::from_icc(&tagged).expect("parses").description(),
        Profile::coated_cmyk().description()
    );
    let written = write_psd(&doc).expect("writes");
    let out = read_psd_with_options(&written, &profile_options(Policy::Convert)).expect("reads");
    assert_eq!(
        Profile::from_icc(&resource_1039(&out).expect("working profile"))
            .expect("parses")
            .description(),
        Profile::coated_cmyk().description()
    );
}

#[test]
fn convert_options_match_lcms2() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![40, 80, 120, 220, 30, 90].into();
    let original = doc.composite.data.to_vec();
    let options = ConversionOptions {
        intent: Intent::Perceptual,
        black_point_compensation: true,
        dither: false,
    };
    assert!(convert_document_with(
        &mut doc,
        &Profile::adobe_rgb(),
        options
    ));
    let interleaved = [
        original[0],
        original[2],
        original[4],
        original[1],
        original[3],
        original[5],
    ];
    let expected = pictura_color::convert_space(
        &Profile::srgb(),
        &Profile::adobe_rgb(),
        &interleaved,
        Intent::Perceptual,
        true,
    )
    .unwrap();
    let (pixels, _) = expected.as_chunks::<3>();
    let planar: Vec<u8> = (0..3)
        .flat_map(|c| pixels.iter().map(move |p| p[c]))
        .collect();
    assert_eq!(doc.composite.data.to_vec(), planar);
}

#[test]
fn convert_srgb_to_adobe_matches_lcms2() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![30, 140, 210].into();
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    assert!(convert_document_with(
        &mut doc,
        &Profile::adobe_rgb(),
        options
    ));
    let expected = pictura_color::convert_space(
        &Profile::srgb(),
        &Profile::adobe_rgb(),
        &[30, 140, 210],
        Intent::RelativeColorimetric,
        true,
    )
    .unwrap();
    assert_eq!(doc.composite.data.to_vec(), expected);
}

#[test]
fn convert_flatten_leaves_one_background() {
    let mut doc = rgb_doc(2, 1, &[[255, 0, 0], [0, 255, 0]]);
    pictura_render::add_layer(&mut doc, 0, "Second");
    pictura_render::flatten(&mut doc).expect("flattens");
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    assert!(convert_document_with(
        &mut doc,
        &Profile::adobe_rgb(),
        options
    ));
    assert_eq!(doc.layers.len(), 1);
    assert!(doc.layers[0].background);
}

#[test]
fn convert_refuses_non_rgb() {
    let mut doc = flat_cmyk_document();
    let before = doc.clone();
    assert!(!convert_document_with(
        &mut doc,
        &Profile::adobe_rgb(),
        ConversionOptions::default()
    ));
    assert_eq!(doc, before);
}

fn flat_cmyk_document() -> Document {
    let mut doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    doc.composite.data = vec![10, 20, 30, 40].into();
    doc
}

fn rgb_doc(width: u32, height: u32, pixels: &[[u8; 3]]) -> Document {
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    Document::from_rgba("Layer 1", width, height, &rgba)
}

#[test]
fn psd_embed_toggle_keeps_resource() {
    let profile = Profile::coated_cmyk().to_icc();
    let psd = cmyk_pixel_psd(Some(&profile));
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    let written = write_psd(&doc).expect("writes");
    let out = read_psd_with_options(&written, &profile_options(Policy::Preserve)).expect("reads");
    assert_eq!(resource_1039(&out).as_deref(), Some(profile.as_slice()));
}

#[test]
fn psd_embed_toggle_strips_resource() {
    let psd = cmyk_pixel_psd(Some(&Profile::coated_cmyk().to_icc()));
    let mut doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    remove_document_profile(&mut doc);
    let written = write_psd(&doc).expect("writes");
    assert!(
        probe_psd_profile(&written).embedded.is_none(),
        "the saved file carries no resource 1039"
    );
}

#[test]
fn cmyk_profile_committed_fixture_matches_lcms2() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cmyk_profile.psd"
    );
    let bytes = std::fs::read(path).expect("profile-bearing fixture");
    let doc = read_psd_with_options(&bytes, &profile_options(Policy::Preserve))
        .expect("profile-bearing fixture reads");
    let stored = doc.source_planes.as_ref().expect("retained CMYK planes");
    let Samples::U8(values) = &stored.samples else {
        panic!("8-bit CMYK store");
    };
    let n = doc.width as usize * doc.height as usize;
    let mut interleaved = Vec::with_capacity(n * 4);
    for i in 0..n {
        for c in 0..4 {
            interleaved.push(values[c * n + i]);
        }
    }
    let expected = pictura_color::convert_space(
        &Profile::coated_cmyk(),
        &Profile::srgb(),
        &interleaved,
        Intent::RelativeColorimetric,
        true,
    )
    .expect("lcms2 reference");
    let mut planar = vec![0u8; n * 3];
    for i in 0..n {
        for c in 0..3 {
            planar[c * n + i] = expected[i * 3 + c];
        }
    }
    assert_eq!(doc.composite.data.to_vec(), planar);
    let tagged = resource_1039(&doc).expect("embedded profile kept");
    assert_eq!(
        Profile::from_icc(&tagged).expect("parses").description(),
        Profile::coated_cmyk().description()
    );
}

fn native_layer_rgb(doc: &Document) -> Vec<u8> {
    (0i16..3)
        .map(|id| {
            doc.layers[0]
                .channels
                .iter()
                .find(|c| c.id == id)
                .expect("layer color channel")
                .data[0]
        })
        .collect()
}

#[test]
fn convert_does_not_reinterpret_native_cmyk_layers_as_rgb() {
    let profile = Profile::coated_cmyk().to_icc();
    let cmyk = [0x80_u8, 0x00, 0x40, 0x00, 0x20, 0x00, 0xc8, 0x00];
    let composite = [&cmyk[0..2], &cmyk[2..4], &cmyk[4..6], &cmyk[6..8]];
    let layer: Vec<(i16, &[u8])> = (0i16..4)
        .map(|id| (id, &cmyk[id as usize * 2..id as usize * 2 + 2]))
        .collect();
    let psd = with_icc(layered_psd_depth(16, 4, 4, &composite, &layer), &profile);
    let doc = read_psd_with_options(&psd, &profile_options(Policy::Preserve)).expect("reads");
    assert_eq!(doc.source_mode, Some(ColorMode::Cmyk));
    assert_eq!(doc.source_depth, Some(BitDepth::Sixteen));
    assert_eq!(
        native_layer_rgb(&doc),
        doc.composite.data[..3].to_vec(),
        "read keeps the layer and composite in sync"
    );

    let mut converted = doc;
    assert!(convert_document_with(
        &mut converted,
        &Profile::srgb(),
        ConversionOptions {
            dither: false,
            ..Default::default()
        }
    ));
    assert_eq!(
        native_layer_rgb(&converted),
        converted.composite.data[..3].to_vec(),
        "Convert must not transform the retained CMYK layer store as RGB"
    );
}
