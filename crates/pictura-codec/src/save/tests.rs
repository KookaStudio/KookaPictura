use super::*;
use pictura_core::path::{NamedPath, PathPoint, Subpath, VectorPath};
use pictura_core::{BitDepth, ColorMode, Document, Samples, SourcePlanes};

const JPEG: &[u8] = include_bytes!("../../tests/fixtures/one.jpg");

fn rgba_doc(rgba: &[u8], width: u32, height: u32) -> Document {
    Document::from_rgba("test", width, height, rgba)
}

fn raster(doc: &Document) -> Raster {
    Raster::from_document(doc).expect("raster")
}

fn opts() -> SaveOptions {
    SaveOptions::default().with_defaults()
}

fn jpeg() -> impl Fn(&[u8], u32, u32, u8, u8) -> Option<Vec<u8>> {
    |_samples: &[u8], _w: u32, _h: u32, _c: u8, _q: u8| Some(JPEG.to_vec())
}

fn preview_block(bytes: &[u8]) -> Option<&[u8]> {
    if !bytes.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) {
        return None;
    }
    // The TIFF preview lives in the metafile offset/length fields (20/24);
    // the 12/16 pair is the WMF preview.
    let offset = u32::from_le_bytes(bytes.get(20..24)?.try_into().ok()?) as usize;
    let len = u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?) as usize;
    bytes.get(offset..offset + len)
}

fn triangle() -> VectorPath {
    let mut path = VectorPath::default();
    path.subpaths.push(Subpath {
        points: vec![
            PathPoint {
                anchor: (1.0, 1.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
            PathPoint {
                anchor: (3.0, 1.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
            PathPoint {
                anchor: (2.0, 3.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
        ],
        closed: true,
    });
    path
}

fn with_clipping_path(mut doc: Document, name: &str, path: VectorPath) -> Document {
    doc.saved_paths.push(NamedPath {
        name: name.to_string(),
        path,
    });
    let mut data = vec![name.len() as u8];
    data.extend_from_slice(name.as_bytes());
    doc.image_resources =
        crate::encode_image_resources(&[crate::frame_image_resource(2999, "", &data)]).into();
    doc
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u32_be(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn tiff_ifd_tag(bytes: &[u8], ifd: usize, tag: u16) -> Option<Vec<u32>> {
    let little = bytes.starts_with(b"II");
    let read_u16 = |at: usize| {
        if little {
            u16::from_le_bytes([bytes[at], bytes[at + 1]])
        } else {
            u16::from_be_bytes([bytes[at], bytes[at + 1]])
        }
    };
    let read_u32 = |at: usize| {
        if little {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        } else {
            u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        }
    };
    let count = read_u16(ifd) as usize;
    for i in 0..count {
        let at = ifd + 2 + i * 12;
        if read_u16(at) != tag {
            continue;
        }
        let typ = read_u16(at + 2);
        let n = read_u32(at + 4) as usize;
        let element = match typ {
            3 => 2,
            4 => 4,
            _ => 1,
        };
        let data_at = if element * n <= 4 {
            at + 8
        } else {
            read_u32(at + 8) as usize
        };
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            out.push(if element == 2 {
                read_u16(data_at + k * 2) as u32
            } else if element == 4 {
                read_u32(data_at + k * 4)
            } else {
                bytes[data_at + k] as u32
            });
        }
        return Some(out);
    }
    None
}

fn tiff_tag(bytes: &[u8], tag: u16) -> Option<Vec<u32>> {
    let ifd = if bytes.starts_with(b"II") {
        u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize
    } else {
        u32::from_be_bytes(bytes.get(4..8)?.try_into().ok()?) as usize
    };
    tiff_ifd_tag(bytes, ifd, tag)
}

#[test]
fn save_is_deterministic() {
    let doc = rgba_doc(&[10, 20, 30, 255, 40, 50, 60, 255], 2, 1);
    let r = raster(&doc);
    let a = write_png(&r, &opts()).unwrap();
    let b = write_png(&r, &opts()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn save_atomic_failure() {
    let path = std::path::Path::new("/definitely-missing-dir/out.png");
    assert!(write_atomic(path, b"x").is_err());
}

#[test]
fn save_atomic_set() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let path = std::path::Path::new("/definitely-missing-dir/art.eps");
    assert!(eps::write_dcs(path, &raster(&doc), &opts(), &doc, None).is_err());
}

#[test]
fn save_atomic_set_rename() {
    let dir = std::env::temp_dir().join(format!("pictura-set-rename-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let bad = dir.join("missing/art.C");
    let files = vec![(path.clone(), b"master".to_vec()), (bad, b"plate".to_vec())];
    assert!(eps::write_set(files).is_err());
    assert!(!path.exists(), "the master was rolled back");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dcs_set_refuses_directory_target() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs-dir-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    std::fs::write(&path, b"old master").unwrap();
    std::fs::create_dir_all(dir.join("art.C")).unwrap();
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.dcs_version = 1;
    o.dcs_preview = 0;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"old master");
    assert!(dir.join("art.C").is_dir());
    for ext in ["M", "Y", "K"] {
        assert!(!dir.join(format!("art.{ext}")).exists());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn before_write_guard_aborts_before_replacing() {
    let dir = std::env::temp_dir().join(format!("pictura-guard-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let called = std::cell::Cell::new(false);
    let guard = || {
        called.set(true);
        false
    };
    let png = dir.join("guard.png");
    std::fs::write(&png, b"old png").unwrap();
    assert!(save_document_with_jpeg_guarded(
        SaveKind::Png,
        &doc,
        &opts(),
        &png,
        None,
        Some(&guard)
    )
    .is_err());
    assert!(called.get());
    assert_eq!(std::fs::read(&png).unwrap(), b"old png");

    called.set(false);
    let dcs = dir.join("set.eps");
    std::fs::write(&dcs, b"old set").unwrap();
    let mut o = opts();
    o.dcs_version = 1;
    o.dcs_preview = 0;
    assert!(
        save_document_with_jpeg_guarded(SaveKind::Dcs, &doc, &o, &dcs, None, Some(&guard)).is_err()
    );
    assert!(called.get());
    assert_eq!(std::fs::read(&dcs).unwrap(), b"old set");
    for ext in ["C", "M", "Y", "K"] {
        assert!(!dir.join(format!("set.{ext}")).exists());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bmp_depth_headers() {
    let doc = rgba_doc(
        &[
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ],
        2,
        2,
    );
    let r = raster(&doc);
    for depth in [1u8, 4, 8, 16, 24, 32] {
        let mut o = opts();
        o.bmp_depth = depth;
        let bytes = write_bmp(&r, &o).unwrap();
        assert_eq!(u16_at(&bytes, 28) as u8, depth);
    }
}

#[test]
fn bmp_rle_rules() {
    let doc = rgba_doc(&[1, 2, 3, 255, 4, 5, 6, 255], 2, 2);
    let r = raster(&doc);
    let mut o = opts();
    o.bmp_depth = 8;
    o.bmp_rle = true;
    let bytes = write_bmp(&r, &o).unwrap();
    assert_eq!(u32_at(&bytes, 30), 1);
    o.bmp_depth = 24;
    assert!(write_bmp(&r, &o).is_err());
}

#[test]
fn bmp_flip_rows() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.bmp_depth = 24;
    o.bmp_flip = true;
    let bytes = write_bmp(&raster(&doc), &o).unwrap();
    assert!(i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]) < 0);
}

#[test]
fn bmp_os2_header() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.bmp_os2 = true;
    o.bmp_depth = 24;
    let bytes = write_bmp(&raster(&doc), &o).unwrap();
    assert_eq!(u32_at(&bytes, 14), 12);
}

#[test]
fn bmp_os2_limits() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.bmp_os2 = true;
    o.bmp_flip = true;
    assert!(write_bmp(&raster(&doc), &o).is_err());
    o.bmp_flip = false;
    o.bmp_depth = 32;
    assert!(write_bmp(&raster(&doc), &o).is_err());
}

#[test]
fn bmp_advanced_16_layouts() {
    let doc = rgba_doc(&[255, 0, 0, 255], 1, 1);
    let mut o = opts();
    o.bmp_depth = 16;
    for layout in 0..3 {
        o.bmp_advanced = layout;
        assert!(write_bmp(&raster(&doc), &o).is_ok());
    }
}

#[test]
fn bmp_32_alpha() {
    let doc = rgba_doc(&[1, 2, 3, 7], 1, 1);
    let mut o = opts();
    o.bmp_depth = 32;
    let bytes = write_bmp(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[54 + 3], 7);
}

#[test]
fn png_alpha_round_trip() {
    let doc = rgba_doc(&[1, 2, 3, 4], 1, 1);
    let bytes = write_png(&raster(&doc), &opts()).unwrap();
    let decoded = crate::png::decode_rgba(&bytes).unwrap();
    assert_eq!(decoded.rgba[3], 4);
}

#[test]
fn png_nonuniform_pixels() {
    // The composite is planar; a multi-pixel, non-uniform image catches a
    // decode that mistakes the planes for interleaved samples.
    let rgba = [
        10, 20, 30, 200, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
    ];
    let doc = rgba_doc(&rgba, 2, 2);
    let bytes = write_png(&raster(&doc), &opts()).unwrap();
    let decoded = crate::png::decode_rgba(&bytes).unwrap();
    assert_eq!(decoded.rgba, rgba);
}

#[test]
fn png_adam7_round_trip() {
    let doc = rgba_doc(&[1, 2, 3, 255, 4, 5, 6, 255], 2, 1);
    let mut o = opts();
    o.png_interlace = true;
    let bytes = write_png(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[28], 1);
    o.png_interlace = false;
    let bytes = write_png(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[28], 0);
}

#[test]
fn png_compression_levels() {
    let mut rgba = Vec::new();
    for i in 0..64u8 {
        rgba.extend_from_slice(&[i, i, i, 255]);
    }
    let doc = rgba_doc(&rgba, 8, 8);
    let mut o = opts();
    o.png_compression = 0;
    let a = write_png(&raster(&doc), &o).unwrap();
    o.png_compression = 9;
    let b = write_png(&raster(&doc), &o).unwrap();
    assert_ne!(a.len(), b.len());
}

fn tiff_doc() -> Document {
    rgba_doc(
        &[
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ],
        2,
        2,
    )
}

#[test]
fn tiff_compression_tags() {
    let doc = tiff_doc();
    let jpeg = |_samples: &[u8], _w: u32, _h: u32, _c: u8, _q: u8| Some(JPEG.to_vec());
    for (code, tag) in [(0u8, 1u32), (1, 32773), (2, 5), (3, 8), (4, 7)] {
        let mut o = opts();
        o.tiff_compression = code;
        if code == 4 {
            o.tiff_transparency = false;
        }
        let bytes = tiff::write(&doc, &raster(&doc), &o, Some(&jpeg)).unwrap();
        assert_eq!(tiff_tag(&bytes, 259).unwrap()[0], tag);
        if code == 4 {
            assert_eq!(tiff_tag(&bytes, 262).unwrap()[0], 6, "YCbCr JPEG");
            assert_eq!(tiff_tag(&bytes, 530).unwrap(), vec![2, 2]);
            let offset = tiff_tag(&bytes, 273).unwrap()[0] as usize;
            let count = tiff_tag(&bytes, 279).unwrap()[0] as usize;
            let strip = &bytes[offset..offset + count];
            assert!(strip.starts_with(&[0xFF, 0xD8]));
            assert!(strip.ends_with(&[0xFF, 0xD9]));
        }
    }
}

#[test]
fn tiff_jpeg_limits() {
    let mut o = opts();
    o.tiff_compression = 4;
    let alpha = rgba_doc(&[1, 2, 3, 4], 1, 1);
    assert!(tiff::write(&alpha, &raster(&alpha), &o, None).is_err());
    let mut depth = rgba_doc(&[1, 2, 3, 255], 1, 1);
    depth.source_depth = Some(BitDepth::Sixteen);
    assert!(tiff::write(&depth, &raster(&depth), &o, None).is_err());
    let cmyk = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    assert!(tiff::write(&cmyk, &raster(&cmyk), &o, None).is_err());
    let wide = rgba_doc(&vec![0u8; 30_001 * 4], 30_001, 1);
    assert!(tiff::write(&wide, &raster(&wide), &o, None).is_err());
    let opaque = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut planar = o.clone();
    planar.tiff_pixel_order = true;
    assert!(tiff::write(&opaque, &raster(&opaque), &planar, None).is_err());
}

#[test]
fn tiff_planar_strips() {
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_pixel_order = true;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert_eq!(tiff_tag(&bytes, 284).unwrap()[0], 2);
    assert_eq!(tiff_tag(&bytes, 273).unwrap().len(), 3);
}

#[test]
fn tiff_byte_order_honors_option_for_flat_files() {
    let doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let mut o = opts();
    o.tiff_byte_order_mac = true;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert!(bytes.starts_with(b"MM"));
    o.tiff_byte_order_mac = false;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert!(bytes.starts_with(b"II"));
}

#[test]
fn layered_tiff_is_big_endian() {
    // The PSD layer payload is big-endian; tag 37724 and the container must
    // agree, so a layered TIFF ignores the IBM PC option.
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_byte_order_mac = false;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert!(bytes.starts_with(b"MM"));
    assert!(tiff_tag(&bytes, 37724).is_some());
}

#[test]
fn tiff_predictor_tag() {
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_compression = 3;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert_eq!(tiff_tag(&bytes, 317).unwrap()[0], 2);
}

#[test]
fn tiff_alpha_flatten() {
    let doc = rgba_doc(&[1, 2, 3, 4], 1, 1);
    let mut o = opts();
    o.tiff_transparency = true;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert_eq!(tiff_tag(&bytes, 338).unwrap()[0], 2);
    o.tiff_transparency = false;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert!(tiff_tag(&bytes, 338).is_none());
}

#[test]
fn tiff_metadata_tags() {
    let mut doc = tiff_doc();
    let iptc = crate::frame_image_resource(crate::IPTC_NAA, "", b"iptc");
    let xmp = crate::frame_image_resource(crate::XMP_METADATA, "", b"<x/>");
    let mut exif = b"II*\0\x08\0\0\0".to_vec();
    exif.extend_from_slice(&0u16.to_le_bytes());
    exif.extend_from_slice(&0u32.to_le_bytes());
    let exif = crate::frame_image_resource(crate::EXIF_DATA_1, "Exif", &exif);
    doc.image_resources = crate::encode_image_resources(&[iptc, xmp, exif]).into();
    let mut options = opts();
    options.icc = Some(b"icc".to_vec());
    let bytes = tiff::write(&doc, &raster(&doc), &options, None).unwrap();
    assert!(tiff_tag(&bytes, 33723).is_some());
    assert!(tiff_tag(&bytes, 700).is_some());
    assert!(tiff_tag(&bytes, 34377).is_some());
    assert!(tiff_tag(&bytes, 34665).is_some());
    assert_eq!(tiff_tag(&bytes, 34675).unwrap().len(), 3);
    let encode = jpeg();
    let mut jpeg_options = opts();
    jpeg_options.tiff_compression = 4;
    jpeg_options.tiff_transparency = false;
    let bytes = tiff::write(&doc, &raster(&doc), &jpeg_options, Some(&encode)).unwrap();
    assert!(tiff_tag(&bytes, 33723).is_some());
    assert!(tiff_tag(&bytes, 700).is_some());
}

#[test]
fn tiff_mode_photometric() {
    for (mode, tag) in [
        (ColorMode::Grayscale, 1u32),
        (ColorMode::Rgb, 2),
        (ColorMode::Cmyk, 5),
        (ColorMode::Lab, 8),
    ] {
        let mut doc = Document::new(1, 1, mode, BitDepth::Eight);
        doc.composite = pictura_core::PixelBuffer::new(1, 1, mode.color_channels());
        let bytes = tiff::write(&doc, &raster(&doc), &opts(), None).unwrap();
        assert_eq!(tiff_tag(&bytes, 262).unwrap()[0], tag);
    }
    let mut doc = Document::new(1, 1, ColorMode::Indexed, BitDepth::Eight);
    doc.source_mode = Some(ColorMode::Indexed);
    doc.source_palette = Some([0; 768]);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Eight,
        width: 1,
        height: 1,
        samples: Samples::U8(vec![0].into()),
    });
    let bytes = tiff::write(&doc, &raster(&doc), &opts(), None).unwrap();
    assert_eq!(tiff_tag(&bytes, 262).unwrap()[0], 3);
    assert!(tiff_tag(&bytes, 320).is_some());
}

#[test]
fn tiff_pyramid_subifds() {
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_compression = 0;
    o.tiff_pyramid = true;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    let offsets = tiff_tag(&bytes, 330).unwrap();
    assert_eq!(offsets.len(), 1);
    let sub = offsets[0] as usize;
    assert_eq!(tiff_ifd_tag(&bytes, sub, 256).unwrap()[0], 1);
    assert_eq!(tiff_ifd_tag(&bytes, sub, 257).unwrap()[0], 1);
    let strip = tiff_ifd_tag(&bytes, sub, 273).unwrap()[0] as usize;
    let count = tiff_ifd_tag(&bytes, sub, 279).unwrap()[0] as usize;
    assert_eq!(count, 3);
    assert!(bytes.len() > strip + count);
}

#[test]
fn tiff_layer_section_round_trip() {
    let doc = tiff_doc();
    let bytes = tiff::write(&doc, &raster(&doc), &opts(), None).unwrap();
    let tag = tiff_tag(&bytes, 37724).expect("layer tag");
    let data: Vec<u8> = tag.iter().map(|value| *value as u8).collect();
    assert!(
        data.starts_with(b"Adobe Photoshop Document Data Block\0"),
        "magic"
    );
    let block = &data[b"Adobe Photoshop Document Data Block\0".len()..];
    assert_eq!(&block[..4], b"8BIM");
    assert_eq!(&block[4..8], b"Layr");
    let length = u32::from_be_bytes(block[8..12].try_into().unwrap()) as usize;
    assert_eq!((block.len() - 12) % 4, 0);
    assert!(length > 0 && 12 + length <= block.len());
    // The layer section carries the document's single layer (from_rgba).
    let count = i16::from_be_bytes([block[12], block[13]]);
    assert_eq!(count, 1);
}

#[test]
fn tiff_layer_compression() {
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_layer_compression = 0;
    let a = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    o.tiff_layer_compression = 1;
    let b = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert_ne!(tiff_tag(&a, 37724), tiff_tag(&b, 37724));
}

#[test]
fn tiff_discard_layers() {
    let doc = tiff_doc();
    let mut o = opts();
    o.tiff_layer_compression = 2;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    assert!(tiff_tag(&bytes, 37724).is_none());
}

#[test]
fn gif89a_structure() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let bytes = write_gif(&raster(&doc), &opts()).unwrap();
    assert!(bytes.starts_with(b"GIF89a"));
    assert_eq!(*bytes.last().unwrap(), 0x3b);
}

#[test]
fn gif_indexed_plane() {
    let mut doc = Document::new(1, 1, ColorMode::Indexed, BitDepth::Eight);
    doc.source_mode = Some(ColorMode::Indexed);
    doc.source_palette = Some([0; 768]);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Eight,
        width: 1,
        height: 1,
        samples: Samples::U8(vec![0].into()),
    });
    let bytes = write_gif(&raster(&doc), &opts()).unwrap();
    assert!(bytes.starts_with(b"GIF89a"));
}

#[test]
fn gif_interlace_flag() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.gif_interlaced = true;
    let bytes = write_gif(&raster(&doc), &o).unwrap();
    assert!(bytes.contains(&0x40));
}

#[test]
fn gif_transparent_index() {
    let indexed = crate::Indexed {
        width: 1,
        height: 1,
        palette: vec![[0, 0, 0, 0], [255, 255, 255, 255]],
        indices: vec![0],
    };
    let bytes = crate::encode_gif(&indexed, false);
    assert!(bytes.windows(2).any(|w| w == [0x21, 0xF9]));
}

#[test]
fn tga_depth_decode() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    for depth in [16u8, 24, 32] {
        let mut o = opts();
        o.tga_depth = depth;
        let bytes = write_tga(&raster(&doc), &o).unwrap();
        assert_eq!(bytes[16], depth);
    }
}

#[test]
fn tga_rle_decode() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.tga_rle = true;
    let bytes = write_tga(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[2], 10);
    o.tga_rle = false;
    let bytes = write_tga(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[2], 2);
}

#[test]
fn tga_origin() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let bytes = write_tga(&raster(&doc), &opts()).unwrap();
    assert_ne!(bytes[17] & 0x20, 0);
}

#[test]
fn raw_header_bytes() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.raw_header = 16;
    let bytes = write_raw(&raster(&doc), &o).unwrap();
    assert_eq!(&bytes[..16], &[0u8; 16][..]);
    o.raw_header = 0;
    let bytes = write_raw(&raster(&doc), &o).unwrap();
    assert_eq!(bytes.len(), 3);
}

#[test]
fn raw_interleave_planes() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let r = raster(&doc);
    let mut o = opts();
    o.raw_interleaved = true;
    assert_eq!(write_raw(&r, &o).unwrap(), vec![1, 2, 3]);
    o.raw_interleaved = false;
    assert_eq!(write_raw(&r, &o).unwrap(), vec![1, 2, 3]);
}

#[test]
fn raw_mode_channels() {
    for (mode, channels) in [
        (ColorMode::Grayscale, 1usize),
        (ColorMode::Rgb, 3),
        (ColorMode::Cmyk, 4),
        (ColorMode::Lab, 3),
    ] {
        let doc = Document::new(1, 1, mode, BitDepth::Eight);
        let bytes = write_raw(&raster(&doc), &opts()).unwrap();
        assert_eq!(bytes.len(), channels);
    }
}

#[test]
fn raw_refuses_multichannel_without_planes() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.mode = ColorMode::Multichannel;
    assert!(write_raw(&raster(&doc), &opts()).is_err());
}

#[test]
fn raw_eight_bit_size() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.raw_header = 4;
    assert_eq!(write_raw(&raster(&doc), &o).unwrap().len(), 4 + 3);
}

#[test]
fn cineon_header_magic() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let bytes = write_cineon(&raster(&doc), &opts()).unwrap();
    assert_eq!(&bytes[..4], &0x802A_5FD7u32.to_be_bytes()[..]);
}

#[test]
fn cineon_10bit_packing() {
    let doc = rgba_doc(&[1, 2, 3, 255, 4, 5, 6, 255], 2, 1);
    let bytes = write_cineon(&raster(&doc), &opts()).unwrap();
    assert_eq!(bytes.len(), 1024 + 2 * 4);
}

#[test]
fn cineon_deterministic_mapping() {
    let doc = rgba_doc(&[0, 0, 0, 255], 1, 1);
    let a = write_cineon(&raster(&doc), &opts()).unwrap();
    let b = write_cineon(&raster(&doc), &opts()).unwrap();
    assert_eq!(a, b);
    assert_eq!(&a[1024..1028], &[0u8, 0, 0, 0][..]);
}

#[test]
fn cineon_no_alpha() {
    let doc = rgba_doc(&[1, 2, 3, 4], 1, 1);
    let bytes = write_cineon(&raster(&doc), &opts()).unwrap();
    assert_eq!(bytes[193], 3);
    assert_eq!(bytes.len(), 1028);
}

#[test]
fn eps_dsc_header() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.eps_preview = 0;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    assert!(bytes.starts_with(b"%!PS-Adobe-3.0 EPSF-3.0"));
    assert!(String::from_utf8_lossy(&bytes).contains("%%BoundingBox: 0 0 1 1"));
}

#[test]
fn eps_preview_options() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    for preview in 1..=2 {
        let mut o = opts();
        o.eps_preview = preview;
        let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
        assert!(bytes.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]));
        let block = preview_block(&bytes).expect("preview block");
        assert!(block.starts_with(b"II") || block.starts_with(b"MM"));
        assert_eq!(u32_at(&bytes, 4), 30, "DOS EPS header size");
        assert_eq!(u32_at(&bytes, 12), 0, "no WMF preview");
        assert!(crate::eps::postscript_section(&bytes).is_some());
    }
}

#[test]
fn eps_encodings() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let encode = jpeg();
    for encoding in 0..4 {
        let mut o = opts();
        o.eps_preview = 0;
        o.eps_encoding = encoding;
        let bytes = eps::write(&raster(&doc), &o, &doc, Some(&encode)).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        match encoding {
            0 => assert!(text.contains("ASCIIHexDecode")),
            1 => assert!(text.contains("ASCII85Decode")),
            2 => {
                assert!(!text.contains("ASCIIHexDecode"));
                assert!(!text.contains("ASCII85Decode"));
                assert!(!text.contains("DCTDecode"));
            }
            3 => {
                assert!(text.contains("DCTDecode"));
                assert!(bytes.windows(2).any(|w| w == [0xFF, 0xD8]));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn eps_clipping_path() {
    let doc = with_clipping_path(rgba_doc(&[1, 2, 3, 255], 3, 3), "cutout", triangle());
    let mut o = opts();
    o.eps_preview = 0;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("newpath"));
    assert!(text.contains("clip"));
    assert!(text.contains("moveto"));

    // A Work Path that was never designated as the clipping path must not
    // silently crop the artwork.
    let mut work_only = rgba_doc(&[1, 2, 3, 255], 3, 3);
    work_only.work_path = triangle();
    let bytes = eps::write(&raster(&work_only), &o, &work_only, None).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("clip"));
}

#[test]
fn eps_vector_data() {
    let mut doc = rgba_doc(&[1, 2, 3, 255], 3, 3);
    doc.work_path = triangle();
    let mut o = opts();
    o.eps_preview = 0;
    o.eps_vector_data = true;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("curveto") || text.contains("lineto"));
    assert!(text.contains("closepath"));
    assert!(text.contains("stroke"));
}

#[test]
fn eps_color_spaces() {
    for (mode, space) in [
        (ColorMode::Grayscale, "DeviceGray"),
        (ColorMode::Rgb, "DeviceRGB"),
        (ColorMode::Cmyk, "DeviceCMYK"),
    ] {
        let doc = Document::new(1, 1, mode, BitDepth::Eight);
        let mut o = opts();
        o.eps_preview = 0;
        let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains(space));
    }
}

#[test]
fn dcs_1_plate_files() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    let mut o = opts();
    o.dcs_version = 1;
    o.dcs_preview = 0;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_ok());
    for ext in ["eps", "C", "M", "Y", "K"] {
        assert!(dir.join(format!("art.{ext}")).exists());
    }
    let master = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(master);
    assert!(text.contains("%%DocumentProcessColors: Cyan Magenta Yellow Black"));
    for name in ["Cyan", "Magenta", "Yellow", "Black"] {
        assert!(
            text.contains(&format!("%%{name}Plate: ")),
            "master names the {name} plate"
        );
    }
    for ext in ["C", "M", "Y", "K"] {
        let plate = std::fs::read(dir.join(format!("art.{ext}"))).unwrap();
        let ps = crate::eps::postscript_section(&plate).expect("plate PostScript");
        assert!(String::from_utf8_lossy(ps).contains(" image"));
        assert!(!String::from_utf8_lossy(ps).contains("%%PlateFile:"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dcs_2_single_file() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs2-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    let mut o = opts();
    o.dcs_version = 2;
    o.dcs_preview = 0;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_ok());
    assert!(path.exists());
    assert!(!dir.join("art.C").exists());
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("DeviceCMYK"));
    assert!(!text.contains("%%Plate:"));
    let records: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("%%PlateFile:"))
        .collect();
    assert_eq!(records.len(), 4);
    let mut expected = 0usize;
    for (index, record) in records.iter().enumerate() {
        let fields: Vec<&str> = record.split_whitespace().collect();
        assert_eq!(fields.len(), 5, "{record}");
        let offset = fields[3]
            .strip_prefix('#')
            .and_then(|value| value.parse::<usize>().ok())
            .expect("plate offset");
        let size = fields[4].parse::<usize>().expect("plate size");
        if index == 0 {
            expected = offset;
        }
        assert_eq!(offset, expected, "plate records are contiguous");
        assert!(
            bytes[offset..].starts_with(b"%!PS-Adobe-3.0 EPSF-3.0"),
            "plate {index} starts its own EPS"
        );
        expected = offset + size;
    }
    assert_eq!(expected, bytes.len(), "the last plate ends the file");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dcs_process_only() {
    let doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    let mut o = opts();
    o.dcs_version = 2;
    o.eps_preview = 0;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("DeviceCMYK"));
}

#[test]
fn dcs_preview() {
    let dir = std::env::temp_dir().join(format!("pictura-dcsprev-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    let mut o = opts();
    o.dcs_version = 2;
    o.dcs_preview = 2;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_ok());
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]));
    assert!(preview_block(&bytes)
        .expect("DCS preview")
        .starts_with(b"II"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_refuses_invalid() {
    let doc = Document::new(0, 0, ColorMode::Rgb, BitDepth::Eight);
    assert!(Raster::from_document(&doc).is_err());
    let mut short = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    short.composite.data = vec![0u8; 3].into();
    assert!(Raster::from_document(&short).is_err());
}

#[test]
fn dcs_encode_failure_leaves_no_set() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs-encode-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.dcs_version = 1;
    o.eps_encoding = 3;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_err());
    for suffix in ["eps", "C", "M", "Y", "K"] {
        assert!(!dir.join(format!("art.{suffix}")).exists());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_refuses_mode() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.tga_depth = 17;
    assert!(write_tga(&raster(&doc), &o).is_err());
}

#[test]
fn tiff_ifd_is_sorted_and_aligned() {
    let doc = tiff_doc();
    let bytes = tiff::write(&doc, &raster(&doc), &opts(), None).unwrap();
    let little = bytes.starts_with(b"II");
    let read_u32 = |at: usize| {
        let b = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
        if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        }
    };
    let ifd = read_u32(4) as usize;
    assert_eq!(ifd % 2, 0, "IFD must be word aligned");
    let count = if little {
        u16::from_le_bytes([bytes[ifd], bytes[ifd + 1]])
    } else {
        u16::from_be_bytes([bytes[ifd], bytes[ifd + 1]])
    } as usize;
    let mut previous = 0;
    for i in 0..count {
        let at = ifd + 2 + i * 12;
        let tag = if little {
            u16::from_le_bytes([bytes[at], bytes[at + 1]])
        } else {
            u16::from_be_bytes([bytes[at], bytes[at + 1]])
        };
        assert!(
            tag > previous,
            "IFD tags must ascend: {previous} then {tag}"
        );
        previous = tag;
    }
}

#[test]
fn bmp_rle4_and_rle8_headers() {
    let doc = rgba_doc(&[1, 2, 3, 255, 4, 5, 6, 255], 2, 1);
    for depth in [4u8, 8] {
        let mut o = opts();
        o.bmp_depth = depth;
        o.bmp_rle = true;
        let bytes = write_bmp(&raster(&doc), &o).unwrap();
        assert_eq!(u32_at(&bytes, 30), if depth == 8 { 1 } else { 2 });
    }
}

#[test]
fn bmp_advanced_16_has_masks() {
    let doc = rgba_doc(&[255, 0, 0, 255], 1, 1);
    let mut o = opts();
    o.bmp_depth = 16;
    o.bmp_advanced = 2;
    let bytes = write_bmp(&raster(&doc), &o).unwrap();
    assert_eq!(u32_at(&bytes, 30), 3, "BI_BITFIELDS");
    assert_eq!(u32_at(&bytes, 54), 0xF800);
    assert_eq!(u32_at(&bytes, 58), 0x07E0);
    assert_eq!(u32_at(&bytes, 62), 0x001F);
}

#[test]
fn tga_16_declares_attribute_bits() {
    let doc = rgba_doc(&[1, 2, 3, 255], 1, 1);
    let mut o = opts();
    o.tga_depth = 16;
    let bytes = write_tga(&raster(&doc), &o).unwrap();
    assert_eq!(bytes[17] & 0x0F, 1);
}

#[test]
fn gif_writer_marks_transparency() {
    let doc = rgba_doc(&[1, 2, 3, 0, 4, 5, 6, 255], 2, 1);
    let bytes = write_gif(&raster(&doc), &opts()).unwrap();
    assert!(bytes.windows(2).any(|window| window == [0x21, 0xF9]));
}

#[test]
fn cineon_header_fields_are_documented_offsets() {
    let doc = rgba_doc(&[1, 2, 3, 255], 2, 1);
    let bytes = write_cineon(&raster(&doc), &opts()).unwrap();
    assert_eq!(u32_be(&bytes, 4), 1024);
    assert_eq!(u32_be(&bytes, 8), 1024);
    assert_eq!(u32_be(&bytes, 20), 1024 + 2 * 4);
    assert_eq!(bytes[193], 3);
    assert_eq!(bytes[198], 10);
    assert_eq!(u32_be(&bytes, 200), 2);
    assert_eq!(u32_be(&bytes, 204), 1);
    assert_eq!(bytes[681], 5);
}

#[test]
fn dcs_rgb_to_cmyk_uses_process_inks() {
    let doc = rgba_doc(&[255, 255, 255, 255], 1, 1);
    assert_eq!(eps::dcs_cmyk(&raster(&doc)), [0, 0, 0, 0]);
    let doc = rgba_doc(&[255, 0, 0, 255], 1, 1);
    assert_eq!(eps::dcs_cmyk(&raster(&doc)), [0, 255, 255, 0]);
}

#[test]
fn dcs_failed_set_preserves_previous_files() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs-preserve-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let existing = dir.join("keep.eps");
    std::fs::write(&existing, b"old").unwrap();
    let bad = dir.join("missing/plate.eps");
    let files = vec![(existing.clone(), b"new".to_vec()), (bad, b"bad".to_vec())];
    assert!(eps::write_set(files).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), b"old");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tiff_transparency_off_composites_white() {
    let doc = rgba_doc(&[255, 0, 0, 128], 1, 1);
    let mut o = opts();
    o.tiff_compression = 0;
    o.tiff_transparency = false;
    let bytes = tiff::write(&doc, &raster(&doc), &o, None).unwrap();
    let offset = tiff_tag(&bytes, 273).unwrap()[0] as usize;
    assert_eq!(&bytes[offset..offset + 3], &[255, 127, 127]);
}

#[test]
fn bmp_and_tga_without_alpha_flatten_white() {
    let doc = rgba_doc(&[255, 0, 0, 0], 1, 1);
    let r = raster(&doc);
    let mut bmp = opts();
    bmp.bmp_depth = 24;
    let bytes = write_bmp(&r, &bmp).unwrap();
    let offset = u32_at(&bytes, 10) as usize;
    assert_eq!(&bytes[offset..offset + 3], &[255, 255, 255]);
    let mut tga = opts();
    tga.tga_depth = 24;
    let bytes = write_tga(&r, &tga).unwrap();
    assert_eq!(&bytes[18..21], &[255, 255, 255]);
}

#[test]
fn eps_bounding_box_uses_document_resolution() {
    let mut doc = rgba_doc(&[1, 2, 3, 255], 4, 2);
    crate::set_document_resolution(
        &mut doc,
        crate::Resolution {
            ppi: 300.0,
            per_cm: false,
        },
    );
    let mut o = opts();
    o.eps_preview = 0;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("%%BoundingBox: 0 0 1 0"), "{text}");
}

#[test]
fn eps_image_and_paths_share_the_point_scale() {
    let mut doc = rgba_doc(&[1, 2, 3, 255], 4, 2);
    crate::set_document_resolution(
        &mut doc,
        crate::Resolution {
            ppi: 300.0,
            per_cm: false,
        },
    );
    doc.work_path = triangle();
    let mut o = opts();
    o.eps_preview = 0;
    o.eps_vector_data = true;
    let bytes = eps::write(&raster(&doc), &o, &doc, None).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("/ImageMatrix [0.96 0 0 -0.48 0 0.48]"),
        "raster is placed in the point-sized bounding box: {text}"
    );
    assert!(
        text.contains("0.24 0.24 moveto"),
        "vector coordinates use the same pixel-to-point scale: {text}"
    );
}

#[test]
fn dcs_master_is_cmyk() {
    let dir = std::env::temp_dir().join(format!("pictura-dcs-cmyk-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("art.eps");
    let doc = rgba_doc(&[255, 0, 0, 255], 1, 1);
    let mut o = opts();
    o.dcs_version = 1;
    o.dcs_preview = 0;
    o.eps_preview = 0;
    assert!(eps::write_dcs(&path, &raster(&doc), &o, &doc, None).is_ok());
    let master = std::fs::read(&path).unwrap();
    let ps = crate::eps::postscript_section(&master).unwrap();
    assert!(String::from_utf8_lossy(ps).contains("DeviceCMYK"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tiff_transparency_off_composites_cmyk_white() {
    let doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    let raster = Raster {
        width: 1,
        height: 1,
        mode: ColorMode::Cmyk,
        rgba: vec![0, 0, 0, 128],
        rgb: vec![0; 3],
        gray: vec![0],
        cmyk: vec![200, 100, 50, 40],
        lab: Vec::new(),
        alpha: true,
        palette: None,
        indices: None,
    };
    let mut o = opts();
    o.tiff_compression = 0;
    o.tiff_transparency = false;
    let bytes = tiff::write(&doc, &raster, &o, None).unwrap();
    let offset = tiff_tag(&bytes, 273).unwrap()[0] as usize;
    let count = tiff_tag(&bytes, 279).unwrap()[0] as usize;
    assert_eq!(&bytes[offset..offset + count], &[100, 50, 25, 20]);
}

#[test]
fn tiff_transparency_off_composites_lab_white() {
    let doc = Document::new(1, 1, ColorMode::Lab, BitDepth::Eight);
    let raster = Raster {
        width: 1,
        height: 1,
        mode: ColorMode::Lab,
        rgba: vec![0, 0, 0, 128],
        rgb: vec![0; 3],
        gray: vec![0],
        cmyk: Vec::new(),
        lab: vec![100, 200, 50],
        alpha: true,
        palette: None,
        indices: None,
    };
    let mut o = opts();
    o.tiff_compression = 0;
    o.tiff_transparency = false;
    let bytes = tiff::write(&doc, &raster, &o, None).unwrap();
    let offset = tiff_tag(&bytes, 273).unwrap()[0] as usize;
    let count = tiff_tag(&bytes, 279).unwrap()[0] as usize;
    assert_eq!(&bytes[offset..offset + count], &[177, 164, 89]);
}
