use crate::probe::{probe_image, ImageBudget, ImageFormat, ImportError, LimitKind};

fn png(width: u32, height: u32, depth: u8) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    v.extend_from_slice(&13u32.to_be_bytes());
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.push(depth);
    v.push(6);
    v.extend_from_slice(&[0, 0, 0]);
    v.extend_from_slice(&0u32.to_be_bytes());
    v
}

fn jpeg(width: u16, height: u16, precision: u8) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    v.extend_from_slice(b"JFIF\0");
    v.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    v.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, precision]);
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
    v
}

fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&[0x00, 0x00, 0x00]);
    v
}

fn bmp(width: i32, height: i32, bpp: u16) -> Vec<u8> {
    let mut v = b"BM".to_vec();
    v.extend_from_slice(&[0u8; 12]);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&bpp.to_le_bytes());
    v.extend_from_slice(&[0u8; 24]);
    v
}

fn tiff(width: u32, height: u32, bits: u16) -> Vec<u8> {
    let mut v = b"II\x2a\x00".to_vec();
    v.extend_from_slice(&8u32.to_le_bytes());
    v.extend_from_slice(&3u16.to_le_bytes());
    for (tag, value) in [(256u16, width), (257, height)] {
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&4u16.to_le_bytes());
        v.extend_from_slice(&1u32.to_le_bytes());
        v.extend_from_slice(&value.to_le_bytes());
    }
    v.extend_from_slice(&258u16.to_le_bytes());
    v.extend_from_slice(&3u16.to_le_bytes());
    v.extend_from_slice(&1u32.to_le_bytes());
    v.extend_from_slice(&(bits as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

fn webp_vp8x(width: u32, height: u32) -> Vec<u8> {
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(b"WEBP");
    v.extend_from_slice(b"VP8X");
    v.extend_from_slice(&10u32.to_le_bytes());
    v.push(0);
    v.extend_from_slice(&[0, 0, 0]);
    v.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    v.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    v.extend_from_slice(&[0, 0]);
    v
}

#[test]
fn recognizes_each_container_header() {
    let budget = ImageBudget::default();
    let p = probe_image(&png(640, 480, 8), budget).expect("png");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::Png, 640, 480, 8)
    );

    let p = probe_image(&jpeg(320, 240, 8), budget).expect("jpeg");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::Jpeg, 320, 240, 8)
    );

    let p = probe_image(&gif(12, 34), budget).expect("gif");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::Gif, 12, 34, 8)
    );

    let p = probe_image(&bmp(100, 50, 24), budget).expect("bmp");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::Bmp, 100, 50, 24)
    );

    let p = probe_image(&tiff(70, 90, 8), budget).expect("tiff");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::Tiff, 70, 90, 8)
    );

    let p = probe_image(&webp_vp8x(800, 600), budget).expect("webp");
    assert_eq!(
        (p.format, p.width, p.height, p.bit_depth),
        (ImageFormat::WebP, 800, 600, 8)
    );
}

#[test]
fn bmp_top_down_negative_height_is_absolute() {
    let p = probe_image(&bmp(64, -48, 32), ImageBudget::default()).expect("bmp");
    assert_eq!((p.width, p.height, p.bit_depth), (64, 48, 32));
}

#[test]
fn refuses_dimension_over_budget_naming_the_limit() {
    let err = probe_image(&png(40_000, 10, 8), ImageBudget::default()).unwrap_err();
    assert_eq!(err.limit_exceeded(), Some(LimitKind::Dimension));
    assert!(matches!(err, ImportError::DimensionLimit { .. }));
}

#[test]
fn refuses_allocation_over_budget_naming_the_limit() {
    let err = probe_image(&png(30_000, 30_000, 8), ImageBudget::default()).unwrap_err();
    assert_eq!(err.limit_exceeded(), Some(LimitKind::Allocation));
    match err {
        ImportError::AllocationLimit { bytes, limit, .. } => {
            assert_eq!(bytes, 30_000u64 * 30_000 * 4);
            assert_eq!(limit, 2 * 1024 * 1024 * 1024);
        }
        other => panic!("expected AllocationLimit, got {other:?}"),
    }
}

#[test]
fn admits_a_16000_class_raster_under_the_default_budget() {
    // 16507×16196×4 ≈ 1020 MiB: the 16000²-class import the canvas targets,
    // over the former 512 MiB bound. The header must probe Ok, not AllocationLimit.
    let p = probe_image(&png(16_507, 16_196, 8), ImageBudget::default())
        .expect("a 16000-class RGBA raster is within the default budget");
    assert_eq!((p.width, p.height), (16_507, 16_196));
    assert_eq!(p.format, ImageFormat::Png);
}

#[test]
fn overflowing_allocation_estimate_is_refused_not_panicking() {
    // A permissive budget leaves the dimension check silent, so the multiply is
    // reached with u32::MAX x u32::MAX, whose RGBA estimate exceeds u64.
    let budget = ImageBudget {
        max_dimension: u32::MAX,
        max_alloc_bytes: u64::MAX,
        ..ImageBudget::default()
    };
    let err = probe_image(&png(u32::MAX, u32::MAX, 8), budget).unwrap_err();
    assert_eq!(err.limit_exceeded(), Some(LimitKind::Allocation));
    assert!(matches!(
        err,
        ImportError::AllocationLimit {
            bytes: u64::MAX,
            ..
        }
    ));
}

#[test]
fn refuses_unknown_and_truncated_without_panicking() {
    let unknown = probe_image(b"not an image at all", ImageBudget::default()).unwrap_err();
    assert!(matches!(unknown, ImportError::UnknownContainer { .. }));
    assert_eq!(unknown.source(), "unknown");

    let truncated = probe_image(b"\x89PNG\r\n\x1a\n", ImageBudget::default()).unwrap_err();
    assert!(matches!(truncated, ImportError::Truncated { .. }));
    assert_eq!(truncated.source(), "PNG");

    for bytes in [
        &b""[..],
        &b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR"[..],
        &[0xFF, 0xD8, 0xFF][..],
        &b"GIF89a\x00"[..],
        &b"BM\x00"[..],
        &b"II\x2a\x00"[..],
        &b"RIFF\x00\x00\x00\x00WEBP"[..],
    ] {
        assert!(probe_image(bytes, ImageBudget::default()).is_err());
    }
}

#[test]
fn default_budget_values_are_documented() {
    let budget = ImageBudget::default();
    assert_eq!(budget.max_dimension, 30_000);
    assert_eq!(budget.max_alloc_bytes, 2 * 1024 * 1024 * 1024);
}
