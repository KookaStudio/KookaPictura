//! End-to-end native-depth editing: a covered tonal adjustment applied to a
//! depth-16 document's retained samples, saved and re-read, keeps the adjusted
//! native samples instead of the 8-bit widen.

use std::path::PathBuf;

use pictura_adjust::{apply_native, Adjustment, LevelsParams};
use pictura_codec::{read_psd, write_psd};
use pictura_core::{BitDepth, ColorMode, Document, Samples};
use pictura_paint::bucket::{fill, BucketPaint};
use pictura_paint::{Alpha, PaintMode, Rgba};
use pictura_render::{
    refresh_native_composite, transform_layer, transform_layer_quad, LayerTransform,
};

#[test]
fn native_depth_edit_survives_a_psd_round_trip() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pictura-codec/tests/fixtures/rgb16.psd");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut doc = read_psd(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let width = doc.width as usize;
    let height = doc.height as usize;
    let channels = doc.composite.channels;
    let source = doc
        .source_planes
        .as_mut()
        .expect("rgb16 retains source planes");
    assert_eq!(source.depth, BitDepth::Sixteen);

    let adjustment = Adjustment::Levels(LevelsParams {
        input_black: 0,
        input_white: 255,
        gamma: 1.0,
        output_black: 0,
        output_white: 200,
        red: None,
        green: None,
        blue: None,
    });
    apply_native(&adjustment, &mut source.samples, width, height, channels)
        .expect("native apply on retained samples");
    let adjusted = source.samples.clone();

    let plane = width * height * channels as usize;
    doc.composite.data = adjusted.narrow_to_u8()[..plane].to_vec().into();

    let out = write_psd(&doc).expect("save edited document");
    let back = read_psd(&out).expect("re-read saved document");
    assert_eq!(back.source_planes.unwrap().samples, adjusted);

    let Samples::U16(v) = &adjusted else {
        panic!("rgb16 retains u16 samples")
    };
    assert!(
        v.iter().any(|s| (s >> 8) != (s & 0xff)),
        "expected an adjusted sample that is not the 8-bit widen high*257"
    );
}

/// `refresh_native_composite` recomposites a layered depth-16 RGB document and
/// splices the native color planes back in: the extra plane is preserved
/// byte-for-byte and the saved file re-reads at 16 with native (non-widened)
/// composite samples.
#[test]
fn refresh_splices_native_color_planes_into_a_layered_depth_document() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-codec/tests/fixtures/rgb16_layered.psd");
    let bytes = std::fs::read(&path).unwrap();
    let mut doc = read_psd(&bytes).unwrap();
    assert_eq!(doc.source_depth, Some(BitDepth::Sixteen));
    assert!(!doc.layers.is_empty(), "fixture carries a layer");

    let plane = doc.width as usize * doc.height as usize;
    let color_channels = doc.mode.color_channels() as usize;
    let edited = doc.composite.data[..color_channels * plane].to_vec();
    let extras_before = doc.channels.clone();
    let retained_before = doc.source_planes.as_ref().unwrap().samples.to_bytes();

    assert!(
        refresh_native_composite(&mut doc),
        "refresh runs for a layered 16-bit RGB document"
    );
    assert_eq!(doc.channels, extras_before, "document extras untouched");

    let retained_after = doc.source_planes.as_ref().unwrap().samples.to_bytes();
    let cut = color_channels * plane * 2;
    assert_eq!(
        &retained_after[cut..],
        &retained_before[cut..],
        "native color planes replaced, extra planes kept byte-for-byte"
    );

    let out = write_psd(&doc).expect("save refreshed document");
    assert_eq!(
        u16::from_be_bytes(out[22..24].try_into().unwrap()),
        16,
        "output header keeps the source depth"
    );
    let back = read_psd(&out).expect("re-read saved document");
    let Samples::U16(v) = back.source_planes.unwrap().samples else {
        panic!("depth-16 read retains u16 samples")
    };
    for c in 0..color_channels {
        let native = &v[c * plane..(c + 1) * plane];
        assert!(
            (0..plane).any(|i| native[i] != (edited[c * plane + i] as u16) * 257),
            "color plane {c}: a native sample is not the 8-bit widening"
        );
    }
}

/// Out-of-scope documents are returned unchanged: no layers, a converted source
/// mode, no recorded source depth, and no retained source planes.
#[test]
fn refresh_leaves_out_of_scope_documents_untouched() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-codec/tests/fixtures/rgb16_layered.psd");
    let bytes = std::fs::read(&path).unwrap();
    let base = read_psd(&bytes).unwrap();

    let mut no_layers = base.clone();
    no_layers.layers = pictura_core::LayerStack::default();
    let mut converted = base.clone();
    converted.source_mode = Some(ColorMode::Cmyk);
    let mut no_depth = base.clone();
    no_depth.source_depth = None;
    let mut no_planes = base.clone();
    no_planes.source_planes = None;

    for (name, mut doc) in [
        ("no layers", no_layers),
        ("converted mode", converted),
        ("no source depth", no_depth),
        ("no source planes", no_planes),
    ] {
        let before = doc.clone();
        assert!(!refresh_native_composite(&mut doc), "{name}: not refreshed");
        assert_eq!(doc, before, "{name}: document unmutated");
    }
}

/// The 8×8 layered depth-16 fixture (one pixel layer with color, alpha, mask,
/// and an unmodeled channel; one document extra channel).
fn layered_fixture() -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-codec/tests/fixtures/rgb16_layered.psd");
    let bytes = std::fs::read(&path).unwrap();
    read_psd(&bytes).unwrap()
}

fn assert_layer_round_trips_native(apply: impl Fn(&mut Document)) {
    let mut doc = layered_fixture();
    assert_eq!(doc.source_depth, Some(BitDepth::Sixteen));
    apply(&mut doc);
    let out = write_psd(&doc).expect("save transformed document");
    assert_eq!(
        u16::from_be_bytes(out[22..24].try_into().unwrap()),
        16,
        "output header keeps the source depth"
    );
    let back = read_psd(&out).expect("re-read saved document");
    let store = back.layers[0]
        .source_channels
        .as_ref()
        .expect("the transformed layer kept a native store");
    let (_, samples) = store
        .planes
        .iter()
        .find(|(id, _)| *id == 0)
        .expect("the color plane survived");
    let Samples::U16(v) = samples else {
        panic!("depth-16 layer retains u16 samples")
    };
    assert!(
        v.iter().any(|s| (*s >> 8) != (*s & 0xff)),
        "the saved layer plane is native, not the 8-bit widen"
    );
}

#[test]
fn moved_scaled_and_projected_layers_keep_native_samples() {
    let translate = LayerTransform {
        scale_x: 1.0,
        scale_y: 1.0,
        angle_radians: 0.0,
        dx: 2.0,
        dy: 1.0,
    };
    let scale = LayerTransform {
        scale_x: 2.0,
        scale_y: 2.0,
        angle_radians: 0.0,
        dx: 0.0,
        dy: 0.0,
    };
    let quad = [(1.0, 0.0), (9.0, 1.0), (8.0, 9.0), (0.0, 8.0)];

    assert_layer_round_trips_native(|d| assert!(transform_layer(d, "0", translate)));
    assert_layer_round_trips_native(|d| assert!(transform_layer(d, "0", scale)));
    assert_layer_round_trips_native(|d| assert!(transform_layer_quad(d, "0", quad)));
}

/// A depth-16 fill writes the layer's native store, and the saved file keeps
/// the native samples: the re-read layer's store holds the fill, not the 8-bit
/// widening of the working copy.
#[test]
fn a_depth16_fill_survives_a_psd_round_trip() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-codec/tests/fixtures/rgb16_layered.psd");
    let bytes = std::fs::read(&path).unwrap();
    let mut doc = read_psd(&bytes).unwrap();
    assert_eq!(doc.source_depth, Some(BitDepth::Sixteen));

    // Fill the layer with a solid colour.
    let mask = vec![255u8; (doc.width * doc.height) as usize];
    let dirty = fill(
        &mut doc,
        "0",
        &mask,
        BucketPaint::Foreground(Rgba {
            r: 220,
            g: 0,
            b: 0,
            a: 255,
        }),
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    );
    assert!(dirty.is_some(), "the fill changed the layer");

    // The save path the app takes: the native composite is recomposited from
    // the layers before the file is written.
    assert!(
        refresh_native_composite(&mut doc),
        "the native composite is refreshed"
    );

    let out = write_psd(&doc).expect("save the filled document");
    let back = read_psd(&out).expect("re-read the filled document");
    let store = back.layers[0]
        .source_channels
        .as_ref()
        .expect("the layer kept its native store");
    let (_, samples) = store
        .planes
        .iter()
        .find(|(id, _)| *id == 0)
        .expect("the red plane");
    let Samples::U16(values) = samples else {
        panic!("depth-16 read retains u16 samples")
    };
    assert!(
        values.iter().all(|&v| v == 220 * 257),
        "the re-read samples are the fill: {values:?}"
    );
}

/// A filter committed on a depth-16 layer is what the saved file holds: the
/// retained native planes are re-emitted only while they still match the
/// working pixels, so the edited plane is written from the filtered result.
#[test]
fn a_depth16_filter_survives_a_psd_round_trip() {
    let mut doc = layered_fixture();
    assert_eq!(doc.source_depth, Some(BitDepth::Sixteen));
    let filter = pictura_filters::Filter::LensCorrection {
        params: pictura_filters::LensCorrectionParams {
            distortion: 100.0,
            vignette_amount: -100.0,
            edge: pictura_filters::LensEdge::Extension,
            ..Default::default()
        },
    };
    let before: Vec<u8> = doc.layers[0].channels[0].data.to_vec();
    pictura_render::apply_filter(&mut doc.layers[0], &filter, None, false).expect("filter");
    let after: Vec<u8> = doc.layers[0].channels[0].data.to_vec();
    assert_ne!(before, after, "the filter changed the working plane");

    let out = write_psd(&doc).expect("save the filtered document");
    let back = read_psd(&out).expect("re-read the filtered document");
    let saved = back.layers[0]
        .channels
        .iter()
        .find(|c| c.id == doc.layers[0].channels[0].id)
        .unwrap();
    assert_eq!(
        saved.data.to_vec(),
        after,
        "the saved layer holds the filter"
    );
    // Its native store is the filtered plane too, not the pre-filter samples.
    let (_, native) = back.layers[0]
        .source_channels
        .as_ref()
        .expect("the re-read layer keeps a native store")
        .planes
        .iter()
        .find(|(id, _)| *id == 0)
        .expect("the color plane");
    assert_eq!(native.narrow_to_u8(), after);
}
