//! PDF/X and PDF/A structure, and Convert To Destination. Validation by a
//! conformance checker is a manual test-plan row (no validator in CI).

use pictura_core::Document;

use super::oracle::{difference, expected_rgb, has_tool, render};
use super::{image, inflate, plain_options, rgb_doc, stream, text, write};
use crate::pdf::{ColorConversion, Destination, PdfOptions, ProfileInclusion, Standard};

fn with_standard(standard: Standard) -> PdfOptions {
    let mut options = plain_options();
    options.set(
        "standard",
        &Standard::ALL
            .iter()
            .position(|s| *s == standard)
            .unwrap()
            .to_string(),
    );
    options
}

/// A muted document (every sample 60–200) that survives a CMYK round trip.
fn muted_doc() -> Document {
    let (w, h) = (8u32, 6u32);
    let mut rgba = Vec::new();
    for i in 0..(w * h) as usize {
        rgba.extend([60 + (i * 3 % 140) as u8, 90 + (i * 5 % 100) as u8, 120, 255]);
    }
    Document::from_rgba("muted", w, h, &rgba)
}

#[test]
fn pdf_x1a_conformance() {
    let pdf = write(
        &rgb_doc(6, 4, 72.0, Some(2)),
        &with_standard(Standard::X1a2001),
    );
    let all = text(&pdf);
    assert!(pdf.starts_with(b"%PDF-1.3"));
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/ColorSpace /DeviceCMYK"), "{dict}");
    assert!(!dict.contains("/SMask"), "no transparency");
    assert!(all.contains("/S /GTS_PDFX"));
    assert!(all.contains("/GTS_PDFXVersion (PDF/X-1:2001)"));
    assert!(all.contains("/GTS_PDFXConformance (PDF/X-1a:2001)"));
    assert!(all.contains("/Trapped /False"));
    assert!(all.contains("/Title (test)"));
    assert!(all.contains("/TrimBox [0 0 6 4]"));
    assert!(!all.contains("/Encrypt"));
    let (_, profile) = stream(&pdf, &["/N 4"]).expect("output intent profile");
    assert_eq!(inflate(profile)[8], 2, "PDF 1.3 embeds an ICC v2 profile");
}

#[test]
fn pdf_x3_conformance() {
    let pdf = write(&rgb_doc(6, 4, 72.0, None), &with_standard(Standard::X32002));
    assert!(pdf.starts_with(b"%PDF-1.3"));
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/ColorSpace [/ICCBased "), "{dict}");
    let all = text(&pdf);
    assert!(all.contains("/GTS_PDFXVersion (PDF/X-3:2002)"));
    assert!(all.contains("/S /GTS_PDFX"));
}

#[test]
fn pdf_x4_conformance() {
    let pdf = write(
        &rgb_doc(6, 4, 72.0, Some(2)),
        &with_standard(Standard::X42008),
    );
    assert!(pdf.starts_with(b"%PDF-1.6"));
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/SMask "), "X-4 keeps live transparency");
    let (_, xmp) = stream(&pdf, &["/Type /Metadata"]).expect("XMP");
    let xmp = text(xmp);
    assert!(xmp.contains("<pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>"));
}

#[test]
fn pdf_a1b_conformance() {
    let mut options = with_standard(Standard::A1b2005);
    options.security.open_password = "ignored".into();
    let pdf = write(&rgb_doc(6, 4, 72.0, Some(2)), &options);
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let all = text(&pdf);
    assert!(!all.contains("/SMask"));
    assert!(!all.contains("/Encrypt"));
    assert!(all.contains("/S /GTS_PDFA1"));
    let (dict, xmp) = stream(&pdf, &["/Type /Metadata"]).expect("XMP");
    assert!(!dict.contains("/Filter"), "PDF/A metadata is unfiltered");
    let xmp = text(xmp);
    assert!(xmp.contains("<pdfaid:part>1</pdfaid:part>"));
    assert!(xmp.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
    assert!(xmp.contains("<xmp:CreateDate>2026-10-17T12:34:56Z</xmp:CreateDate>"));
    assert!(all.contains("/CreationDate (D:20261017123456Z)"));
}

fn to_cmyk(inclusion: ProfileInclusion) -> PdfOptions {
    PdfOptions {
        color_conversion: ColorConversion::ToDestination,
        destination: Destination::CoatedCmyk,
        profile_inclusion: inclusion,
        ..plain_options()
    }
}

#[test]
fn pdf_convert_to_cmyk_tags_destination() {
    let doc = muted_doc();
    let pdf = write(&doc, &to_cmyk(ProfileInclusion::IncludeDestination));
    let (dict, _) = image(&pdf);
    assert!(dict.contains("/ColorSpace [/ICCBased "), "{dict}");
    let (_, profile) = stream(&pdf, &["/N 4", "/Alternate /DeviceCMYK"]).expect("profile");
    assert_eq!(
        inflate(profile),
        pictura_color::Profile::coated_cmyk_icc(),
        "PDF 1.5 embeds the bundled bytes verbatim"
    );
    if has_tool("gs") {
        let (_, _, rgb) = render(&pdf, 72.0, "gs-icc", None);
        let (_, mean) = difference(&rgb, &expected_rgb(&doc));
        assert!(
            mean < 12.0,
            "a colour-managed round trip stays close (mean {mean})"
        );
    }
}

#[test]
fn pdf_dont_include_profiles_writes_device() {
    let pdf = write(&muted_doc(), &to_cmyk(ProfileInclusion::None));
    let (dict, samples) = image(&pdf);
    assert!(dict.contains("/ColorSpace /DeviceCMYK"));
    assert_eq!(samples.len(), 8 * 6 * 4);
}
