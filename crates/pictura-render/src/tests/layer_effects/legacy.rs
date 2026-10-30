use super::*;

use crate::layer_effects::{decode_layer_effects, decode_legacy_effects};

// --- lrFX byte builders ----------------------------------------------------

fn push_u16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_be_bytes());
}

fn push_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_be_bytes());
}

fn push_i32(v: &mut Vec<u8>, x: i32) {
    v.extend_from_slice(&x.to_be_bytes());
}

/// A 16-bit `Color`: `u16` space then four `u16` channels.
fn color16(space: u16, rgb: [u8; 3]) -> Vec<u8> {
    let mut v = Vec::new();
    push_u16(&mut v, space);
    for c in rgb {
        push_u16(&mut v, (c as u16) << 8);
    }
    push_u16(&mut v, 0);
    v
}

/// `cmnS` common state.
fn cmns(visible: u8) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, 0);
    v.push(visible);
    v.extend_from_slice(&[0, 0]);
    v
}

#[derive(Clone)]
struct ShadowArgs {
    version: u32,
    blur: u32,
    intensity: u32,
    angle: i32,
    distance: u32,
    color: [u8; 3],
    blend: [u8; 4],
    enabled: u8,
    use_global: u8,
    opacity: u8,
}

impl Default for ShadowArgs {
    fn default() -> Self {
        Self {
            version: 0,
            blur: 6,
            intensity: 10,
            angle: 45,
            distance: 8,
            color: [10, 20, 30],
            blend: *b"mul ",
            enabled: 1,
            use_global: 0,
            opacity: 255,
        }
    }
}

fn shadow_body(a: &ShadowArgs) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, a.version);
    push_u32(&mut v, a.blur);
    push_u32(&mut v, a.intensity);
    push_i32(&mut v, a.angle);
    push_u32(&mut v, a.distance);
    v.extend_from_slice(&color16(0, a.color));
    v.extend_from_slice(b"8BIM");
    v.extend_from_slice(&a.blend);
    v.push(a.enabled);
    v.push(a.use_global);
    v.push(a.opacity);
    if a.version >= 2 {
        v.extend_from_slice(&color16(0, [0, 0, 0]));
    }
    v
}

#[derive(Clone)]
struct GlowArgs {
    version: u32,
    blur: u32,
    intensity: u32,
    color: [u8; 3],
    blend: [u8; 4],
    enabled: u8,
    opacity: u8,
    invert: u8,
}

impl Default for GlowArgs {
    fn default() -> Self {
        Self {
            version: 0,
            blur: 10,
            intensity: 20,
            color: [40, 80, 120],
            blend: *b"scrn",
            enabled: 1,
            opacity: 191,
            invert: 0,
        }
    }
}

fn glow_body(a: &GlowArgs, inner: bool) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, a.version);
    push_u32(&mut v, a.blur);
    push_u32(&mut v, a.intensity);
    v.extend_from_slice(&color16(0, a.color));
    v.extend_from_slice(b"8BIM");
    v.extend_from_slice(&a.blend);
    v.push(a.enabled);
    v.push(a.opacity);
    if a.version >= 2 {
        if inner {
            v.push(a.invert);
        }
        v.extend_from_slice(&color16(0, [0, 0, 0]));
    }
    v
}

#[derive(Clone)]
struct BevelArgs {
    version: u32,
    angle: i32,
    depth: u32,
    blur: u32,
    hl_blend: [u8; 4],
    sh_blend: [u8; 4],
    hl_color: [u8; 3],
    sh_color: [u8; 3],
    style: u8,
    hl_opacity: u8,
    sh_opacity: u8,
    enabled: u8,
    use_global: u8,
    direction: u8,
}

impl Default for BevelArgs {
    fn default() -> Self {
        Self {
            version: 0,
            angle: 120,
            depth: 100,
            blur: 5,
            hl_blend: *b"scrn",
            sh_blend: *b"mul ",
            hl_color: [250, 240, 230],
            sh_color: [10, 20, 30],
            style: 1,
            hl_opacity: 200,
            sh_opacity: 200,
            enabled: 1,
            use_global: 0,
            direction: 0,
        }
    }
}

fn bevel_body(a: &BevelArgs) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, a.version);
    push_i32(&mut v, a.angle);
    push_u32(&mut v, a.depth);
    push_u32(&mut v, a.blur);
    v.extend_from_slice(b"8BIM");
    v.extend_from_slice(&a.hl_blend);
    v.extend_from_slice(b"8BIM");
    v.extend_from_slice(&a.sh_blend);
    v.extend_from_slice(&color16(0, a.hl_color));
    v.extend_from_slice(&color16(0, a.sh_color));
    v.push(a.style);
    v.push(a.hl_opacity);
    v.push(a.sh_opacity);
    v.push(a.enabled);
    v.push(a.use_global);
    v.push(a.direction);
    if a.version == 2 {
        v.extend_from_slice(&color16(0, [0, 0, 0]));
        v.extend_from_slice(&color16(0, [0, 0, 0]));
    }
    v
}

fn sofi_body(version: u32, blend: [u8; 4], color: [u8; 3], opacity: u8, enabled: u8) -> Vec<u8> {
    let mut v = Vec::new();
    push_u32(&mut v, version);
    v.extend_from_slice(b"8BIM");
    v.extend_from_slice(&blend);
    v.extend_from_slice(&color16(0, color));
    v.push(opacity);
    v.push(enabled);
    v.extend_from_slice(&color16(0, [0, 0, 0]));
    v
}

/// An `lrFX` block body from `(ostype, body)` records.
fn lr_fx_data(version: u16, records: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut data = Vec::new();
    push_u16(&mut data, version);
    push_u16(&mut data, records.len() as u16);
    for (ostype, body) in records {
        data.extend_from_slice(b"8BIM");
        data.extend_from_slice(*ostype);
        push_u32(&mut data, body.len() as u32);
        data.extend_from_slice(body);
    }
    data
}

fn lr_fx(records: &[(&[u8; 4], Vec<u8>)]) -> LayerBlock {
    LayerBlock {
        key: *b"lrFX",
        data: lr_fx_data(0, records).into(),
    }
}

fn raw_block(data: Vec<u8>) -> LayerBlock {
    LayerBlock {
        key: *b"lrFX",
        data: data.into(),
    }
}

fn legacy_layer(block: LayerBlock) -> Layer {
    let mut layer = solid(
        "Legacy",
        rect(3, 3, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    layer.extra_blocks = vec![block];
    layer
}

fn compose_legacy(layer: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(9, 9),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(9, 9, vec![backdrop, layer]))
}

fn plain_legacy() -> PixelBuffer {
    compose_legacy(solid(
        "Legacy",
        rect(3, 3, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    ))
}

/// An `lfx2` `DrSh` with the given render-affecting fields.
#[allow(clippy::too_many_arguments)]
fn lfx2_drsh(
    blend: &[u8],
    color: [u8; 3],
    opacity: f64,
    angle: f64,
    distance: f64,
    spread: f64,
    size: f64,
) -> LayerBlock {
    lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), blenm(blend)),
        (
            b"Clr ".to_vec(),
            rgbc(color[0] as f64, color[1] as f64, color[2] as f64),
        ),
        (b"Opct".to_vec(), unit(opacity, PRC)),
        (b"uglg".to_vec(), DescValue::Bool(false)),
        (b"lagl".to_vec(), unit(angle, ANG)),
        (b"Dstn".to_vec(), unit(distance, PXL)),
        (b"Ckmt".to_vec(), unit(spread, PRC)),
        (b"blur".to_vec(), unit(size, PXL)),
        (b"layerConceals".to_vec(), DescValue::Bool(false)),
    ])))
}

// --- Decode ----------------------------------------------------------------

#[test]
fn legacy_drop_shadow_and_outer_glow_decode() {
    let block = lr_fx(&[
        (b"cmnS", cmns(1)),
        (b"dsdw", shadow_body(&ShadowArgs::default())),
        (
            b"oglw",
            glow_body(
                &GlowArgs {
                    opacity: 128,
                    ..Default::default()
                },
                false,
            ),
        ),
    ]);
    let effects = decode_legacy_effects(&legacy_layer(block)).expect("decodes");
    let shadow = effects.drop_shadow.expect("drop shadow");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [10, 20, 30]);
    assert_eq!(shadow.opacity, 100.0);
    assert_eq!(shadow.angle_deg, 45.0);
    assert_eq!(shadow.distance, 8.0);
    assert_eq!(shadow.spread, 10.0);
    assert_eq!(shadow.size, 6.0);
    assert!(!shadow.use_global_angle);
    assert!(!shadow.knocks_out);

    let glow = effects.outer_glow.expect("outer glow");
    assert!(glow.enabled && glow.present);
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [40, 80, 120]);
    assert_eq!(glow.spread, 20.0);
    assert_eq!(glow.size, 10.0);
    assert_eq!(glow.technique, GlowTechnique::Softer);
    assert!((glow.opacity - 128.0 * 100.0 / 255.0).abs() < 1e-4);
}

#[test]
fn legacy_inner_shadow_and_inner_glow_decode() {
    let block = lr_fx(&[
        (b"isdw", shadow_body(&ShadowArgs::default())),
        (
            b"iglw",
            glow_body(
                &GlowArgs {
                    version: 2,
                    invert: 1,
                    ..Default::default()
                },
                true,
            ),
        ),
    ]);
    let effects = decode_legacy_effects(&legacy_layer(block)).expect("decodes");
    let inner = effects.inner_shadow.expect("inner shadow");
    assert_eq!(inner.choke, 10.0);
    assert_eq!(inner.blend_mode, BlendMode::Multiply);
    let glow = effects.inner_glow.expect("inner glow");
    assert_eq!(glow.choke, 20.0);
    assert_eq!(glow.source, GlowSource::Center);
    assert_eq!(glow.technique, GlowTechnique::Softer);

    let edge = lr_fx(&[(
        b"iglw",
        glow_body(
            &GlowArgs {
                version: 2,
                invert: 0,
                ..Default::default()
            },
            true,
        ),
    )]);
    assert_eq!(
        decode_legacy_effects(&legacy_layer(edge))
            .expect("decodes")
            .inner_glow
            .expect("inner glow")
            .source,
        GlowSource::Edge
    );
}

#[test]
fn legacy_bevel_and_solid_fill_decode() {
    let block = lr_fx(&[
        (b"bevl", bevel_body(&BevelArgs::default())),
        (b"sofi", sofi_body(2, *b"mul ", [10, 20, 30], 191, 1)),
    ]);
    let effects = decode_legacy_effects(&legacy_layer(block)).expect("decodes");
    let bevel = effects.bevel.expect("bevel");
    assert!(bevel.enabled && bevel.present);
    assert_eq!(bevel.style, BevelStyle::Inner);
    assert_eq!(bevel.technique, BevelTechnique::Smooth);
    assert_eq!(bevel.direction, BevelDirection::Up);
    assert_eq!(bevel.size, 5.0);
    assert_eq!(bevel.depth, 100.0);
    assert_eq!(bevel.altitude_deg, 30.0);
    assert_eq!(bevel.soften, 0.0);
    assert_eq!(bevel.highlight.mode, BlendMode::Screen);
    assert_eq!(bevel.shadow.mode, BlendMode::Multiply);
    assert_eq!(bevel.highlight.color, [250, 240, 230]);

    let overlay = effects.color_overlay.expect("color overlay");
    assert_eq!(overlay.blend_mode, BlendMode::Multiply);
    assert_eq!(overlay.color, [10, 20, 30]);
    assert!((overlay.opacity - 191.0 * 100.0 / 255.0).abs() < 1e-4);
}

#[test]
fn legacy_common_state_gates_every_effect() {
    let hidden = lr_fx(&[
        (b"cmnS", cmns(0)),
        (b"dsdw", shadow_body(&ShadowArgs::default())),
    ]);
    assert!(
        !decode_legacy_effects(&legacy_layer(hidden))
            .expect("decodes")
            .drop_shadow
            .expect("drop shadow")
            .enabled
    );

    let absent = lr_fx(&[(b"dsdw", shadow_body(&ShadowArgs::default()))]);
    assert!(
        decode_legacy_effects(&legacy_layer(absent))
            .expect("decodes")
            .drop_shadow
            .expect("drop shadow")
            .enabled,
        "an absent cmnS leaves the record's own enabled byte"
    );

    // `cmnS` after the shadow still gates it (order independent).
    let trailing = lr_fx(&[
        (b"dsdw", shadow_body(&ShadowArgs::default())),
        (b"cmnS", cmns(0)),
    ]);
    assert!(
        !decode_legacy_effects(&legacy_layer(trailing))
            .expect("decodes")
            .drop_shadow
            .expect("drop shadow")
            .enabled
    );
}

#[test]
fn legacy_fields_absent_from_the_record_take_defaults() {
    let glow = GlowArgs {
        blend: *b"xxxx",
        version: 0,
        ..Default::default()
    };
    let block = lr_fx(&[(b"oglw", glow_body(&glow, false))]);
    let decoded = decode_legacy_effects(&legacy_layer(block))
        .expect("decodes")
        .outer_glow
        .expect("outer glow");
    assert_eq!(
        decoded.blend_mode,
        BlendMode::Screen,
        "unknown key defaults"
    );
    assert_eq!(decoded.technique, GlowTechnique::Softer);
}

#[test]
fn legacy_out_of_range_values_clamp() {
    let block = lr_fx(&[(
        b"dsdw",
        shadow_body(&ShadowArgs {
            blur: 100_000,
            intensity: 1000,
            distance: 100_000,
            opacity: 255,
            ..Default::default()
        }),
    )]);
    let shadow = decode_legacy_effects(&legacy_layer(block))
        .expect("decodes")
        .drop_shadow
        .expect("drop shadow");
    assert_eq!(shadow.size, 250.0);
    assert_eq!(shadow.spread, 100.0);
    assert_eq!(shadow.distance, 30_000.0);
    assert_eq!(shadow.opacity, 100.0);
}

#[test]
fn legacy_malformed_block_is_none() {
    let bare = solid(
        "Legacy",
        rect(3, 3, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    assert!(decode_legacy_effects(&bare).is_none(), "no lrFX");

    let bad_version = lr_fx_data(1, &[(b"dsdw", shadow_body(&ShadowArgs::default()))]);
    assert!(
        decode_legacy_effects(&legacy_layer(raw_block(bad_version))).is_none(),
        "unsupported EffectsLayer version"
    );

    let mut truncated = Vec::new();
    push_u16(&mut truncated, 0);
    push_u16(&mut truncated, 1);
    truncated.extend_from_slice(b"8BIMdsdw");
    push_u32(&mut truncated, 50);
    assert!(
        decode_legacy_effects(&legacy_layer(raw_block(truncated))).is_none(),
        "truncated body"
    );

    let mut bad_signature = Vec::new();
    push_u16(&mut bad_signature, 0);
    push_u16(&mut bad_signature, 1);
    bad_signature.extend_from_slice(b"XXXXdsdw");
    push_u32(&mut bad_signature, 0);
    assert!(
        decode_legacy_effects(&legacy_layer(raw_block(bad_signature))).is_none(),
        "non-8BIM signature"
    );

    let mut overrun = Vec::new();
    push_u16(&mut overrun, 0);
    push_u16(&mut overrun, 5);
    assert!(
        decode_legacy_effects(&legacy_layer(raw_block(overrun))).is_none(),
        "count overruns the payload"
    );
}

#[test]
fn legacy_malformed_record_is_skipped() {
    let mut bad_color = shadow_body(&ShadowArgs::default());
    bad_color[20..22].copy_from_slice(&1u16.to_be_bytes());
    let block = lr_fx(&[
        (b"xxxx", Vec::new()),
        (b"sofi", sofi_body(1, *b"norm", [0, 0, 0], 255, 1)),
        (b"dsdw", bad_color),
        (b"dsdw", shadow_body(&ShadowArgs::default())),
    ]);
    let effects = decode_legacy_effects(&legacy_layer(block)).expect("decodes");
    assert!(
        effects.drop_shadow.is_some(),
        "the valid dsdw still decodes"
    );
    assert!(
        effects.color_overlay.is_none(),
        "a version-1 sofi is skipped"
    );
}

#[test]
fn legacy_never_panics_on_arbitrary_bytes() {
    for len in 0..48usize {
        let data: Vec<u8> = (0..len).map(|i| (i * 7 + 13) as u8).collect();
        let _ = decode_legacy_effects(&legacy_layer(raw_block(data)));
    }
    // A huge count with a valid first record still terminates.
    let mut data = Vec::new();
    push_u16(&mut data, 0);
    push_u16(&mut data, 0xFFFF);
    data.extend_from_slice(b"8BIMdsdw");
    push_u32(&mut data, 4);
    data.extend_from_slice(&[0, 0, 0, 0]);
    let _ = decode_legacy_effects(&legacy_layer(raw_block(data)));
}

// --- Precedence ------------------------------------------------------------

#[test]
fn lfx2_wins_when_both_blocks_are_present() {
    let lfx2_block = lfx2_drsh(b"Mltp", [1, 2, 3], 50.0, 10.0, 2.0, 0.0, 0.0);
    let lr_block = lr_fx(&[(b"dsdw", shadow_body(&ShadowArgs::default()))]);
    let mut both = legacy_layer(lfx2_block.clone());
    both.extra_blocks.push(lr_block);
    let resolved = decode_layer_effects(&both).drop_shadow.expect("shadow");
    assert_eq!(resolved.color, [1, 2, 3]);
    assert_eq!(resolved.opacity, 50.0);

    let resolved_legacy = decode_legacy_effects(&both)
        .expect("legacy still decodes standalone")
        .drop_shadow
        .expect("legacy shadow");
    assert_eq!(resolved_legacy.color, [10, 20, 30]);

    // The composite ignores the legacy block entirely.
    let mut only_lfx2 = legacy_layer(lfx2_block);
    only_lfx2.extra_blocks.truncate(1);
    assert_eq!(compose_legacy(both), compose_legacy(only_lfx2));
}

#[test]
fn lfx2_omitting_an_effect_suppresses_its_legacy_record() {
    let orgl = object(
        b"OrGl",
        vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
        ],
    );
    let mut layer = legacy_layer(lfx2_effect(b"OrGl", orgl));
    layer
        .extra_blocks
        .push(lr_fx(&[(b"dsdw", shadow_body(&ShadowArgs::default()))]));
    let resolved = decode_layer_effects(&layer);
    assert!(resolved.drop_shadow.is_none());
    assert!(resolved.outer_glow.is_some());
}

#[test]
fn legacy_only_leaves_unmapped_effects_absent() {
    let layer = legacy_layer(lr_fx(&[(b"dsdw", shadow_body(&ShadowArgs::default()))]));
    let resolved = decode_layer_effects(&layer);
    assert!(resolved.satin.is_none());
    assert!(resolved.stroke.is_none());
    assert!(resolved.gradient_overlay.is_none());
    assert!(resolved.pattern_overlay.is_none());
    assert!(resolved.color_overlay.is_none());
    assert!(resolved.drop_shadow.is_some());
}

// --- Render ----------------------------------------------------------------

#[test]
fn legacy_drop_shadow_renders_like_the_equivalent_lfx2() {
    let legacy = legacy_layer(lr_fx(&[
        (b"cmnS", cmns(1)),
        (
            b"dsdw",
            shadow_body(&ShadowArgs {
                blur: 5,
                intensity: 0,
                angle: 120,
                distance: 5,
                color: [10, 20, 30],
                blend: *b"mul ",
                opacity: 255,
                ..Default::default()
            }),
        ),
    ]));
    let modern = legacy_layer(lfx2_drsh(
        b"Mltp",
        [10, 20, 30],
        100.0,
        120.0,
        5.0,
        0.0,
        5.0,
    ));
    assert_eq!(compose_legacy(legacy), compose_legacy(modern));
}

#[test]
fn legacy_outer_glow_renders_around_the_content() {
    let layer = legacy_layer(lr_fx(&[(
        b"oglw",
        glow_body(
            &GlowArgs {
                blur: 2,
                intensity: 0,
                color: [40, 80, 120],
                ..Default::default()
            },
            false,
        ),
    )]));
    let with_effect = compose_legacy(layer);
    let plain = plain_legacy();
    // One pixel left of the content at (3,3)..(6,6): the glow tints it.
    assert_ne!(
        px(&with_effect, 2, 4),
        px(&plain, 2, 4),
        "the glow tints near the content"
    );
    // blur 2 reaches at most (1,1)..(7,7), so the far corner is untouched.
    assert_eq!(
        px(&with_effect, 8, 8),
        px(&plain, 8, 8),
        "a corner beyond the glow's reach is byte-identical to no effect"
    );
}

#[test]
fn legacy_solid_fill_changes_interior_only() {
    let layer = legacy_layer(lr_fx(&[
        (b"cmnS", cmns(1)),
        (b"sofi", sofi_body(2, *b"norm", [0, 255, 0], 255, 1)),
    ]));
    let with_effect = compose_legacy(layer);
    let plain = plain_legacy();
    // Inside the 3x3 content at (3,3)..(6,6) the fill tints.
    assert_ne!(px(&with_effect, 4, 4), px(&plain, 4, 4));
    // Outside, nothing changes.
    for y in 0..9 {
        for x in 0..9 {
            if !(3..6).contains(&x) || !(3..6).contains(&y) {
                assert_eq!(
                    px(&with_effect, x, y),
                    px(&plain, x, y),
                    "({x},{y}) outside the coverage"
                );
            }
        }
    }
}

#[test]
fn legacy_inner_and_outer_bevels_render() {
    let inner = legacy_layer(lr_fx(&[(
        b"bevl",
        bevel_body(&BevelArgs {
            style: 1,
            ..Default::default()
        }),
    )]));
    assert_ne!(compose_legacy(inner), plain_legacy());

    let outer = legacy_layer(lr_fx(&[(
        b"bevl",
        bevel_body(&BevelArgs {
            style: 0,
            ..Default::default()
        }),
    )]));
    assert_ne!(compose_legacy(outer), plain_legacy());
}

#[test]
fn legacy_disabled_and_invisible_are_noops() {
    let disabled = legacy_layer(lr_fx(&[(
        b"dsdw",
        shadow_body(&ShadowArgs {
            enabled: 0,
            ..Default::default()
        }),
    )]));
    assert_eq!(compose_legacy(disabled), plain_legacy());

    let invisible = legacy_layer(lr_fx(&[
        (b"cmnS", cmns(0)),
        (b"dsdw", shadow_body(&ShadowArgs::default())),
    ]));
    assert_eq!(compose_legacy(invisible), plain_legacy());
}

// --- Fixture ---------------------------------------------------------------

const LEGACY_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/legacy_effects.psd");

#[test]
fn legacy_effects_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(LEGACY_FIXTURE).expect("legacy_effects.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Legacy")
        .expect("Legacy layer");
    let effects = decode_legacy_effects(layer).expect("decodes the authored lrFX");
    let shadow = effects.drop_shadow.expect("drop shadow");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [10, 20, 30]);
    assert_eq!(shadow.opacity, 100.0);
    assert_eq!(shadow.angle_deg, 120.0);
    assert_eq!(shadow.distance, 5.0);
    assert_eq!(shadow.spread, 0.0);
    assert_eq!(shadow.size, 5.0);
    assert!(!shadow.use_global_angle);
    let glow = effects.outer_glow.expect("outer glow");
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [40, 80, 120]);
    assert_eq!(glow.spread, 0.0);
    assert_eq!(glow.size, 6.0);
    assert!((glow.opacity - 191.0 * 100.0 / 255.0).abs() < 1e-4);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the effects render");
}

// --- GPU -------------------------------------------------------------------

#[test]
fn gpu_rejects_a_legacy_effect_and_falls_back() {
    let d = doc(
        9,
        9,
        vec![
            solid(
                "Backdrop",
                full(9, 9),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ),
            legacy_layer(lr_fx(&[
                (b"cmnS", cmns(1)),
                (b"dsdw", shadow_body(&ShadowArgs::default())),
            ])),
        ],
    );
    assert_gpu_accepts_and_matches_cpu(&d);
}

#[test]
fn gpu_does_not_reject_disabled_or_unmapped_legacy() {
    let disabled = legacy_layer(lr_fx(&[(
        b"dsdw",
        shadow_body(&ShadowArgs {
            enabled: 0,
            ..Default::default()
        }),
    )]));
    let unmapped = legacy_layer(lr_fx(&[
        (b"sofi", sofi_body(1, *b"norm", [0, 0, 0], 255, 1)),
        (b"xxxx", Vec::new()),
    ]));
    let malformed = legacy_layer(raw_block(vec![0, 0, 0, 1]));
    for layer in [disabled, unmapped, malformed] {
        let d = doc(9, 9, vec![layer]);
        assert_gpu_accepts_and_matches_cpu(&d);
    }
}
