use std::fmt::Debug;
use std::mem::size_of;

use super::{copy_payload, parse_cube, read_descriptor};
use crate::budget::DecodeBudget;
use crate::{DescValue, ImageBudget, PsdError};

fn budget(per_allocation: u64, total: u64) -> DecodeBudget {
    DecodeBudget::new(ImageBudget {
        max_dimension: 16,
        max_alloc_bytes: per_allocation,
        max_document_bytes: total,
        max_tiled_bytes: total,
    })
}

fn resource_error<T: Debug>(result: Result<T, PsdError>) -> String {
    match result.expect_err("allocation must be refused") {
        PsdError::Unsupported(message) => {
            assert!(message.contains("budget"), "{message}");
            message
        }
        other => panic!("expected the typed budget refusal, got {other:?}"),
    }
}

fn object(value: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"data".to_vec(), value)],
    }
}

fn raw_data(bytes: &[u8]) -> DescValue {
    let mut raw = b"tdta".to_vec();
    raw.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    raw.extend_from_slice(bytes);
    DescValue::Raw(raw)
}

fn cube(size: usize, count: usize) -> Vec<u8> {
    let mut bytes = format!("LUT_3D_SIZE {size}\n").into_bytes();
    for _ in 0..count {
        bytes.extend_from_slice(b"0.25 0.5 0.75\n");
    }
    bytes
}

#[test]
fn admitted_descriptor_matches_the_compatibility_reader() {
    let value = object(DescValue::List(vec![
        DescValue::Long(-7),
        DescValue::Double(1.5),
        DescValue::Text("text 🦜\0".into()),
        DescValue::Enum {
            kind: b"Kind".to_vec(),
            value: b"a-long-value".to_vec(),
        },
        object(DescValue::Bool(true)),
        raw_data(&[1, 2, 3, 4]),
    ]));
    let bytes = crate::write_descriptor(&value);
    let expected = crate::read_descriptor(&bytes).expect("compatibility reader");
    let parsed = read_descriptor(&bytes, &mut budget(1 << 20, 1 << 20))
        .expect("admitted")
        .expect("valid descriptor");
    assert_eq!(parsed, expected);
    assert_eq!(parsed, value);
    assert_eq!(crate::write_descriptor(&parsed), bytes);
}

#[test]
fn malformed_descriptor_is_distinct_from_resource_refusal() {
    let bytes = crate::write_descriptor(&object(DescValue::Bool(true)));
    let mut wrong_version = bytes.clone();
    wrong_version[..4].copy_from_slice(&15u32.to_be_bytes());
    assert_eq!(
        read_descriptor(&wrong_version, &mut budget(0, 0)).expect("malformed"),
        None
    );
    assert_eq!(
        read_descriptor(&bytes[..3], &mut budget(0, 0)).expect("truncated"),
        None
    );
    resource_error(read_descriptor(&bytes, &mut budget(0, 0)));
}

#[test]
fn content_decode_temporaries_are_admitted_before_allocation() {
    let large_text = object(DescValue::Text("a".repeat(4096)));
    let large_raw = object(raw_data(&vec![3; 4096]));
    let large_list = object(DescValue::List(vec![DescValue::Bool(true); 128]));
    let large_object = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: (0..128)
            .map(|i| (format!("key-{i}").into_bytes(), DescValue::Bool(true)))
            .collect(),
    };
    for value in [large_text, large_raw, large_list, large_object] {
        let bytes = crate::write_descriptor(&value);
        resource_error(read_descriptor(&bytes, &mut budget(1024, 1 << 20)));
    }
    resource_error(copy_payload(&[4; 1025], &mut budget(1024, 1 << 20)));
    let bytes = crate::write_descriptor(&object(DescValue::Text("small".into())));
    resource_error(read_descriptor(&bytes, &mut budget(1 << 20, 1)));
}

#[test]
fn descriptor_container_capacity_is_admitted_as_a_whole() {
    let count = 17;
    let descriptor = object(DescValue::List(vec![DescValue::Bool(true); count]));
    let bytes = crate::write_descriptor(&descriptor);
    let per_allocation = (size_of::<DescValue>() * count - 1) as u64;
    resource_error(read_descriptor(
        &bytes,
        &mut budget(per_allocation, 1 << 20),
    ));
}

#[test]
fn descriptor_temporary_claims_are_not_refunded() {
    let bytes = crate::write_descriptor(&object(DescValue::Text("value".into())));
    let mut shared = budget(4096, 4096);
    let mut admitted = 0;
    let mut refused = false;
    for _ in 0..128 {
        match read_descriptor(&bytes, &mut shared) {
            Ok(Some(value)) => {
                assert_eq!(value, object(DescValue::Text("value".into())));
                admitted += 1;
                drop(value);
            }
            Ok(None) => panic!("valid descriptor was reinterpreted as malformed"),
            Err(error) => {
                resource_error::<Option<DescValue>>(Err(error));
                refused = true;
                break;
            }
        }
    }
    assert!(admitted > 0, "the sufficient-budget control must decode");
    assert!(
        refused,
        "one shared cumulative budget must eventually refuse"
    );
}

#[test]
fn raw_copy_respects_individual_and_shared_total_claims() {
    let mut shared = budget(8, 12);
    let first = copy_payload(&[1; 8], &mut shared).expect("first copy admitted");
    assert_eq!(first, [1; 8]);
    resource_error(copy_payload(&[2; 5], &mut shared));
    assert_eq!(first, [1; 8], "the retained copy remains intact");
    resource_error(copy_payload(&[3; 9], &mut budget(8, 64)));
}

#[test]
fn cube_preflight_rejects_excess_without_point_allocation() {
    let bytes = cube(64, 64 * 64 * 64 + 1);
    let before = bytes.clone();
    assert_eq!(
        parse_cube(&bytes, &mut budget(0, 0)).expect("overlong is malformed"),
        None,
        "the point-count preflight must reject excess rows without allocating points"
    );
    assert_eq!(bytes, before, "the original opaque payload is unchanged");
}

#[test]
fn supported_maximum_cube_is_admitted_once_without_growth() {
    let count = 64 * 64 * 64;
    let points_bytes = (count * size_of::<[f32; 3]>()) as u64;
    let bytes = cube(64, count);
    let (size, points) = parse_cube(&bytes, &mut budget(points_bytes, points_bytes))
        .expect("one exact points allocation fits")
        .expect("supported maximum is valid");
    assert_eq!(size, 64);
    assert_eq!(points.len(), count);
    assert_eq!(points[0], [0.25, 0.5, 0.75]);
    assert_eq!(points[count - 1], [0.25, 0.5, 0.75]);
}

#[test]
fn valid_cube_budget_failure_does_not_become_a_missing_lookup() {
    let bytes = cube(2, 8);
    let points_bytes = (8 * size_of::<[f32; 3]>()) as u64;
    resource_error(parse_cube(
        &bytes,
        &mut budget(points_bytes - 1, points_bytes * 2),
    ));
    resource_error(parse_cube(
        &bytes,
        &mut budget(points_bytes * 2, points_bytes - 1),
    ));
}

#[test]
fn retained_lookup_tables_share_the_document_budget() {
    // The import walker passes this same budget regardless of layer visibility.
    let bytes = cube(2, 8);
    let points_bytes = (8 * size_of::<[f32; 3]>()) as u64;
    let mut shared = budget(points_bytes, points_bytes * 2 - 1);
    let first = parse_cube(&bytes, &mut shared)
        .expect("first retained table fits")
        .expect("valid");
    resource_error(parse_cube(&bytes, &mut shared));
    assert_eq!(first.1, vec![[0.25, 0.5, 0.75]; 8]);
    let mut sufficient = budget(points_bytes, points_bytes * 2);
    assert!(parse_cube(&bytes, &mut sufficient).unwrap().is_some());
    assert!(parse_cube(&bytes, &mut sufficient).unwrap().is_some());
}

#[test]
fn dropping_retained_points_does_not_refund_cumulative_admission() {
    let bytes = cube(2, 8);
    let points_bytes = (8 * size_of::<[f32; 3]>()) as u64;
    let mut shared = budget(points_bytes, points_bytes);
    drop(parse_cube(&bytes, &mut shared).unwrap().unwrap());
    resource_error(parse_cube(&bytes, &mut shared));
}

#[test]
fn admitted_cube_preserves_later_size_headers_and_raw_descriptor() {
    let mut bytes = cube(2, 27);
    bytes.extend_from_slice(b"LUT_3D_SIZE 3\n");
    let points_bytes = (27 * size_of::<[f32; 3]>()) as u64;
    let (size, points) = parse_cube(&bytes, &mut budget(points_bytes, points_bytes))
        .expect("admitted")
        .expect("later size declaration retains existing accepted semantics");
    assert_eq!(size, 3);
    assert_eq!(points, vec![[0.25, 0.5, 0.75]; 27]);
    let descriptor = crate::write_descriptor(&object(raw_data(&bytes)));
    let original = descriptor.clone();
    let decoded = read_descriptor(&descriptor, &mut budget(1 << 20, 1 << 20))
        .expect("descriptor admitted")
        .expect("descriptor valid");
    assert_eq!(crate::write_descriptor(&decoded), original);
    assert_eq!(descriptor, original);
}

#[test]
fn malformed_cube_keeps_the_existing_no_lookup_result() {
    for bytes in [
        b"LUT_3D_SIZE 1\n".as_slice(),
        b"LUT_3D_SIZE 2\n0 0 0\n".as_slice(),
        b"LUT_3D_SIZE 2\nNaN 0 0\n".as_slice(),
        b"0 0 0\n".as_slice(),
    ] {
        assert_eq!(
            parse_cube(bytes, &mut budget(0, 0)).expect("malformed, not resource refusal"),
            None
        );
    }
}

fn image_budget(per_allocation: u64, total: u64) -> ImageBudget {
    ImageBudget {
        max_dimension: 16,
        max_alloc_bytes: per_allocation,
        max_document_bytes: total,
        max_tiled_bytes: total,
    }
}

fn lookup_document(payload: &[u8], count: usize) -> pictura_core::Document {
    use pictura_core::{BitDepth, ColorMode, Document, LayerParts, LayerStack, PsdRect};

    let mut document = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    document.layers = LayerStack::from_layers(
        (0..count)
            .map(|index| {
                crate::layer_from_parts(LayerParts {
                    name: format!("Hidden Lookup {index}"),
                    rect: PsdRect {
                        top: 0,
                        left: 0,
                        bottom: 1,
                        right: 1,
                    },
                    visible: false,
                    adjustment: Some(pictura_core::AdjustmentData {
                        key: *b"clrL",
                        data: payload.to_vec().into(),
                    }),
                    ..Default::default()
                })
            })
            .collect(),
    );
    document
}

#[test]
fn oversized_cube_points_are_bounded_during_import() {
    let raw = crate::layer_content::encode_color_lookup(&cube(64, 64 * 64 * 64 + 1), "Excess");
    let document = lookup_document(&raw.data, 1);
    let bytes = crate::write_psd(&document).expect("generated PSD");
    let reopened = crate::read_psd_within_options(
        &bytes,
        &crate::OpenOptions::default(),
        image_budget(32 << 20, 64 << 20),
    )
    .expect("malformed lookup is preserved within the admitted storage");
    assert!(!reopened.layers[0].visible);
    let Some(pictura_core::Adjustment::ColorLookup(parameters)) =
        reopened.layers[0].adjustment_parameters()
    else {
        panic!("the supported Color Lookup keeps its no-lookup representation");
    };
    assert_eq!(parameters.lookup, None);
    assert_eq!(reopened.layers[0].adjustment(), Some(&raw));
}

#[test]
fn hidden_lookup_tables_share_the_document_budget() {
    let raw = crate::layer_content::encode_color_lookup(&cube(4, 64), "Hidden");
    let one = crate::write_psd(&lookup_document(&raw.data, 1)).unwrap();
    let two = crate::write_psd(&lookup_document(&raw.data, 2)).unwrap();
    let options = crate::OpenOptions::default();
    let admitted = crate::read_psd_within_options(&two, &options, image_budget(1 << 20, 1 << 20))
        .expect("sufficient-budget control");
    assert_eq!(admitted.layers.len(), 2);
    for layer in admitted.layers.iter() {
        assert!(!layer.visible);
        let Some(pictura_core::Adjustment::ColorLookup(parameters)) = layer.adjustment_parameters()
        else {
            panic!("hidden layers still retain their supported content");
        };
        assert_eq!(parameters.lookup.as_ref().unwrap().points.len(), 64);
    }

    // The LUT allocation window is wider than this step on every supported
    // target; finding it avoids hard-coding DOM enum and channel metadata sizes.
    let mut retained_total_refused = false;
    for total in (128..16384).step_by(128) {
        let limits = image_budget(1 << 20, total);
        if crate::read_psd_within_options(&one, &options, limits).is_err() {
            continue;
        }
        if let Err(PsdError::Unsupported(message)) =
            crate::read_psd_within_options(&two, &options, limits)
        {
            if message.contains("content LUT points") && message.contains("budget") {
                retained_total_refused = true;
                break;
            }
        }
    }
    assert!(
        retained_total_refused,
        "the second hidden LUT must be admitted against the same document total"
    );
}

#[test]
fn imported_ignored_dom_fields_are_still_budgeted() {
    let raw = crate::layer_content::encode_color_lookup(&cube(2, 8), "Small");
    let mut descriptor = crate::read_descriptor(&raw.data[2..]).unwrap();
    let DescValue::Object { items, .. } = &mut descriptor else {
        panic!("Color Lookup descriptor");
    };
    items.push((b"unmodeled".to_vec(), DescValue::Text("x".repeat(4096))));
    let mut payload = 1u16.to_be_bytes().to_vec();
    payload.extend_from_slice(&crate::write_descriptor(&descriptor));
    let bytes = crate::write_psd(&lookup_document(&payload, 1)).unwrap();
    resource_error(crate::read_psd_within_options(
        &bytes,
        &crate::OpenOptions::default(),
        image_budget(2048, 1 << 20),
    ));
    let admitted = crate::read_psd_within_options(
        &bytes,
        &crate::OpenOptions::default(),
        image_budget(1 << 20, 1 << 20),
    )
    .expect("the sufficient-budget control still reads valid known parameters");
    assert_eq!(
        admitted.layers[0].adjustment().unwrap().data,
        payload.as_slice()
    );
}

#[test]
fn admitted_content_preserves_parameters_and_bytes() {
    use pictura_core::{AdjustmentData, FillContent, LayerKind, LayerParts, PsdRect};

    let mut cube_bytes = cube(2, 27);
    cube_bytes.extend_from_slice(b"LUT_3D_SIZE 3\n");
    let mut raw = crate::layer_content::encode_color_lookup(&cube_bytes, "Later Header");
    let mut data = raw.data.to_vec();
    data.resize(data.len() + (data.len() & 1), 0);
    raw.data = data.into();
    let mut document = lookup_document(&raw.data, 1);
    document
        .layers
        .push(crate::layer_from_parts(LayerParts {
            name: "Fill".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 1,
            },
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![10, 20, 30, 255].into(),
            }),
            ..Default::default()
        }))
        .expect("valid fill layer");
    let bytes = crate::write_psd(&document).unwrap();
    let reopened = crate::read_psd_within_options(
        &bytes,
        &crate::OpenOptions::default(),
        image_budget(1 << 20, 1 << 20),
    )
    .expect("all derived content fits the explicit budget");
    let Some(pictura_core::Adjustment::ColorLookup(parameters)) =
        reopened.layers[0].adjustment_parameters()
    else {
        panic!("retained typed lookup");
    };
    let lookup = parameters.lookup.as_ref().unwrap();
    assert_eq!(lookup.size, 3);
    assert_eq!(lookup.points, vec![[0.25, 0.5, 0.75]; 27]);
    assert_eq!(reopened.layers[1].kind(), LayerKind::Fill);
    assert_eq!(
        reopened.layers[1].fill_content(),
        Some(&FillContent::Solid([10, 20, 30, 255]))
    );
    assert_eq!(
        pictura_render::composite_rgba(&reopened).data.as_slice(),
        &[10, 20, 30, 255]
    );
    for (original, decoded) in document.layers.iter().zip(reopened.layers.iter()) {
        assert_eq!(decoded.adjustment(), original.adjustment());
    }
    assert_eq!(crate::write_psd(&reopened).unwrap(), bytes);
}

#[test]
fn growing_content_vectors_admit_replacement_while_old_storage_is_live() {
    let element = size_of::<[f32; 3]>() as u64;
    let mut refused = budget(8 * element, 12 * element - 1);
    let mut alloc = super::ContentAlloc::new(Some(&mut refused));
    let mut points: Vec<[f32; 3]> = alloc.vec(4, "test content vector").unwrap();
    points.extend_from_slice(&[[0.0; 3]; 4]);
    assert!(alloc
        .push(&mut points, [1.0; 3], "test content vector")
        .is_none());
    resource_error(alloc.finish(Some(points)));

    let mut admitted = budget(8 * element, 12 * element);
    let mut alloc = super::ContentAlloc::new(Some(&mut admitted));
    let mut points: Vec<[f32; 3]> = alloc.vec(4, "test content vector").unwrap();
    points.extend_from_slice(&[[0.0; 3]; 4]);
    alloc
        .push(&mut points, [1.0; 3], "test content vector")
        .unwrap();
    let points = alloc.finish(Some(points)).unwrap().unwrap();
    assert_eq!(points.len(), 5);
    assert_eq!(points[4], [1.0; 3]);
    resource_error(admitted.charge("the cumulative claim remains", &[1]));
}
