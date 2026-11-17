use super::*;

const STROKE_GRADIENT_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/stroke_gradient.psd");
const STROKE_PATTERN_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/stroke_pattern.psd");

#[test]
fn stroke_gradient_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(STROKE_GRADIENT_FIXTURE).expect("stroke_gradient.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Stroked")
        .expect("Stroked layer");
    let stroke = decode_stroke(layer).expect("decodes the authored FrFX gradient");
    match &stroke.fill {
        StrokeFill::Gradient {
            params,
            align_with_layer,
        } => {
            assert_eq!(params.stops.len(), 2, "authored two stops");
            assert_eq!(params.kind, GradientKind::Linear);
            assert_eq!(params.angle_deg, 45.0);
            assert!(params.reverse);
            assert!(*align_with_layer);
        }
        other => panic!("expected a gradient fill, got {other:?}"),
    }

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        pictura_codec::edit_layer(l.as_mut(), |parts| parts.extra_blocks.clear());
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the gradient renders");
}

#[test]
fn stroke_pattern_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(STROKE_PATTERN_FIXTURE).expect("stroke_pattern.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Stroked")
        .expect("Stroked layer");
    let stroke = decode_stroke(layer).expect("decodes the authored FrFX pattern");
    match &stroke.fill {
        StrokeFill::Pattern { params, .. } => {
            assert_eq!(params.pattern_id, "pictura-pattern");
            assert!(!params.link_with_layer, "the authored Lnkd false wins");
        }
        other => panic!("expected a pattern fill, got {other:?}"),
    }

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        pictura_codec::edit_layer(l.as_mut(), |parts| parts.extra_blocks.clear());
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the pattern renders");
    // The fixture pattern library resolves the real tile, not the placeholder.
    let mut band = std::collections::BTreeSet::new();
    for y in 0..d.height {
        for x in 0..d.width {
            band.insert(rgb(&with_effect, x, y));
        }
    }
    assert!(
        band.contains(&[0, 255, 0]) || band.contains(&[0, 0, 255]),
        "the authored tile, not the grey placeholder: {band:?}"
    );
}
