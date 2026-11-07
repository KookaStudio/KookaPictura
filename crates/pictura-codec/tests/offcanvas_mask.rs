use pictura_codec::{read_psd, write_psb, write_psd};
use pictura_core::{BitDepth, Document, LayerMask, PsdRect, Samples, SourceChannels};

#[test]
fn offcanvas_mask_round_trips_at_native_depth() {
    for depth in [BitDepth::Eight, BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        let native = match depth {
            BitDepth::Eight => Samples::U8(vec![0, 1, 64, 128, 254, 255].into()),
            BitDepth::Sixteen => Samples::U16(vec![0, 1, 257, 32768, 65534, 65535].into()),
            BitDepth::ThirtyTwo => {
                Samples::F32(vec![0.0, f32::from_bits(1), 0.0001, 0.5, 0.9999, 1.0].into())
            }
            _ => unreachable!(),
        };
        for copies in [1, 25] {
            let native = match &native {
                Samples::U8(p) => Samples::U8(p.to_vec().repeat(copies).into()),
                Samples::U16(p) => Samples::U16(p.to_vec().repeat(copies).into()),
                Samples::F32(p) => Samples::F32(p.to_vec().repeat(copies).into()),
            };
            let (width, height) = if copies == 1 { (3, 2) } else { (15, 10) };
            for psb in [false, true] {
                let rgba = [32, 64, 192, 255].repeat(12 * 8);
                let mut doc = Document::from_rgba("Mask", 12, 8, &rgba);
                doc.source_depth = (depth != BitDepth::Eight).then_some(depth);
                let mut mask = LayerMask {
                    rect: PsdRect {
                        left: -30,
                        top: 40,
                        right: -30 + width,
                        bottom: 40 + height,
                    },
                    default_color: 0,
                    density: 192,
                    flags: 0x11,
                    data: Some(native.narrow_to_u8().into()),
                    ..Default::default()
                };
                mask.set_feather(1.25);
                doc.layers[0].mask = Some(mask.clone());
                if depth != BitDepth::Eight {
                    doc.layers[0].source_channels = Some(SourceChannels::new(
                        depth,
                        doc.layers[0].rect,
                        vec![(-2, native.clone())],
                    ));
                }
                let encoded = if psb {
                    write_psb(&doc)
                } else {
                    write_psd(&doc)
                }
                .expect("encode retained bounds");
                assert_eq!(
                    u16::from_be_bytes(encoded[22..24].try_into().unwrap()),
                    match depth {
                        BitDepth::Eight => 8,
                        BitDepth::Sixteen => 16,
                        BitDepth::ThirtyTwo => 32,
                        _ => unreachable!(),
                    }
                );
                let mut loaded = read_psd(&encoded).expect("decode retained bounds");
                assert_eq!(loaded.layers[0].mask, Some(mask.clone()));
                if depth != BitDepth::Eight {
                    let retained = &loaded.layers[0]
                        .source_channels
                        .as_ref()
                        .unwrap()
                        .planes
                        .iter()
                        .find(|(id, _)| *id == -2)
                        .unwrap()
                        .1;
                    assert_eq!(retained.to_bytes(), native.to_bytes());
                }
                let moved = loaded.layers[0].mask.as_mut().unwrap();
                moved.rect.left += 32;
                moved.rect.right += 32;
                moved.rect.top -= 39;
                moved.rect.bottom -= 39;
                assert_eq!(
                    moved.rect,
                    PsdRect {
                        left: 2,
                        top: 1,
                        right: 2 + width,
                        bottom: 1 + height
                    }
                );
                assert_eq!(
                    moved.data.as_ref().unwrap().as_slice(),
                    native.narrow_to_u8()
                );
                assert_eq!(
                    (moved.default_color, moved.density, moved.feather()),
                    (0, 192, 1.25)
                );
            }
        }
    }
}
