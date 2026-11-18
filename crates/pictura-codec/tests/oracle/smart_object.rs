use super::*;

/// A 4x4 RGB document with one layer holding an embedded smart object, which
/// the writer authors as a `SoLd` block plus a document-level `lnk2` record.
fn embedded_smart_doc() -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .edit_data(|data| {
            for (i, b) in data.iter_mut().enumerate() {
                *b = (i * 7 + 1) as u8;
            }
        })
        .expect("pixel edit preserves geometry");
    doc.layers = pictura_core::LayerStack::from_layers(vec![pictura_codec::layer_from_parts(
        pictura_core::LayerParts {
            name: "Smart".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![10; 16].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20; 16].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30; 16].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; 16].into(),
                },
            ],
            smart_object: Some(
                {
                    let mut so = SmartObject::default();
                    so.filename = "source.psb".into();
                    so.kind = SmartObjectKind::Embedded;
                    so
                }
                .with_payload((0..4096).map(|i| (i % 251) as u8).collect()),
            ),
            ..Default::default()
        },
    )]);
    doc
}

/// Run psd-tools over `bytes`, asserting `PSD.read` and `PSDImage.open` both
/// succeed, and return its stdout.
fn psd_tools_reads(tag: &str, bytes: &[u8]) -> String {
    psd_tools_inspect(
        tag,
        bytes,
        r#"
import sys
from psd_tools import PSDImage
from psd_tools.psd import PSD
with open(sys.argv[1], "rb") as f:
    PSD.read(f)
image = PSDImage.open(sys.argv[1], lazy=False)
so = image[0].smart_object
print(so.kind)
print(so.filename)
print(len(so.data))
"#,
    )
}

fn psd_tools_inspect(tag: &str, bytes: &[u8], script: &str) -> String {
    let dir = scratch_dir(tag);
    let path = dir.join("doc.psb");
    std::fs::write(&path, bytes).unwrap();
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "psd-tools failed to parse:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Assert `psd_tools_reads` stdout reports the authored embedded smart object.
fn assert_embedded_smart_object(stdout: &str) {
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next().map(str::trim),
        Some("data"),
        "psd-tools sees the embedded smart object; stdout={stdout}"
    );
    assert_eq!(
        lines.next().map(str::trim),
        Some("source.psb"),
        "psd-tools reports the authored filename; stdout={stdout}"
    );
    let len: usize = lines
        .next()
        .expect("embedded data length line")
        .trim()
        .parse()
        .unwrap();
    assert_eq!(len, 4096, "embedded payload length matches");
}

/// CRITICAL repro: a PSB carrying an authored smart object frames its `lnk2`
/// big-key block with a `u64` length, so psd-tools parses the whole document
/// without a framing error (previously a MemoryError).
#[test]
fn psd_tools_parses_authored_smart_object_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let bytes = write_psb(&embedded_smart_doc()).unwrap();
    let stdout = psd_tools_reads("psb-authored-smart-object", &bytes);
    assert_embedded_smart_object(&stdout);
}

#[test]
fn vector_source_round_trips_with_its_preview_and_selection() {
    let mut doc = embedded_smart_doc();
    let pdf = pictura_codec::illustrator::write_pdf(4, 4, &[]);
    let selection = pictura_core::VectorPlacement {
        page: 1,
        page_count: 1,
        crop: 2,
        width_points: 4.0,
        height_points: 4.0,
    };
    let original = &mut doc.layers[0];
    let source_rect = original.rect;
    pictura_codec::edit_layer(original, |parts| {
        let object = parts.smart_object.as_mut().unwrap();
        object.uuid = "11111111-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into();
        object.filename = "vector.ai".into();
        object.filetype = *b"AI  ";
        object.vector_placement = Some(selection);
        object
            .vector_transforms
            .push(pictura_core::VectorTransform {
                source: source_rect,
                destination: Some(source_rect),
                quad: [(0.0, 0.0), (4.0, 0.0), (5.0, 5.0), (0.0, 4.0)],
                warp: Some(pictura_core::warp::Warp::preset(
                    pictura_core::warp::WarpStyle::Arc,
                    20.0,
                    [0.0, 0.0, 4.0, 4.0],
                )),
            });
        object.set_payload(pdf.clone());
    });
    let shared = original.clone();
    let mut independent = original.clone();
    let independent_id = "22222222-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    pictura_codec::edit_layer(&mut independent, |parts| {
        parts.smart_object.as_mut().unwrap().uuid = independent_id.into();
    });
    doc.layers.push(shared).unwrap();
    doc.layers.push(independent).unwrap();

    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let restored = pictura_codec::read_psd(&bytes).unwrap();
        assert_eq!(restored.layers.len(), 3);
        for (layer, original) in restored.layers.iter().zip(doc.layers.iter()) {
            let object = layer.smart_object().unwrap();
            assert_eq!(object.uuid, original.smart_object().unwrap().uuid);
            assert_eq!(object.filename, "vector.ai");
            assert_eq!(object.filetype, *b"AI  ");
            assert_eq!(object.vector_placement, Some(selection));
            assert_eq!(
                object.vector_transforms,
                original.smart_object().unwrap().vector_transforms
            );
            assert_eq!(object.payload(), Some(pdf.as_slice()));
            assert_eq!(layer.channels(), original.channels());
        }
        if psd_tools_available() {
            let stdout = psd_tools_reads("vector-smart-object", &bytes);
            let lines = stdout.lines().collect::<Vec<_>>();
            assert_eq!(lines, vec!["data", "vector.ai", &pdf.len().to_string()]);
        }
    }
}

#[test]
fn authored_vector_placement_descriptor_oracle() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = embedded_smart_doc();
    let layer = &mut doc.layers[0];
    layer.rect = PsdRect {
        left: 3,
        top: 2,
        right: 7,
        bottom: 6,
    };
    let payload = pictura_codec::illustrator::write_pdf(8, 4, &[]);
    pictura_codec::edit_layer(layer, |parts| {
        let object = parts.smart_object.as_mut().unwrap();
        object.set_payload(payload);
        object.filename = "source.ai".into();
        object.filetype = *b"AI  ";
        object.vector_placement = Some(pictura_core::VectorPlacement {
            page: 1,
            page_count: 1,
            crop: 4,
            width_points: 8.0,
            height_points: 4.0,
        });
    });
    let inspect = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag
desc = PSDImage.open(sys.argv[1])[0].tagged_blocks.get_data(Tag.SMART_OBJECT_LAYER_DATA1).data
assert desc[b'PgNm'].value == 1 and desc[b'totalPages'].value == 1
assert desc[b'Crop'].value == 4
assert desc[b'Sz  '][b'Wdth'].value == 8 and desc[b'Sz  '][b'Hght'].value == 4
quad = tuple(v.value for v in desc[b'Trnf'])
assert quad == tuple(v.value for v in desc[b'nonAffineTransform'])
print(' '.join(f'{v:.8g}' for v in quad))
print(desc[b'warp'][b'warpStyle'].enum.decode('ascii'))
print(f"{desc[b'warp'][b'warpValue'].value:.8g}")
"#;
    let original = write_psd(&doc).unwrap();
    assert_eq!(
        psd_tools_inspect("vector-place-initial-geometry", &original, inspect).trim(),
        "3 2 7 2 7 6 3 6\nwarpNone\n0"
    );
    let mut reopened = read_psd(&original).unwrap();
    let layer = &mut reopened.layers[0];
    let source = layer.rect;
    pictura_codec::edit_layer(layer, |parts| {
        let object = parts.smart_object.as_mut().unwrap();
        assert!(object.authored_vector_placement);
        assert!(object.vector_transforms.is_empty());
        object
            .vector_transforms
            .push(pictura_core::VectorTransform {
                source,
                destination: Some(source),
                quad: [(7.0, 2.0), (7.0, 6.0), (3.0, 6.0), (3.0, 2.0)],
                warp: Some(pictura_core::warp::Warp::preset(
                    pictura_core::warp::WarpStyle::Arc,
                    20.0,
                    [3.0, 2.0, 7.0, 6.0],
                )),
            });
    });
    layer.rect.left += 6;
    layer.rect.right += 6;
    let resaved = write_psb(&reopened).unwrap();
    assert_eq!(
        psd_tools_inspect("vector-place-resaved-geometry", &resaved, inspect).trim(),
        "13 2 13 6 9 6 9 2\nwarpArc\n20"
    );
    let restored = read_psd(&resaved).unwrap();
    assert_eq!(
        restored.layers[0].smart_object().unwrap().vector_transforms,
        reopened.layers[0].smart_object().unwrap().vector_transforms
    );
}

/// CRITICAL-1 regression: an odd-length tagged block is declared even with its
/// pad inside, so psd-tools still finds the block that follows it.
#[test]
fn psd_tools_parses_block_after_odd_length_block_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = embedded_smart_doc();
    // A 5-byte (odd) and a 6-byte (non-4-multiple) per-layer block precede the
    // authored `SoLd`. With the old external pad the next `8BIM` is misread and
    // the smart object is dropped.
    pictura_codec::edit_layer(doc.layers[0].as_mut(), |parts| {
        parts.extra_blocks = vec![
            LayerBlock {
                key: *b"zzzz",
                data: vec![1, 2, 3, 4, 5].into(),
            },
            LayerBlock {
                key: *b"yyyy",
                data: vec![1, 2, 3, 4, 5, 6].into(),
            },
        ]
    });
    let bytes = write_psb(&doc).unwrap();
    let stdout = psd_tools_reads("psb-odd-block-then-smart", &bytes);
    assert_embedded_smart_object(&stdout);
}

/// CRITICAL-2 regression: a PSD-sourced document whose preserved `lnk2` is
/// u32-framed is re-framed to u64 when written as a PSB, so psd-tools reads it.
#[test]
fn psd_tools_parses_reframed_psd_big_key_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let psd_bytes = write_psd(&embedded_smart_doc()).unwrap();
    let src = read_psd(&psd_bytes).unwrap();
    assert!(!src.is_psb, "the intermediate is a PSD");

    let bytes = write_psb(&src).unwrap();
    assert_eq!(
        u16::from_be_bytes([bytes[4], bytes[5]]),
        2,
        "the re-save is a PSB"
    );
    let stdout = psd_tools_reads("psb-reframed-psd-big-key", &bytes);
    assert_embedded_smart_object(&stdout);
}

/// Fix A regression: a preserved document-level block whose length is not a
/// multiple of 4 declares its exact length and is padded externally to 4, so
/// psd-tools still reads the authored `lnk2` that follows it.
#[test]
fn psd_tools_parses_doc_level_block_not_multiple_of_four_in_psb() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }
    let mut doc = embedded_smart_doc();
    // A 6-byte unknown global block: exact length 6 + 2 external pad bytes, which
    // the old per-layer even-inside rule would have mis-framed.
    let mut block = b"8BIMzzzz".to_vec();
    block.extend_from_slice(&6u32.to_be_bytes());
    block.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    block.extend_from_slice(&[0, 0]);
    doc.layer_section_extra = block.into();

    let bytes = write_psb(&doc).unwrap();
    let stdout = psd_tools_reads("psb-doc-block-not-mult-of-four", &bytes);
    assert_embedded_smart_object(&stdout);
}
