use super::*;

#[test]
fn packed_bitmap_composite_keeps_native_and_extra_planes() {
    for width in [1, 8, 10] {
        let packed = if width == 10 {
            vec![0xaa, 0xc0, 0x00, 0xff]
        } else {
            vec![0xaa, 0x00]
        };
        let extra = if width == 10 {
            vec![0xf0, 0x00, 0x55, 0xc0]
        } else {
            vec![0xf0, 0x55]
        };
        let bytes = flat_psd(1, 0, 2, width, 2, &[&packed, &extra]);
        let mut doc = read_psd(&bytes).unwrap();
        assert_eq!(
            doc.composite.data.as_slice(),
            crate::color_mode::bitmap_rows_to_rgb(&packed, width as usize, 2)
        );
        assert_eq!(doc.channels[0].data.as_slice(), extra);
        let Samples::U8(samples) = &doc.source.planes().as_ref().unwrap().samples else {
            panic!("expected packed bytes")
        };
        assert_eq!(samples.as_slice(), [&packed[..], &extra[..]].concat());
        for compression in [Compression::Raw, Compression::Rle] {
            doc.composite_compression = compression;
            for output in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
                let back = read_psd(&output).unwrap();
                assert_eq!(back.composite.data, doc.composite.data);
                assert_eq!(back.channels[0].data, doc.channels[0].data);
                assert_eq!(
                    back.source.planes().as_ref().unwrap().samples,
                    doc.source.planes().as_ref().unwrap().samples
                );
            }
        }
    }
}

#[test]
fn bitmap_without_merged_image_keeps_placeholder_and_output_policy() {
    let mut bytes = layered_psd_depth(1, 0, 1, &[&[0x80]], &[(0, &[0x80])]);
    bytes.truncate(bytes.len() - 3);
    let doc = read_psd(&bytes).unwrap();
    assert!(!doc.composite.merged_output());
    assert_eq!(doc.composite.data.as_slice(), &[255, 255, 255]);
    assert!(doc.source.planes().is_none());
}

#[test]
fn short_indexed_composite_data_is_a_typed_error_not_a_panic() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.source
        .set_encoding(
            Some(ColorMode::Indexed),
            Some(super::indexed::indexed_palette()),
        )
        .unwrap();
    doc.source
        .replace_planes(Some(SourcePlanes {
            depth: BitDepth::Eight,
            width: 2,
            height: 2,
            samples: Samples::U8(vec![0u8; 4].into()),
        }))
        .unwrap();
    let before = doc.clone();
    assert!(doc.composite.replace_data(vec![0u8; 4].into()).is_err());
    assert_eq!(
        doc, before,
        "an invalid composite is rejected before reaching the writer"
    );
}

#[test]
fn native_and_constructed_documents_have_no_source_mode() {
    for (mode, code) in [(ColorMode::Rgb, 3u16), (ColorMode::Grayscale, 1u16)] {
        let channels = mode.color_channels() as u16;
        let plane = vec![7u8; channels as usize];
        let p = flat_psd(8, code, channels, 1, 1, &[&plane]);
        assert_eq!(read_psd(&p).unwrap().source.mode(), None);
    }
    assert_eq!(
        Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight)
            .source
            .mode(),
        None
    );
    assert_eq!(
        Document::from_rgba("x", 1, 1, &[0u8; 4]).source.mode(),
        None
    );
}

#[test]
fn edited_cmyk_plane_uses_the_exact_inverse() {
    // An edit replaces the retained plane, so the writer re-encodes with
    // `rgb_to_cmyk`; K = 255 makes it an exact right-inverse, so the edited RGB
    // reads back byte-for-byte (unlike the quantizing Lab inverse).
    let p = flat_psd(8, 4, 4, 1, 1, &[&[128], &[64], &[32], &[200]]);
    let mut doc = read_psd(&p).unwrap();
    assert_eq!(doc.composite.data, vec![100, 50, 25]);
    let edited = [7u8, 200, 123];
    doc.composite
        .edit_data(|data| {
            data[..3].copy_from_slice(&edited);
        })
        .expect("pixel edit preserves geometry");

    let out = write_psd(&doc).unwrap();
    let back = read_psd(&out).unwrap();
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes(),
        crate::color_mode::rgb_to_cmyk(&edited),
        "an edited plane is re-encoded with rgb_to_cmyk"
    );
    assert_eq!(
        back.composite.data, edited,
        "the CMYK inverse is exact, not approximate"
    );
}

#[test]
fn edited_lab_plane_uses_the_approximate_inverse() {
    // An edit replaces the retained plane, so the writer re-encodes with
    // `rgb_to_lab`. ponytail: 8-bit Lab cannot represent saturated colors
    // exactly, so the re-decoded RGB is approximate (the documented ceiling).
    let p = flat_psd(8, 9, 3, 1, 1, &[&[225], &[82], &[114]]);
    let mut doc = read_psd(&p).unwrap();
    let edited = [0u8, 255, 128];
    doc.composite
        .edit_data(|data| {
            data[..3].copy_from_slice(&edited);
        })
        .expect("pixel edit preserves geometry");

    let out = write_psd(&doc).unwrap();
    let back = read_psd(&out).unwrap();
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes(),
        crate::color_mode::rgb_to_lab(&edited),
        "an edited plane is re-encoded with rgb_to_lab"
    );
    assert_ne!(
        back.composite.data[..3],
        edited[..],
        "the saturated Lab decode is approximate, not exact"
    );
}
