//! Malformed and hostile PDFs (#844): each one is refused or read partially,
//! never a panic, an unbounded allocation, or a stack overflow.

use std::io::Write as _;

use super::images::{list_images, wrap_image};
use super::objects::{decode_stream, Object, PdfObjects, Unlock, MAX_NESTING};
use super::tests::pdf;
use super::{budget_scale, crop_raster, MAX_IMAGES};
use crate::ghostscript::Raster;

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn decoded(file: &PdfObjects, num: u32) -> Option<Vec<u8>> {
    match file.get(num)? {
        Object::Stream(dict, data) => decode_stream(file, dict, data),
        _ => None,
    }
}

#[test]
fn stream_inflation_is_bounded_per_stream_and_per_file() {
    // A small zlib stream may expand without limit (#844 H2).
    let bomb = zlib(&[0u8; 4000]);
    let bytes = pdf(&[
        b"<< /Type /Catalog >>".to_vec(),
        stream("/Filter /FlateDecode", &bomb),
        stream("/Filter /FlateDecode", &bomb),
    ]);
    let mut file = PdfObjects::parse(&bytes).unwrap();
    file.stream_limit = 3999;
    assert_eq!(decoded(&file, 2), None, "over the per-stream limit");

    let file = PdfObjects::parse(&bytes).unwrap();
    file.inflate_budget.set(6000);
    assert_eq!(decoded(&file, 2).map(|d| d.len()), Some(4000));
    assert_eq!(decoded(&file, 3), None, "the file's budget is spent");
}

#[test]
fn hostile_predictor_parameters_are_refused() {
    // `/Columns` sized a `vec!` and `columns * colors * bytes` wrapped to a
    // zero-width `chunks_mut` (#844 H3).
    let data = zlib(&[1u8; 64]);
    let huge = i64::MAX;
    let bytes = pdf(&[
        b"<< /Type /Catalog >>".to_vec(),
        stream(
            &format!("/Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns {huge} >>"),
            &data,
        ),
        stream(
            "/Filter /FlateDecode /DecodeParms << /Predictor 2 /Columns 4611686018427387904 /Colors 4 >>",
            &data,
        ),
        stream(
            "/Filter /FlateDecode /DecodeParms << /Predictor 2 /Columns 8 /Colors 4096 >>",
            &data,
        ),
        stream(
            "/Filter /FlateDecode /DecodeParms << /Predictor 2 /Columns 8 /Colors 2 >>",
            &data,
        ),
    ]);
    let file = PdfObjects::parse(&bytes).unwrap();
    assert_eq!(decoded(&file, 2), None);
    assert_eq!(decoded(&file, 3), None);
    assert_eq!(decoded(&file, 4), None);
    assert_eq!(
        decoded(&file, 5).map(|d| d.len()),
        Some(64),
        "a sane row still decodes"
    );
}

#[test]
fn hostile_object_stream_counts_and_offsets_are_harmless() {
    // `/N 9223372036854775807` panicked with "capacity overflow"; a negative
    // offset plus `/First` overflowed (#844 H3, low).
    let body = b"7 -5 8 0 << /A 1 >>";
    let bytes = pdf(&[
        b"<< /Type /Catalog >>".to_vec(),
        stream(&format!("/Type /ObjStm /N {} /First 9", i64::MAX), body),
    ]);
    let file = PdfObjects::parse(&bytes).unwrap();
    assert!(file.get(7).is_none(), "a negative offset is skipped");
    assert!(file.get(8).is_some(), "a sane entry still expands");
}

#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    // The lexer recursed once per `[` (#844 H6).
    let depth = 200_000;
    let mut deep = "[".repeat(depth).into_bytes();
    deep.extend_from_slice("]".repeat(depth).as_bytes());
    let mut ok = "[".repeat(MAX_NESTING as usize).into_bytes();
    ok.extend_from_slice("]".repeat(MAX_NESTING as usize).as_bytes());
    let bytes = pdf(&[b"<< /Type /Catalog >>".to_vec(), deep, ok]);
    let file = PdfObjects::parse(&bytes).unwrap();
    assert!(file.get(2).is_none(), "nesting past the cap is refused");
    assert!(file.get(3).is_some(), "nesting at the cap still reads");
}

#[test]
fn deep_page_and_form_trees_end() {
    // `collect_pages`/`collect_images` recursed once per level (#844 H6).
    let levels = 5_000;
    let mut objects = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()];
    for i in 0..levels {
        let num = i + 2;
        objects.push(format!("<< /Type /Pages /Kids [{} 0 R] >>", num + 1).into_bytes());
    }
    objects.push(b"<< /Type /Page >>".to_vec());
    let file = PdfObjects::parse(&pdf(&objects)).unwrap();
    assert_eq!(list_images(&file), Some(Vec::new()));
}

#[test]
fn image_listing_is_capped() {
    // Tens of thousands of images were each thumbnailed (#844 M2).
    let count = MAX_IMAGES + 20;
    let names: String = (0..count)
        .map(|i| format!("/I{i} {} 0 R ", i + 4))
        .collect();
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Resources << /XObject << {names}>> >> >>").into_bytes(),
    ];
    for _ in 0..count {
        objects.push(stream("/Subtype /Image /Width 1 /Height 1", &[0]));
    }
    let file = PdfObjects::parse(&pdf(&objects)).unwrap();
    assert_eq!(list_images(&file).unwrap().len(), MAX_IMAGES);
}

#[test]
fn a_non_positive_image_size_is_not_wrapped() {
    // #844 M8: the listing filters these; the wrapper must too.
    let bytes = pdf(&[
        b"<< /Type /Catalog >>".to_vec(),
        stream("/Subtype /Image /Width -5 /Height 3", &[0]),
        stream("/Subtype /Image /Width 2 /Height 3", &[0]),
    ]);
    let file = PdfObjects::parse(&bytes).unwrap();
    assert_eq!(wrap_image(&file, 2), None);
    assert!(wrap_image(&file, 3).is_some());
}

#[test]
fn an_image_render_is_scaled_into_the_budget() {
    // An image dictionary may declare any size; Open Image rendered it as is
    // (#844 M1).
    assert_eq!(budget_scale(640, 480), 1.0);
    let scale = budget_scale(u32::MAX, u32::MAX);
    let side = f64::from(u32::MAX) * scale;
    assert!(side <= 30_000.0, "{side}");
    let scale = budget_scale(29_000, 29_000);
    let bytes = (29_000.0 * scale).powi(2) * 4.0;
    assert!(bytes <= 2.0 * 1024.0 * 1024.0 * 1024.0, "{bytes}");
}

#[test]
fn a_crop_over_the_budget_is_empty() {
    // `w * h * 4` from the dialog was allocated unchecked (#844 M8).
    let src = Raster {
        width: 1,
        height: 1,
        rgba: vec![9; 4],
    };
    assert_eq!(
        crop_raster(&src, 0, 0, u32::MAX, u32::MAX),
        Raster::default()
    );
    assert_eq!(crop_raster(&src, 0, 0, 1, 1).rgba, vec![9; 4]);
}

#[test]
fn a_short_aes_key_is_unsupported() {
    // `/V 4 /R 4 /CFM /AESV2 /Length 40` gave a 5-byte file key that reached
    // AES-256 and panicked (#844 H5).
    let encrypt = format!(
        "<< /Filter /Standard /V 4 /R 4 /Length 40 /P -4 \
         /CF << /StdCF << /CFM /AESV2 >> >> /StmF /StdCF /StrF /StdCF \
         /O <{o}> /U <{u}> >>",
        o = "00".repeat(32),
        u = "00".repeat(32),
    );
    let mut bytes = pdf(&[b"<< /Type /Catalog >>".to_vec(), encrypt.into_bytes()]);
    bytes.extend_from_slice(b"trailer\n<< /Root 1 0 R /Encrypt 2 0 R /ID [<00> <00>] >>\n");
    let mut file = PdfObjects::parse(&bytes).unwrap();
    assert!(file.encrypted());
    assert_eq!(file.decrypt(""), Unlock::Unsupported);
}
