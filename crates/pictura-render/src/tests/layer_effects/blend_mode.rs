use super::*;

#[test]
fn effect_blend_mode_maps_the_blnm_vocabulary() {
    let cases: [(&[u8], BlendMode); 28] = [
        (b"Nrml", BlendMode::Normal),
        (b"Dslv", BlendMode::Dissolve),
        (b"Drkn", BlendMode::Darken),
        (b"Mltp", BlendMode::Multiply),
        (b"CBrn", BlendMode::ColorBurn),
        (b"Lghn", BlendMode::Lighten),
        (b"Scrn", BlendMode::Screen),
        (b"CDdg", BlendMode::ColorDodge),
        (b"Ovrl", BlendMode::Overlay),
        (b"SftL", BlendMode::SoftLight),
        (b"HrdL", BlendMode::HardLight),
        (b"Dfrn", BlendMode::Difference),
        (b"Xclu", BlendMode::Exclusion),
        (b"H   ", BlendMode::Hue),
        (b"Strt", BlendMode::Saturation),
        (b"Clr ", BlendMode::Color),
        (b"Lmns", BlendMode::Luminosity),
        (b"Sbtr", BlendMode::Subtract),
        (b"vLit", BlendMode::VividLight),
        (b"lLit", BlendMode::LinearLight),
        (b"pLit", BlendMode::PinLight),
        (b"hMix", BlendMode::HardMix),
        (b"lbrn", BlendMode::LinearBurn),
        (b"lddg", BlendMode::LinearDodge),
        (b"fsub", BlendMode::Subtract),
        (b"fdiv", BlendMode::Divide),
        (b"dkCl", BlendMode::DarkerColor),
        (b"lgCl", BlendMode::LighterColor),
    ];
    for (code, want) in cases {
        assert_eq!(
            crate::layer_effects::effect_blend_mode(code, BlendMode::Normal),
            want,
            "{code:?}"
        );
    }
    assert_eq!(
        crate::layer_effects::effect_blend_mode(b"zzzz", BlendMode::Multiply),
        BlendMode::Multiply,
        "an unknown code falls back to the effect default"
    );
    assert_eq!(
        crate::layer_effects::effect_blend_mode(b"", BlendMode::Screen),
        BlendMode::Screen,
        "an empty code (absent key) falls back to the default"
    );
}

fn overlay_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Overlaid",
        rect(3, 3, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    if let Some(block) = block {
        pictura_codec::edit_layer(layer.as_mut(), |parts| parts.extra_blocks = vec![block]);
    }
    layer
}

fn sofi(mode: &[u8]) -> Layer {
    overlay_layer(Some(lfx2_effect(
        b"SoFi",
        object(
            b"SoFi",
            vec![
                (b"enab".to_vec(), DescValue::Bool(true)),
                (b"present".to_vec(), DescValue::Bool(true)),
                (b"Md  ".to_vec(), blenm(mode)),
                (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
                (b"Opct".to_vec(), unit(60.0, PRC)),
            ],
        ),
    )))
}

#[test]
fn a_non_default_blnm_decodes_through_an_effect() {
    // Color Overlay's default is Normal; `Ovrl` must decode to Overlay.
    let overlay = decode_color_overlay(&sofi(b"Ovrl")).expect("decodes");
    assert_eq!(overlay.blend_mode, BlendMode::Overlay);
    // The layer-key spelling is *not* a `BlnM` code: it falls back to Normal.
    let fallback = decode_color_overlay(&sofi(b"mul ")).expect("decodes");
    assert_eq!(fallback.blend_mode, BlendMode::Normal);
    // A wrong typeID still rejects.
    let wrong_kind = overlay_layer(Some(lfx2_effect(
        b"SoFi",
        object(
            b"SoFi",
            vec![
                (b"enab".to_vec(), DescValue::Bool(true)),
                (
                    b"Md  ".to_vec(),
                    DescValue::Enum {
                        kind: b"BlnX".to_vec(),
                        value: b"Ovrl".to_vec(),
                    },
                ),
            ],
        ),
    )));
    assert!(decode_color_overlay(&wrong_kind).is_none());
}
