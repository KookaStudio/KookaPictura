use super::*;

fn frfx(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"FrFX".to_vec(),
        items,
    }
}

fn stroke_top_with(stroke: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (b"FrFX".to_vec(), stroke),
        ],
    }
}

fn blend(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"BlnM".to_vec(),
        value: value.to_vec(),
    }
}

fn styl(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"FStl".to_vec(),
        value: value.to_vec(),
    }
}

fn pntt(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"FrFl".to_vec(),
        value: value.to_vec(),
    }
}

/// A stroked 4x4 pixel layer at `(4, 4)` on a 12x12 canvas, leaving a margin on
/// every side so an outside or centred band has room.
fn stroke_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Stroked",
        rect(4, 4, 8, 8),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    if let Some(block) = block {
        layer.extra_blocks = vec![block];
    }
    layer
}

#[derive(Clone)]
struct StrokeSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    size: f64,
    position: Vec<u8>,
    styl_kind: Vec<u8>,
    fill: Vec<u8>,
    fill_kind: Vec<u8>,
}

impl Default for StrokeSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 0.0],
            opacity: 100.0,
            size: 2.0,
            position: b"OutF".to_vec(),
            styl_kind: b"FStl".to_vec(),
            fill: b"SClr".to_vec(),
            fill_kind: b"FrFl".to_vec(),
        }
    }
}

fn spec_block(spec: &StrokeSpec) -> LayerBlock {
    lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
        (b"present".to_vec(), DescValue::Bool(spec.present)),
        (b"Md  ".to_vec(), blend(&spec.blend)),
        (
            b"Clr ".to_vec(),
            rgbc(spec.color[0], spec.color[1], spec.color[2]),
        ),
        (b"Opct".to_vec(), unit(spec.opacity, PRC)),
        (b"Sz  ".to_vec(), unit(spec.size, PXL)),
        (
            b"Styl".to_vec(),
            DescValue::Enum {
                kind: spec.styl_kind.clone(),
                value: spec.position.clone(),
            },
        ),
        (
            b"PntT".to_vec(),
            DescValue::Enum {
                kind: spec.fill_kind.clone(),
                value: spec.fill.clone(),
            },
        ),
    ])))
}

fn spec_stroke(spec: &StrokeSpec) -> Layer {
    stroke_layer(Some(spec_block(spec)))
}

pub(super) fn stroke_layer_at_opacity(opacity: u8) -> Layer {
    let mut layer = spec_stroke(&StrokeSpec::default());
    layer.opacity = opacity;
    layer
}

fn compose_stroke(stroked: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, stroked]))
}

fn plain_stroke() -> PixelBuffer {
    compose_stroke(stroke_layer(None))
}

/// The content occupies `(4, 4)-(8, 8)`.
fn content() -> (u32, u32, u32, u32) {
    (4, 4, 8, 8)
}

/// Count pixels in the strip left of the content (`x < 4`) that differ from the
/// white backdrop.
fn left_strip_changed(buf: &PixelBuffer) -> u32 {
    let mut n = 0;
    for y in 0..buf.height {
        for x in 0..4 {
            if rgb(buf, x, y) != [255, 255, 255] {
                n += 1;
            }
        }
    }
    n
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn stroke_decodes_typed_parameters() {
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), blend(b"Mltp")),
        (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
        (b"Opct".to_vec(), unit(60.0, PRC)),
        (b"Sz  ".to_vec(), unit(12.0, PXL)),
        (b"Styl".to_vec(), styl(b"InsF")),
        (b"PntT".to_vec(), pntt(b"SClr")),
    ])));
    let stroke = decode_stroke(&stroke_layer(Some(block))).expect("decodes");
    assert!(stroke.enabled && stroke.present);
    assert_eq!(stroke.blend_mode, BlendMode::Multiply);
    assert_eq!(stroke.fill, StrokeFill::Solid([200, 100, 50]));
    assert_eq!(stroke.opacity, 60.0);
    assert_eq!(stroke.size, 12);
    assert_eq!(stroke.position, StrokePosition::Inside);
}

#[test]
fn stroke_missing_keys_take_defaults() {
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
    ])));
    let stroke = decode_stroke(&stroke_layer(Some(block))).expect("decodes");
    assert_eq!(stroke.blend_mode, BlendMode::Normal);
    assert_eq!(stroke.fill, StrokeFill::Solid([0, 0, 0]));
    assert_eq!(stroke.opacity, 100.0);
    assert_eq!(stroke.size, 3);
    assert_eq!(stroke.position, StrokePosition::Outside);
}

#[test]
fn stroke_position_values_decode() {
    for (value, expected) in [
        (b"OutF", StrokePosition::Outside),
        (b"InsF", StrokePosition::Inside),
        (b"CtrF", StrokePosition::Center),
    ] {
        let block = lfx2(stroke_top_with(frfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Styl".to_vec(), styl(value)),
        ])));
        assert_eq!(
            decode_stroke(&stroke_layer(Some(block)))
                .expect("decodes")
                .position,
            expected,
            "FStl/{value:?}"
        );
    }
    // An unknown position value falls back to Outside.
    let unknown = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Styl".to_vec(), styl(b"zzzz")),
    ])));
    assert_eq!(
        decode_stroke(&stroke_layer(Some(unknown)))
            .expect("decodes")
            .position,
        StrokePosition::Outside
    );
    // A wrong `Styl` typeID rejects the effect.
    let wrong_styl = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Styl".to_vec(),
            DescValue::Enum {
                kind: b"XXXX".to_vec(),
                value: b"OutF".to_vec(),
            },
        ),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(wrong_styl))).is_none());
    // A wrong `PntT` typeID rejects the effect.
    let wrong_pntt = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"PntT".to_vec(),
            DescValue::Enum {
                kind: b"XXXX".to_vec(),
                value: b"SClr".to_vec(),
            },
        ),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(wrong_pntt))).is_none());
}

#[test]
fn stroke_non_solid_without_content_is_a_no_op() {
    // A `GrFl`/`Ptrn` fill with no matching content object is malformed, and an
    // unknown fill type is rejected; all three are byte-identical to no effect.
    for fill in [b"GrFl".as_slice(), b"Ptrn".as_slice(), b"zzzz".as_slice()] {
        let spec = StrokeSpec {
            fill: fill.to_vec(),
            ..Default::default()
        };
        assert!(
            decode_stroke(&spec_stroke(&spec)).is_none(),
            "PntT {fill:?} without its content object is a no-op"
        );
        assert_eq!(
            compose_stroke(spec_stroke(&spec)),
            plain_stroke(),
            "a malformed fill is byte-identical to no effect"
        );
    }
}

fn stop(location: f64, color: (f64, f64, f64)) -> DescValue {
    object(
        b"Clrt",
        vec![
            (b"Clr ".to_vec(), rgbc(color.0, color.1, color.2)),
            (b"Lctn".to_vec(), DescValue::Double(location)),
        ],
    )
}

fn gradn(stops: Vec<DescValue>) -> DescValue {
    object(
        b"Grdn",
        vec![
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (b"Clrs".to_vec(), DescValue::List(stops)),
        ],
    )
}

fn gradient_stroke_block(align: bool, enabled: bool) -> LayerBlock {
    lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(enabled)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (
            b"Grad".to_vec(),
            gradn(vec![
                stop(0.0, (0.0, 0.0, 0.0)),
                stop(4096.0, (255.0, 255.0, 255.0)),
            ]),
        ),
        (b"Angl".to_vec(), unit(45.0, ANG)),
        (
            b"Type".to_vec(),
            DescValue::Enum {
                kind: b"GrdT".to_vec(),
                value: b"Lnr ".to_vec(),
            },
        ),
        (b"Rvrs".to_vec(), DescValue::Bool(false)),
        (b"Scl ".to_vec(), unit(100.0, PRC)),
        (b"Algn".to_vec(), DescValue::Bool(align)),
        (b"Sz  ".to_vec(), unit(2.0, PXL)),
    ])))
}

fn ptrn(content: &str) -> DescValue {
    object(
        b"Ptrn",
        vec![(b"Idnt".to_vec(), DescValue::Text(format!("{content}\0")))],
    )
}

fn pattern_stroke_block(id: &str, link: bool) -> LayerBlock {
    lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), ptrn(id)),
        (b"Scl ".to_vec(), unit(100.0, PRC)),
        (b"Lnkd".to_vec(), DescValue::Bool(link)),
        (b"Angl".to_vec(), unit(0.0, ANG)),
        (b"Sz  ".to_vec(), unit(2.0, PXL)),
    ])))
}

#[test]
fn gradient_fill_decodes_to_gradient() {
    let stroke =
        decode_stroke(&stroke_layer(Some(gradient_stroke_block(false, true)))).expect("decodes");
    match stroke.fill {
        StrokeFill::Gradient {
            params,
            align_with_layer,
        } => {
            assert_eq!(
                params.stops,
                vec![
                    GradientStop {
                        location: 0,
                        color: [0, 0, 0]
                    },
                    GradientStop {
                        location: 4096,
                        color: [255, 255, 255]
                    },
                ]
            );
            assert!(!params.reverse);
            assert_eq!(params.kind, GradientKind::Linear);
            assert_eq!(params.angle_deg, 45.0);
            assert_eq!(params.scale, 100.0);
            assert!(!align_with_layer);
        }
        other => panic!("expected a gradient fill, got {other:?}"),
    }
}

#[test]
fn absent_gradient_angl_and_type_take_defaults() {
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (
            b"Grad".to_vec(),
            gradn(vec![
                stop(0.0, (0.0, 0.0, 0.0)),
                stop(4096.0, (255.0, 255.0, 255.0)),
            ]),
        ),
    ])));
    let stroke = decode_stroke(&stroke_layer(Some(block))).expect("decodes");
    match stroke.fill {
        StrokeFill::Gradient {
            params,
            align_with_layer,
        } => {
            assert_eq!(params.angle_deg, 0.0, "absent Angl defaults to 0");
            assert_eq!(
                params.kind,
                GradientKind::Linear,
                "absent Type defaults Linear"
            );
            assert!(align_with_layer, "absent Algn defaults true");
        }
        other => panic!("expected a gradient fill, got {other:?}"),
    }
}

#[test]
fn pattern_fill_decodes_to_pattern() {
    let stroke = decode_stroke(&stroke_layer(Some(pattern_stroke_block(
        "pictura-pattern",
        false,
    ))))
    .expect("decodes");
    match stroke.fill {
        StrokeFill::Pattern { params, angle_deg } => {
            assert_eq!(params.pattern_id, "pictura-pattern");
            assert_eq!(params.scale, 100.0);
            assert!(
                !params.link_with_layer,
                "Lnkd false overrides the helper default"
            );
            assert_eq!(angle_deg, 0.0);
        }
        other => panic!("expected a pattern fill, got {other:?}"),
    }
}

#[test]
fn pattern_link_flag_prefers_lnkd_and_defaults_true() {
    // `Lnkd` absent and `Algn` absent: the helper default true holds.
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
    ])));
    let link = |block: LayerBlock| match decode_stroke(&stroke_layer(Some(block)))
        .expect("decodes")
        .fill
    {
        StrokeFill::Pattern { params, .. } => params.link_with_layer,
        other => panic!("expected a pattern fill, got {other:?}"),
    };
    assert!(link(block), "absent Lnkd/Algn defaults true");
    // `Lnkd` present overrides a contradictory `Algn`.
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
        (b"Algn".to_vec(), DescValue::Bool(true)),
        (b"Lnkd".to_vec(), DescValue::Bool(false)),
    ])));
    assert!(!link(block), "Lnkd overrides Algn");
}

#[test]
fn valid_lnkd_survives_a_wrongly_typed_algn() {
    // `Lnkd` is authoritative: a present-but-malformed `Algn` must not reject
    // the stroke when `Lnkd` is valid.
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
        (b"Algn".to_vec(), DescValue::Double(1.0)),
        (b"Lnkd".to_vec(), DescValue::Bool(false)),
    ])));
    match decode_stroke(&stroke_layer(Some(block)))
        .expect("a valid Lnkd decodes despite the malformed Algn")
        .fill
    {
        StrokeFill::Pattern { params, .. } => {
            assert!(!params.link_with_layer, "the valid Lnkd value is used")
        }
        other => panic!("expected a pattern fill, got {other:?}"),
    }
    // Without `Lnkd`, the malformed `Algn` still rejects.
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
        (b"Algn".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(block))).is_none());
}

#[test]
fn malformed_gradient_or_pattern_content_is_none() {
    let base = |items: Vec<(Vec<u8>, DescValue)>| {
        let mut all = vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
        ];
        all.extend(items);
        lfx2(stroke_top_with(frfx(all)))
    };
    // GrFl without Grad.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![(
        b"PntT".to_vec(),
        pntt(b"GrFl")
    )]))))
    .is_none());
    // Grad that is not an object.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (b"Grad".to_vec(), DescValue::Double(1.0)),
    ]))))
    .is_none());
    // GrdF that is not CstS.
    let not_csts = object(
        b"Grdn",
        vec![
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"FStS".to_vec(),
                },
            ),
            (
                b"Clrs".to_vec(),
                DescValue::List(vec![
                    stop(0.0, (0.0, 0.0, 0.0)),
                    stop(4096.0, (1.0, 1.0, 1.0)),
                ]),
            ),
        ],
    );
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (b"Grad".to_vec(), not_csts),
    ]))))
    .is_none());
    // A single stop.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (b"Grad".to_vec(), gradn(vec![stop(0.0, (0.0, 0.0, 0.0))])),
    ]))))
    .is_none());
    // Non-increasing stops.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (
            b"Grad".to_vec(),
            gradn(vec![
                stop(4096.0, (0.0, 0.0, 0.0)),
                stop(0.0, (1.0, 1.0, 1.0))
            ]),
        ),
    ]))))
    .is_none());
    // Ptrn without Ptrn.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![(
        b"PntT".to_vec(),
        pntt(b"Ptrn")
    )]))))
    .is_none());
    // Ptrn that is not an object.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), DescValue::Double(1.0)),
    ]))))
    .is_none());
    // A missing Idnt.
    assert!(decode_stroke(&stroke_layer(Some(base(vec![
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), object(b"Ptrn", vec![])),
    ]))))
    .is_none());
    // Non-finite Angl rejects a gradient and a pattern.
    for fill in [b"GrFl".as_slice(), b"Ptrn".as_slice()] {
        let mut items = vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"PntT".to_vec(), pntt(fill)),
            (b"Angl".to_vec(), DescValue::Double(f64::NAN)),
        ];
        match fill {
            b"GrFl" => items.push((
                b"Grad".to_vec(),
                gradn(vec![
                    stop(0.0, (0.0, 0.0, 0.0)),
                    stop(4096.0, (1.0, 1.0, 1.0)),
                ]),
            )),
            _ => items.push((b"Ptrn".to_vec(), ptrn("pictura-pattern"))),
        }
        assert!(
            decode_stroke(&stroke_layer(Some(lfx2(stroke_top_with(frfx(items)))))).is_none(),
            "a non-finite Angl rejects {fill:?}"
        );
    }
    // A non-finite `Scl ` rejects a gradient and a pattern too.
    for fill in [b"GrFl".as_slice(), b"Ptrn".as_slice()] {
        let mut items = vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"PntT".to_vec(), pntt(fill)),
            (b"Scl ".to_vec(), DescValue::Double(f64::INFINITY)),
        ];
        match fill {
            b"GrFl" => items.push((
                b"Grad".to_vec(),
                gradn(vec![
                    stop(0.0, (0.0, 0.0, 0.0)),
                    stop(4096.0, (1.0, 1.0, 1.0)),
                ]),
            )),
            _ => items.push((b"Ptrn".to_vec(), ptrn("pictura-pattern"))),
        }
        assert!(
            decode_stroke(&stroke_layer(Some(lfx2(stroke_top_with(frfx(items)))))).is_none(),
            "a non-finite Scl rejects {fill:?}"
        );
    }
}

#[test]
fn stroke_doub_numeric_decodes() {
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Opct".to_vec(), DescValue::Double(40.0)),
        (b"Sz  ".to_vec(), DescValue::Double(7.0)),
    ])));
    let stroke = decode_stroke(&stroke_layer(Some(block))).expect("decodes");
    assert_eq!(stroke.opacity, 40.0);
    assert_eq!(stroke.size, 7);
}

#[test]
fn malformed_or_absent_stroke_is_none() {
    // No lfx2 block.
    assert!(decode_stroke(&stroke_layer(None)).is_none());
    // lfx2 without an FrFX.
    let no_frfx = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_stroke(&stroke_layer(Some(no_frfx))).is_none());
    // An object named FrFX whose class is not FrFX.
    let wrong_class = lfx2(stroke_top_with(DescValue::Object {
        name: String::new(),
        class_id: b"DrSh".to_vec(),
        items: vec![(b"enab".to_vec(), DescValue::Bool(true))],
    }));
    assert!(decode_stroke(&stroke_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        stroke_top_with(frfx(vec![(b"enab".to_vec(), DescValue::Bool(true))])),
        17,
    );
    assert!(decode_stroke(&stroke_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnX".to_vec(),
                value: b"Mltp".to_vec(),
            },
        ),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(wrong_kind))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Clr ".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"LbCl".to_vec(),
                items: vec![
                    (b"Rd  ".to_vec(), DescValue::Double(1.0)),
                    (b"Grn ".to_vec(), DescValue::Double(2.0)),
                    (b"Bl  ".to_vec(), DescValue::Double(3.0)),
                ],
            },
        ),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(wrong_color))).is_none());
    // Non-finite size.
    let nan = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Sz  ".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert!(decode_stroke(&stroke_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2(stroke_top_with(frfx(vec![])));
    short.data.truncate(6);
    assert!(decode_stroke(&stroke_layer(Some(short))).is_none());
}

#[test]
fn huge_numeric_stroke_is_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Sz  ", b"Opct"] {
        let block = lfx2(stroke_top_with(frfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), DescValue::Double(1e300)),
        ])));
        assert!(
            decode_stroke(&stroke_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Sz  ".to_vec(), unit(1e30, PXL)),
        (b"Opct".to_vec(), unit(1e30, PRC)),
    ])));
    let stroke = decode_stroke(&stroke_layer(Some(block))).expect("decodes clamped");
    assert_eq!(stroke.size, 250);
    assert_eq!(stroke.opacity, 100.0);
    // Rendering the clamped effect does not panic and stays bounded.
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        size: 1e30,
        opacity: 1e30,
        ..Default::default()
    }));
    assert_eq!(out.width, 12);
}

#[test]
fn stroke_disabled_decodes_but_is_inert() {
    for spec in [
        StrokeSpec {
            enabled: false,
            ..Default::default()
        },
        StrokeSpec {
            present: false,
            ..Default::default()
        },
    ] {
        let decoded = decode_stroke(&spec_stroke(&spec)).expect("decodes");
        assert!(
            !decoded.enabled || !decoded.present,
            "the inert flag is what makes the composite a no-op"
        );
        assert_eq!(
            compose_stroke(spec_stroke(&spec)),
            plain_stroke(),
            "an inert effect is byte-identical to no effect"
        );
    }
}

// --- Rendering -------------------------------------------------------------

#[test]
fn outside_band_lies_outside_and_leaves_the_interior_unchanged() {
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        size: 2.0,
        position: b"OutF".to_vec(),
        ..Default::default()
    }));
    let plain = plain_stroke();
    assert_eq!(rgb(&out, 2, 5), [0, 0, 0], "band 2px outside the left edge");
    assert_eq!(rgb(&out, 3, 5), [0, 0, 0], "band 1px outside the left edge");
    let (x0, y0, x1, y1) = content();
    for y in y0..y1 {
        for x in x0..x1 {
            assert_eq!(px(&out, x, y), px(&plain, x, y), "interior ({x},{y})");
        }
    }
    assert_eq!(rgb(&out, 1, 5), [255, 255, 255], "beyond the 2px reach");
}

#[test]
fn inside_band_is_confined_to_the_interior() {
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        size: 2.0,
        position: b"InsF".to_vec(),
        ..Default::default()
    }));
    let plain = plain_stroke();
    assert_eq!(rgb(&out, 4, 5), [0, 0, 0], "band inside the left edge");
    // Every pixel outside the content coverage is byte-identical.
    let (x0, y0, x1, y1) = content();
    for y in 0..12u32 {
        for x in 0..12u32 {
            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                continue;
            }
            assert_eq!(px(&out, x, y), px(&plain, x, y), "exterior ({x},{y})");
        }
    }
}

#[test]
fn center_band_straddles_the_edge() {
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        size: 3.0,
        position: b"CtrF".to_vec(),
        ..Default::default()
    }));
    let plain = plain_stroke();
    assert_eq!(rgb(&out, 3, 5), [0, 0, 0], "centred band outside the edge");
    assert_eq!(rgb(&out, 4, 5), [0, 0, 0], "centred band inside the edge");
    // The interior core is untinted.
    assert_eq!(px(&out, 5, 5), px(&plain, 5, 5), "interior core unchanged");
}

#[test]
fn size_changes_the_band_width() {
    let count = |size: f64| {
        left_strip_changed(&compose_stroke(spec_stroke(&StrokeSpec {
            size,
            position: b"OutF".to_vec(),
            ..Default::default()
        })))
    };
    let small = count(2.0);
    let large = count(6.0);
    assert!(small > 0, "the size 2 band reaches the strip");
    assert!(
        large >= small,
        "size 6 covers at least as many pixels as size 2 ({large} vs {small})"
    );
}

#[test]
fn opacity_and_colour_shape_the_stroke() {
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 50.0,
        ..Default::default()
    }));
    let p = rgb(&out, 3, 5);
    assert_eq!(p[0], 255, "red tint keeps red");
    assert!((80..=180).contains(&p[1]), "green attenuated to {}", p[1]);
    assert!(p[0] > p[1] + 50, "red tint, not black: {p:?}");
    // Lowering opacity approaches the unstroked backdrop.
    let faint = compose_stroke(spec_stroke(&StrokeSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 10.0,
        ..Default::default()
    }));
    assert!(
        rgb(&faint, 3, 5)[1] > rgb(&out, 3, 5)[1],
        "opacity 10 is closer to white than opacity 50"
    );
}

#[test]
fn blend_mode_shapes_the_stroke() {
    let normal = compose_stroke(spec_stroke(&StrokeSpec {
        blend: b"Nrml".to_vec(),
        ..Default::default()
    }));
    let screen = compose_stroke(spec_stroke(&StrokeSpec {
        blend: b"Scrn".to_vec(),
        ..Default::default()
    }));
    assert_ne!(normal, screen, "Normal and Screen differ");
    assert_eq!(
        rgb(&normal, 3, 5),
        [0, 0, 0],
        "a black Normal band is opaque"
    );
    assert_eq!(
        rgb(&screen, 3, 5),
        [255, 255, 255],
        "a black Screen band leaves the white backdrop"
    );
}

#[test]
fn layer_mask_shapes_the_matte() {
    let spec = StrokeSpec::default();
    let full = with_mask(spec_stroke(&spec), 255);
    let empty = with_mask(spec_stroke(&spec), 0);
    assert!(
        left_strip_changed(&compose_stroke(full)) > 0,
        "mask 255 keeps the band"
    );
    assert_eq!(
        left_strip_changed(&compose_stroke(empty)),
        0,
        "mask 0 hides the band"
    );
}

#[test]
fn stroke_is_bounded_to_the_content_plus_reach() {
    let out = compose_stroke(spec_stroke(&StrokeSpec {
        size: 4.0,
        position: b"OutF".to_vec(),
        ..Default::default()
    }));
    // Content `(4, 4)-(8, 8)` plus reach 4 is `(0, 0)-(12, 12)` on a 12x12
    // canvas, so the bound is the whole canvas; assert the reach boundary
    // instead on a larger canvas.
    let big = |stroked: Layer| {
        let backdrop = solid(
            "Backdrop",
            full(32, 32),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        composite_rgba(&doc(32, 32, vec![backdrop, stroked]))
    };
    let with = big(spec_stroke(&StrokeSpec {
        size: 3.0,
        position: b"OutF".to_vec(),
        ..Default::default()
    }));
    let plain_big = big(stroke_layer(None));
    // Content `(4, 4)-(8, 8)` padded by 3 is `(1, 1)-(11, 11)`; only that rect
    // may differ.
    for y in 0..32u32 {
        for x in 0..32u32 {
            if (1..11).contains(&x) && (1..11).contains(&y) {
                continue;
            }
            assert_eq!(px(&with, x, y), px(&plain_big, x, y), "far ({x},{y})");
        }
    }
    // The small-canvas case is bounded and does not panic.
    assert_eq!(out.width, 12);
}

#[test]
fn zero_size_or_non_finite_stroke_is_a_bounded_no_op() {
    // A non-finite size rejects the effect, so the composite is a no-op.
    let nan = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Sz  ".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert_eq!(compose_stroke(stroke_layer(Some(nan))), plain_stroke());
    // A zero size decodes to the clamped minimum 1 and renders a 1px band.
    let zero = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Sz  ".to_vec(), unit(0.0, PXL)),
    ])));
    assert_eq!(
        decode_stroke(&stroke_layer(Some(zero.clone())))
            .expect("decodes")
            .size,
        1
    );
    let out = compose_stroke(stroke_layer(Some(zero)));
    assert_eq!(out.width, 12, "the clamped minimum renders without panic");
}

#[test]
fn off_canvas_or_zero_area_stroke_is_a_no_op() {
    let plain = plain_stroke();
    // Opacity 0 writes nothing.
    assert_eq!(
        plain,
        compose_stroke(spec_stroke(&StrokeSpec {
            opacity: 0.0,
            ..Default::default()
        })),
        "opacity 0 is a no-op"
    );
    // A fully off-canvas rect is a no-op.
    let mut off = stroke_layer(None);
    off.rect = rect(-50, -50, -40, -40);
    let mut off_effect = off.clone();
    off_effect.extra_blocks = vec![spec_block(&StrokeSpec {
        size: 250.0,
        ..Default::default()
    })];
    assert_eq!(
        compose_stroke(off_effect),
        compose_stroke(off),
        "off-canvas is a no-op"
    );
    // A zero-area rect is a no-op.
    let mut empty = stroke_layer(None);
    empty.rect = rect(4, 4, 4, 4);
    let mut empty_effect = empty.clone();
    empty_effect.extra_blocks = vec![spec_block(&StrokeSpec::default())];
    assert_eq!(
        compose_stroke(empty_effect),
        compose_stroke(empty),
        "zero area is a no-op"
    );
}

#[test]
fn max_size_small_layer_is_bounded() {
    // A maximum `size` is confined to the content bbox + reach on a large canvas.
    let start = std::time::Instant::now();
    let backdrop = solid(
        "Backdrop",
        full(512, 512),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let out = composite_rgba(&doc(
        512,
        512,
        vec![
            backdrop,
            spec_stroke(&StrokeSpec {
                size: 250.0,
                ..Default::default()
            }),
        ],
    ));
    let elapsed = start.elapsed();
    assert_eq!(out.width, 512);
    assert!(
        elapsed.as_secs_f64() < 15.0,
        "a small layer at max size is bounded ({elapsed:?})"
    );
}

// --- Gradient and pattern fill sources -------------------------------------

/// A stroked 3x3 red layer at `(3, 3)` over a white backdrop on the 8x8
/// pattern-fixture document (which carries the 2x2 `pictura-pattern`).
fn pattern_stroke_doc(id: &str, link: bool) -> Document {
    let mut layer = solid(
        "Stroked",
        rect(3, 3, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    layer.extra_blocks = vec![pattern_stroke_block(id, link)];
    let mut d = pattern_fixture_doc();
    let backdrop = solid(
        "Backdrop",
        full(8, 8),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    d.layers = pictura_core::LayerStack::from_layers(vec![backdrop, layer]);
    d
}

#[test]
fn gradient_stroke_paints_the_band_and_leaves_the_interior() {
    let out = compose_stroke(stroke_layer(Some(gradient_stroke_block(true, true))));
    let plain = plain_stroke();
    let (x0, y0, x1, y1) = content();
    for y in y0..y1 {
        for x in x0..x1 {
            assert_eq!(px(&out, x, y), px(&plain, x, y), "interior ({x},{y})");
        }
    }
    // The ring outside the content varies along the gradient direction.
    let mut colors = std::collections::BTreeSet::new();
    for y in 2..10u32 {
        for x in 2..10u32 {
            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                continue;
            }
            let c = rgb(&out, x, y);
            if c != [255, 255, 255] {
                colors.insert(c);
            }
        }
    }
    assert!(
        colors.len() >= 2,
        "the gradient band varies, not one flat tint: {colors:?}"
    );
    // Beyond the content padded by 2 on a 12x12 canvas is byte-identical.
    assert_eq!(rgb(&out, 0, 0), [255, 255, 255]);
    assert_eq!(px(&out, 0, 0), px(&plain, 0, 0));
}

#[test]
fn gradient_align_flag_moves_the_origin() {
    let aligned = compose_stroke(stroke_layer(Some(gradient_stroke_block(true, true))));
    let unaligned = compose_stroke(stroke_layer(Some(gradient_stroke_block(false, true))));
    assert_ne!(
        aligned, unaligned,
        "the align flag moves the gradient origin"
    );
}

#[test]
fn pattern_stroke_tiles_the_band_and_shows_the_authored_tile() {
    let out = composite_rgba(&pattern_stroke_doc("pictura-pattern", false));
    let mut band = std::collections::BTreeSet::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            band.insert(rgb(&out, x, y));
        }
    }
    // The resolved 2x2 tile contributes colours the red content cannot. White is
    // the backdrop, so it is not evidence of the tile.
    assert!(
        band.contains(&[0, 255, 0]) || band.contains(&[0, 0, 255]),
        "the authored pattern resolves in the band: {band:?}"
    );
    // An unknown id falls back to the shipped grey placeholder, not a no-op.
    let grey = composite_rgba(&pattern_stroke_doc("missing-pattern", false));
    assert!(
        (0..8u32).any(|y| (0..8u32).any(|x| rgb(&grey, x, y) == [128, 128, 128])),
        "an unknown pattern id renders the grey placeholder"
    );
}

#[test]
fn pattern_link_flag_moves_the_anchor() {
    let unlinked = composite_rgba(&pattern_stroke_doc("pictura-pattern", false));
    let linked = composite_rgba(&pattern_stroke_doc("pictura-pattern", true));
    assert_ne!(unlinked, linked, "the pattern anchor follows the link flag");
}

#[test]
fn unreadable_fill_is_a_bounded_no_op() {
    // GrFl without Grad renders nothing rather than panicking.
    let bad_gradient = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"GrFl")),
        (b"Sz  ".to_vec(), unit(2.0, PXL)),
    ])));
    assert_eq!(
        compose_stroke(stroke_layer(Some(bad_gradient))),
        plain_stroke()
    );
    // Ptrn with a non-object payload renders nothing rather than panicking.
    let bad_pattern = lfx2(stroke_top_with(frfx(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"PntT".to_vec(), pntt(b"Ptrn")),
        (b"Ptrn".to_vec(), DescValue::Double(1.0)),
        (b"Sz  ".to_vec(), unit(2.0, PXL)),
    ])));
    assert_eq!(
        compose_stroke(stroke_layer(Some(bad_pattern))),
        plain_stroke()
    );
}

// --- Matte sources ---------------------------------------------------------

#[test]
fn solid_fill_stroke_follows_payload_alpha() {
    let fill = |alpha: u8| {
        let mut layer = Layer {
            name: "Solid".into(),
            rect: rect(4, 4, 8, 8),
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![255, 0, 0, alpha],
            }),
            ..Default::default()
        };
        layer.extra_blocks = vec![spec_block(&StrokeSpec::default())];
        compose_stroke(layer)
    };
    assert_eq!(
        rgb(&fill(0), 3, 5),
        [255, 255, 255],
        "a transparent fill strokes nothing"
    );
    assert_ne!(fill(255), fill(0), "an opaque fill casts an outside band");
}

#[test]
fn pattern_fill_stroke_is_confined_to_the_fill() {
    let params = PatternFillParams {
        pattern_id: "pictura-pattern".into(),
        scale: 100.0,
        link_with_layer: true,
        origin: (0, 0),
    };
    let build = |effect: bool| {
        let mut layer = Layer {
            name: "Pattern".into(),
            rect: rect(3, 3, 6, 6),
            adjustment: Some(ptfl(&params)),
            ..Default::default()
        };
        if effect {
            layer.extra_blocks = vec![spec_block(&StrokeSpec {
                size: 2.0,
                ..Default::default()
            })];
        }
        let mut d = pattern_fixture_doc();
        d.layers = pictura_core::LayerStack::from_layers(vec![
            solid(
                "Backdrop",
                full(8, 8),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ]);
        composite_rgba(&d)
    };
    let with = build(true);
    let plain = build(false);
    let mut changed = false;
    for y in 0..8 {
        for x in 0..8 {
            if px(&with, x, y) != px(&plain, x, y) {
                assert!(
                    (1..8).contains(&x) && (1..8).contains(&y),
                    "stroke outside the padded fill at ({x},{y})"
                );
                changed = true;
            }
        }
    }
    assert!(changed, "the stroke renders at the fill edge");
}

#[test]
fn channel_less_smart_object_stroke_covers_the_rect() {
    let mut layer = Layer {
        name: "Smart".into(),
        rect: rect(4, 4, 8, 8),
        channels: Vec::new(),
        smart_object: Some(pictura_core::SmartObject {
            kind: pictura_core::SmartObjectKind::Embedded,
            payload: Some(vec![0; 4]),
            ..Default::default()
        }),
        ..Default::default()
    };
    layer.extra_blocks = vec![spec_block(&StrokeSpec::default())];
    let out = compose_stroke(layer);
    assert_eq!(
        rgb(&out, 3, 5),
        [0, 0, 0],
        "the layer rect casts an outside band"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_stroke_layer_and_falls_back() {
    let d = doc(
        12,
        12,
        vec![
            solid(
                "Backdrop",
                full(12, 12),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            spec_stroke(&StrokeSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_rejects_a_gradient_or_pattern_stroke_and_falls_back() {
    let gradient = doc(
        12,
        12,
        vec![stroke_layer(Some(gradient_stroke_block(true, true)))],
    );
    assert!(
        matches!(
            composite_gpu(&gradient),
            Err(GpuError::UnsupportedLayerEffect)
        ),
        "a gradient stroke is effect-bearing"
    );
    assert_eq!(composite_gpu_or_cpu(&gradient), composite_rgba(&gradient));

    let pattern = pattern_stroke_doc("pictura-pattern", true);
    assert!(
        matches!(
            composite_gpu(&pattern),
            Err(GpuError::UnsupportedLayerEffect)
        ),
        "a pattern stroke is effect-bearing"
    );
    assert_eq!(composite_gpu_or_cpu(&pattern), composite_rgba(&pattern));
}

#[test]
fn gpu_does_not_reject_an_inert_or_malformed_stroke() {
    for spec in [
        StrokeSpec {
            enabled: false,
            ..Default::default()
        },
        StrokeSpec {
            present: false,
            ..Default::default()
        },
        StrokeSpec {
            // `GrFl` with no `Grad` payload does not decode, so it is not an effect.
            fill: b"GrFl".to_vec(),
            ..Default::default()
        },
    ] {
        let d = doc(12, 12, vec![spec_stroke(&spec)]);
        assert!(
            !matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
            "an inert or malformed stroke must not reject the GPU"
        );
    }
    // A disabled gradient stroke does not reject either.
    let disabled = doc(
        12,
        12,
        vec![stroke_layer(Some(gradient_stroke_block(true, false)))],
    );
    assert!(
        !matches!(
            composite_gpu(&disabled),
            Err(GpuError::UnsupportedLayerEffect)
        ),
        "a disabled gradient stroke must not reject the GPU"
    );
}

// --- Fixture ---------------------------------------------------------------

const STROKE_FIXTURE: &[u8] = include_bytes!("../../../../pictura-codec/tests/fixtures/stroke.psd");

#[test]
fn stroke_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(STROKE_FIXTURE).expect("stroke.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Stroked")
        .expect("Stroked layer");
    let stroke = decode_stroke(layer).expect("decodes the authored FrFX");
    assert!(stroke.enabled && stroke.present);
    assert_eq!(stroke.blend_mode, BlendMode::Normal);
    assert_eq!(stroke.fill, StrokeFill::Solid([0, 0, 0]));
    assert_eq!(stroke.opacity, 100.0);
    assert_eq!(stroke.size, 3);
    assert_eq!(stroke.position, StrokePosition::Outside);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the stroke renders");
}
