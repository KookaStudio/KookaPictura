//! `pictura_codec::pdf` tests: structure checks over the written bytes here,
//! external-tool oracles (`pdftoppm`, `qpdf`) in `oracle.rs`.

mod oracle;
mod standards;

use std::io::Read as _;

use pictura_core::{BitDepth, ColorMode, Document, MarkerKind, PixelBuffer, Samples, SourcePlanes};

use super::*;
use crate::image_resources::{set_document_resolution, Resolution};

pub(super) fn env() -> PdfEnv<'static> {
    PdfEnv {
        jpeg: &|req: &JpegRequest| {
            // Engine tests have no Qt: a marker the structure checks can see.
            let mut fake = b"\xff\xd8FAKEJPEG".to_vec();
            fake.push(req.components);
            fake.push(req.quality);
            Some(fake)
        },
        now: 1_792_240_496,
        entropy: [9; 32],
        title: "test".into(),
        live_text: None,
    }
}

/// An RGB document whose pixel `i` is `(i * 7, i * 13, i * 29)` (mod 256),
/// opaque except where `transparent` names a pixel, at `ppi`.
pub(super) fn rgb_doc(w: u32, h: u32, ppi: f64, transparent: Option<usize>) -> Document {
    let mut rgba = Vec::new();
    for i in 0..(w * h) as usize {
        let alpha = if Some(i) == transparent { 0 } else { 255 };
        rgba.extend([(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, alpha]);
    }
    let mut doc = Document::from_rgba("layer", w, h, &rgba);
    set_document_resolution(&mut doc, Resolution { ppi, per_cm: false });
    doc
}

/// Write `doc` with its own composite.
pub(super) fn write(doc: &Document, options: &PdfOptions) -> Vec<u8> {
    write_pdf(doc, &doc.composite, options, &env()).expect("writes")
}

pub(super) fn text(pdf: &[u8]) -> String {
    String::from_utf8_lossy(pdf).into_owned()
}

/// Every stream object's dictionary text and raw data, in file order.
pub(super) fn streams(pdf: &[u8]) -> Vec<(String, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = find(pdf, b" 0 obj\n", at) {
        let dict_start = start + 7;
        let Some(end) = find(pdf, b"endobj", dict_start) else {
            break;
        };
        at = end;
        if let Some(stream_at) = find(&pdf[..end], b"\nstream\n", dict_start) {
            let data_start = stream_at + 8;
            if let Some(data_end) = find(pdf, b"\nendstream", data_start) {
                out.push((
                    text(&pdf[dict_start..stream_at]),
                    &pdf[data_start..data_end],
                ));
            }
        }
    }
    out
}

/// The first stream whose dictionary contains every marker.
pub(super) fn stream<'a>(pdf: &'a [u8], markers: &[&str]) -> Option<(String, &'a [u8])> {
    streams(pdf)
        .into_iter()
        .find(|(dict, _)| markers.iter().all(|m| dict.contains(m)))
}

pub(super) fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

pub(super) fn inflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .expect("valid zlib");
    out
}

/// The page image's dictionary and decoded samples (Flate or raw); the soft
/// mask (gray, written colour space first) is skipped.
pub(super) fn image(pdf: &[u8]) -> (String, Vec<u8>) {
    let (dict, data) = streams(pdf)
        .into_iter()
        .find(|(d, _)| {
            d.contains("/Subtype /Image") && !d.contains("/ColorSpace /DeviceGray/BitsPerComponent")
        })
        .expect("an image");
    let samples = if dict.contains("/FlateDecode") {
        inflate(data)
    } else {
        data.to_vec()
    };
    (dict, samples)
}

fn options() -> PdfOptions {
    PdfOptions {
        icc_profile: false,
        embed_thumbnails: false,
        preserve_editing: false,
        ..PdfOptions::default()
    }
}

pub(super) fn plain_options() -> PdfOptions {
    options()
}

#[test]
fn pdf_page_size_follows_resolution() {
    let pdf = write(&rgb_doc(600, 300, 300.0, None), &options());
    assert!(text(&pdf).contains("/MediaBox [0 0 144 72]"));
    assert!(pdf.starts_with(b"%PDF-1.5\n"));
    let pdf = write(
        &Document::from_rgba("x", 144, 72, &[255; 144 * 72 * 4]),
        &options(),
    );
    assert!(
        text(&pdf).contains("/MediaBox [0 0 144 72]"),
        "72 ppi by default"
    );
}

#[test]
fn pdf_zip_decodes_to_composite() {
    let doc = rgb_doc(5, 4, 72.0, None);
    let pdf = write(&doc, &options());
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/Filter /FlateDecode"));
    assert!(dict.contains("/ColorSpace /DeviceRGB"));
    let plane = 20;
    for i in 0..plane {
        for c in 0..3 {
            assert_eq!(samples[i * 3 + c], doc.composite.data[c * plane + i]);
        }
    }
}

#[test]
fn pdf_no_compression_writes_raw() {
    let options = PdfOptions {
        compression: Compression::None,
        ..options()
    };
    let pdf = write(&rgb_doc(5, 4, 72.0, None), &options);
    let (dict, samples) = image(&pdf);
    assert!(!dict.contains("/Filter"));
    assert_eq!(samples.len(), 5 * 4 * 3);
}

#[test]
fn pdf_gray_writes_one_component() {
    let mut doc = Document::new(4, 2, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite.data = (0..8u8).map(|v| v * 30).collect::<Vec<_>>().into();
    let pdf = write(&doc, &options());
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/ColorSpace /DeviceGray"));
    assert_eq!(samples, (0..8u8).map(|v| v * 30).collect::<Vec<_>>());
}

pub(super) fn cmyk_doc() -> Document {
    let mut doc = rgb_doc(4, 3, 72.0, None);
    doc.composite.channels = 3;
    let plane = 12;
    doc.composite.data = doc.composite.data[..3 * plane].to_vec().into();
    doc.layers = pictura_core::LayerStack::default();
    doc.source_mode = Some(ColorMode::Cmyk);
    doc
}

#[test]
fn pdf_cmyk_source_writes_device_cmyk() {
    let doc = cmyk_doc();
    let pdf = write(&doc, &options());
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/ColorSpace /DeviceCMYK"));
    // The profile-free encoding: C/M/Y inks are 255 - R/G/B and K is 0.
    assert_eq!(samples[0], 255 - doc.composite.data[0]);
    assert_eq!(samples[3], 0);
    oracle::assert_renders_like(&pdf, &doc, 72.0, 2);
}

#[test]
fn pdf_lab_source_writes_lab() {
    let mut doc = cmyk_doc();
    doc.source_mode = Some(ColorMode::Lab);
    let pdf = write(&doc, &options());
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/ColorSpace [/Lab <</WhitePoint [0.9642 1 0.8249]"));
}

pub(super) fn indexed_doc() -> Document {
    let mut palette = [0u8; 768];
    for i in 0..256 {
        palette[i] = i as u8;
        palette[256 + i] = 255 - i as u8;
        palette[512 + i] = (i * 3) as u8;
    }
    let index: Vec<u8> = (0..12u8).map(|v| v * 20).collect();
    let rgb = crate::color_mode::indexed_to_rgb(&index, &palette);
    let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = PixelBuffer {
        width: 4,
        height: 3,
        channels: 3,
        data: rgb.into(),
    };
    doc.source_mode = Some(ColorMode::Indexed);
    doc.source_palette = Some(palette);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Eight,
        width: 4,
        height: 3,
        samples: Samples::U8(index),
    });
    doc
}

#[test]
fn pdf_indexed_source_writes_palette() {
    let doc = indexed_doc();
    let pdf = write(&doc, &options());
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/ColorSpace [/Indexed /DeviceRGB 255 "));
    assert_eq!(samples, (0..12u8).map(|v| v * 20).collect::<Vec<_>>());
    oracle::assert_renders_like(&pdf, &doc, 72.0, 2);
}

pub(super) fn bitmap_doc() -> Document {
    // 10 × 3: a black diagonal band on white, packed 1 = black.
    let (w, h) = (10usize, 3usize);
    let row = w.div_ceil(8);
    let mut packed = vec![0u8; row * h];
    for y in 0..h {
        for x in 0..w {
            if (x + y) % 4 < 2 {
                packed[y * row + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    let rgb = crate::color_mode::bitmap_rows_to_rgb(&packed, w, h);
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    doc.composite = PixelBuffer {
        width: w as u32,
        height: h as u32,
        channels: 3,
        data: rgb.into(),
    };
    doc.source_mode = Some(ColorMode::Bitmap);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::One,
        width: w as u32,
        height: h as u32,
        samples: Samples::U8(packed),
    });
    doc
}

#[test]
fn pdf_bitmap_uses_ccitt_g4() {
    let doc = bitmap_doc();
    let pdf = write(&doc, &options());
    let (dict, _) = stream(&pdf, &["/Subtype /Image"]).unwrap();
    assert!(dict.contains("/BitsPerComponent 1"));
    assert!(dict.contains("/Filter /CCITTFaxDecode/DecodeParms <</K -1/Columns 10/Rows 3>>"));
    oracle::assert_renders_like(&pdf, &doc, 72.0, 0);
}

#[test]
fn pdf_multichannel_is_refused() {
    let mut doc = rgb_doc(2, 2, 72.0, None);
    doc.source_mode = Some(ColorMode::Multichannel);
    let err = write_pdf(&doc, &doc.composite, &options(), &env()).unwrap_err();
    assert!(err.to_string().contains("Multichannel"));
}

#[test]
fn pdf_icc_profile_embedding_follows_option() {
    let mut doc = rgb_doc(3, 2, 72.0, None);
    let adobe = pictura_color::Profile::adobe_rgb().to_icc();
    doc.document_icc = Some(adobe.clone());
    let tagged = write(
        &doc,
        &PdfOptions {
            icc_profile: true,
            ..options()
        },
    );
    let (dict, _) = image(&tagged);
    assert!(dict.contains("/ColorSpace [/ICCBased "));
    let (icc_dict, icc) = stream(&tagged, &["/N 3", "/Alternate /DeviceRGB"]).unwrap();
    assert!(icc_dict.contains("/FlateDecode"));
    assert_eq!(inflate(icc), adobe);
    let untagged = write(&doc, &options());
    assert!(image(&untagged).0.contains("/ColorSpace /DeviceRGB"));
}

#[test]
fn pdf_alpha_becomes_soft_mask() {
    let doc = rgb_doc(3, 2, 72.0, Some(4));
    let pdf = write(&doc, &options());
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/SMask "));
    let (_, mask) = stream(&pdf, &["/ColorSpace /DeviceGray/BitsPerComponent 8"]).unwrap();
    let mask = inflate(mask);
    assert_eq!(mask[4], 0);
    assert_eq!(mask[0], 255);
}

#[test]
fn pdf_acrobat4_flattens_alpha() {
    let doc = rgb_doc(3, 2, 72.0, Some(4));
    let options = PdfOptions {
        compatibility: Compatibility::Acrobat4,
        ..options()
    };
    let pdf = write(&doc, &options);
    assert!(pdf.starts_with(b"%PDF-1.3"));
    let (dict, samples) = image(&pdf);
    assert!(!dict.contains("/SMask"));
    assert_eq!(&samples[12..15], &[255, 255, 255]);
}

#[test]
fn pdf_downsample_above_threshold() {
    let options = PdfOptions {
        downsample: Downsample::Bicubic,
        downsample_ppi: 150.0,
        downsample_above_ppi: 225.0,
        ..options()
    };
    let pdf = write(&rgb_doc(60, 30, 300.0, None), &options);
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/Width 30/Height 15"));
    assert_eq!(samples.len(), 30 * 15 * 3);
    assert!(text(&pdf).contains("/MediaBox [0 0 14.4 7.2]"));
}

#[test]
fn pdf_downsample_below_threshold_keeps_size() {
    let options = PdfOptions {
        downsample: Downsample::Bicubic,
        downsample_ppi: 150.0,
        downsample_above_ppi: 225.0,
        ..options()
    };
    let pdf = write(&rgb_doc(60, 30, 200.0, None), &options);
    assert!(image(&pdf).0.contains("/Width 60/Height 30"));
}

fn sixteen_bit_doc() -> (Document, Vec<u16>) {
    let mut doc = rgb_doc(3, 2, 72.0, None);
    doc.layers = pictura_core::LayerStack::default();
    doc.composite.channels = 3;
    doc.composite.data = doc.composite.data[..18].to_vec().into();
    let native: Vec<u16> = doc
        .composite
        .data
        .iter()
        .enumerate()
        .map(|(i, &v)| u16::from(v) * 256 + (i as u16 % 200))
        .collect();
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 3,
        height: 2,
        samples: Samples::U16(native.clone()),
    });
    (doc, native)
}

#[test]
fn pdf_sixteen_bit_kept() {
    let (doc, native) = sixteen_bit_doc();
    let options = PdfOptions {
        convert_16_to_8: false,
        ..options()
    };
    let pdf = write(&doc, &options);
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/BitsPerComponent 16"));
    let first = u16::from_be_bytes([samples[0], samples[1]]);
    assert_eq!(first, native[0], "the retained source sample");
}

#[test]
fn pdf_sixteen_bit_forced_below_15() {
    let (doc, _) = sixteen_bit_doc();
    let mut options = PdfOptions {
        convert_16_to_8: false,
        ..options()
    };
    options.set("compatibility", "1");
    assert!(options.locked("convert16To8") && options.convert_16_to_8);
    let pdf = write(&doc, &options);
    assert!(image(&pdf).0.contains("/BitsPerComponent 8"));
}

fn two_layer_doc() -> Document {
    let mut doc = rgb_doc(4, 4, 72.0, None);
    let mut second = doc.layers[0].clone();
    second.name = "second".into();
    let _ = doc.layers.push(second);
    doc
}

#[test]
fn pdf_embeds_psd_payload() {
    let doc = two_layer_doc();
    let options = PdfOptions {
        preserve_editing: true,
        ..options()
    };
    let pdf = write(&doc, &options);
    assert!(text(&pdf).contains("/PieceInfo <</Pictura <</LastModified "));
    let psd = embedded_psd(&pdf).expect("payload");
    let reread = crate::read::read_psd(&psd).expect("a PSD");
    assert_eq!(reread.layers.len(), 2);
    assert_eq!(reread.layers[1].name, "second");
}

#[test]
fn pdf_layers_off_embeds_flat_psd() {
    let doc = two_layer_doc();
    let options = PdfOptions {
        preserve_editing: true,
        layers: false,
        ..options()
    };
    let psd = embedded_psd(&write(&doc, &options)).expect("payload");
    let reread = crate::read::read_psd(&psd).expect("a PSD");
    assert!(reread.layers.is_empty());
    assert_eq!(reread.width, 4);
}

#[test]
fn pdf_read_refuses_foreign_pdf() {
    let pdf = write(&rgb_doc(2, 2, 72.0, None), &options());
    assert_eq!(embedded_psd(&pdf), None, "no payload");
    assert_eq!(embedded_psd(b"not a pdf"), None);
    let encrypted = write(
        &rgb_doc(2, 2, 72.0, None),
        &PdfOptions {
            preserve_editing: true,
            security: Security {
                open_password: "x".into(),
                ..Security::default()
            },
            ..options()
        },
    );
    assert_eq!(embedded_psd(&encrypted), None, "encrypted");
}

fn layer_count(found: EditingPsd) -> usize {
    match found {
        EditingPsd::Psd(psd) => crate::read::read_psd(&psd).expect("a PSD").layers.len(),
        other => panic!("no PSD: {other:?}"),
    }
}

// pdf_encrypted_layers, pdf_owner_password_opens
#[test]
fn password_protected_pdf_reopens_with_its_layers() {
    let doc = two_layer_doc();
    for compatibility in Compatibility::ALL {
        for fast_web_view in [false, true] {
            let pdf = write(
                &doc,
                &PdfOptions {
                    preserve_editing: true,
                    compatibility,
                    fast_web_view,
                    security: Security {
                        open_password: "kooka".into(),
                        permissions_password: "owner".into(),
                        ..Security::default()
                    },
                    ..options()
                },
            );
            let case = format!("{compatibility:?}, fast web view {fast_web_view}");
            assert_eq!(layer_count(editing_psd(&pdf, "kooka")), 2, "{case}");
            assert_eq!(layer_count(editing_psd(&pdf, "owner")), 2, "{case}");
            assert_eq!(editing_psd(&pdf, ""), EditingPsd::Locked, "{case}");
            assert_eq!(editing_psd(&pdf, "wrong"), EditingPsd::Locked, "{case}");
        }
    }
}

// pdf_permissions_only_layers
#[test]
fn permissions_password_alone_reopens_without_a_password() {
    let pdf = write(
        &two_layer_doc(),
        &PdfOptions {
            preserve_editing: true,
            security: Security {
                permissions_password: "owner".into(),
                ..Security::default()
            },
            ..options()
        },
    );
    assert_eq!(embedded_psd(&pdf), None, "encrypted");
    assert_eq!(layer_count(editing_psd(&pdf, "")), 2);
}

#[test]
fn encrypted_pdf_without_editing_data_is_absent_once_open() {
    let pdf = write(
        &rgb_doc(2, 2, 72.0, None),
        &PdfOptions {
            security: Security {
                open_password: "kooka".into(),
                ..Security::default()
            },
            ..options()
        },
    );
    assert_eq!(editing_psd(&pdf, "kooka"), EditingPsd::Absent);
    assert_eq!(editing_psd(&pdf, "nope"), EditingPsd::Locked);
}

#[test]
fn pdf_notes_become_text_annotations() {
    let mut doc = rgb_doc(100, 100, 72.0, None);
    let index = doc.annotations.add(MarkerKind::Note, 10, 20).unwrap();
    doc.annotations.set_note_text(index, "check the sky");
    let pdf = write(&doc, &options());
    let page = text(&pdf);
    assert!(page.contains("/Subtype /Text"));
    assert!(page.contains("(check the sky)"));
    assert!(page.contains("/Rect [10 60 30 80]"));
    let without = write(
        &doc,
        &PdfOptions {
            notes: false,
            ..options()
        },
    );
    assert!(!text(&without).contains("/Subtype /Text"));
}

#[test]
fn pdf_thumbnail_is_106px() {
    let options = PdfOptions {
        embed_thumbnails: true,
        ..options()
    };
    let pdf = write(&rgb_doc(400, 200, 72.0, None), &options);
    assert!(text(&pdf).contains("/Thumb "));
    let (dict, data) = stream(&pdf, &["/Width 106/Height 53"]).expect("thumbnail");
    assert!(dict.contains("/ColorSpace /DeviceRGB"));
    assert_eq!(inflate(data).len(), 106 * 53 * 3);
}

#[test]
fn pdf_jpeg_requests_the_quality() {
    let options = PdfOptions {
        compression: Compression::Jpeg,
        quality: Quality::Low,
        ..options()
    };
    let pdf = write(&rgb_doc(3, 2, 72.0, None), &options);
    let (dict, data) = stream(&pdf, &["/Subtype /Image"]).unwrap();
    assert!(dict.contains("/Filter /DCTDecode"));
    assert_eq!(&data[2..10], b"FAKEJPEG");
    assert_eq!(data[10..], [3, Quality::Low.encoder_quality()]);
}
