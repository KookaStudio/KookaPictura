//! `pdf::import` tests. Ghostscript-backed tests self-skip when `gs` is absent
//! (CI installs it); fixtures are written here by hand.

use std::io::Write as _;
use std::path::PathBuf;

use super::*;
use crate::ghostscript::ScratchDir;

/// A PDF whose object `i + 1` has body `objects[i]`; object 1 is the catalog.
pub(crate) fn pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(&crate::pdf::object::xref_section(1, &offsets));
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A one-page PDF: `page` is extra page-dictionary text, `content` the page's
/// content stream.
pub(crate) fn one_page(page: &str, content: &str) -> Vec<u8> {
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R {page} /Contents 4 0 R >>").into_bytes(),
        stream("", content.as_bytes()),
    ])
}

const RED_RECT: &str = "1 0 0 rg 20 20 60 30 re f";

/// Ghostscript, or `None` (the test skips) when it is not installed.
fn gs() -> Option<Ghostscript> {
    let found = Ghostscript::locate().ok();
    if found.is_none() {
        eprintln!("skipping: Ghostscript (gs) not found");
    }
    found
}

fn write(scratch: &ScratchDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = scratch.path(name);
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(bytes))
        .expect("write fixture");
    path
}

fn pixel(r: &Raster, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * r.width as usize + x as usize) * 4;
    r.rgba[at..at + 4].try_into().unwrap()
}

fn close(a: &PdfRect, b: &PdfRect, tolerance: f64) -> bool {
    (a.x0 - b.x0).abs() <= tolerance
        && (a.y0 - b.y0).abs() <= tolerance
        && (a.x1 - b.x1).abs() <= tolerance
        && (a.y1 - b.y1).abs() <= tolerance
}

#[test]
fn page_boxes_apply_pdf_defaults() {
    let pages = parse_page_info(
        "PAGES 1\nPAGE 1 /MediaBox [0 0 612 792] /CropBox [36 36 576 756] /TrimBox [72.0 72.0 540.0 720.0] /Rotate 0\n",
    );
    assert_eq!(pages.len(), 1);
    let page = &pages[0];
    let size = |r: PdfRect| (r.width(), r.height());
    assert_eq!(size(page.view_box(CropTo::MediaBox)), (612.0, 792.0));
    assert_eq!(size(page.view_box(CropTo::CropBox)), (540.0, 720.0));
    assert_eq!(size(page.view_box(CropTo::BleedBox)), (540.0, 720.0));
    assert_eq!(size(page.view_box(CropTo::ArtBox)), (540.0, 720.0));
    assert_eq!(size(page.view_box(CropTo::TrimBox)), (468.0, 648.0));
    // Without a Bounding Box query the Crop Box stands in.
    assert_eq!(size(page.view_box(CropTo::BoundingBox)), (540.0, 720.0));
}

#[test]
fn page_info_reads_boxes_through_ghostscript() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "boxes.pdf",
        &one_page(
            "/MediaBox [0 0 612 792] /CropBox [36 36 576 756] /TrimBox [72 72 540 720]",
            RED_RECT,
        ),
    );
    let pages = page_info(&gs, &path, None).expect("page info");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].trim, PdfRect::new(72.0, 72.0, 540.0, 720.0));
    assert_eq!(pages[0].bleed, PdfRect::new(36.0, 36.0, 576.0, 756.0));
    assert_eq!(pages[0].media, PdfRect::new(0.0, 0.0, 612.0, 792.0));
}

#[test]
fn bounding_box_hugs_marks() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "marks.pdf",
        &one_page("/MediaBox [0 0 144 72]", RED_RECT),
    );
    let mut pages = page_info(&gs, &path, None).unwrap();
    query_bounding_boxes(&gs, &path, None, &mut pages).unwrap();
    let bounding = pages[0].view_box(CropTo::BoundingBox);
    assert!(
        close(&bounding, &PdfRect::new(20.0, 20.0, 80.0, 50.0), 0.6),
        "{bounding:?}"
    );
}

#[test]
fn rotated_page_swaps_box() {
    let pages = parse_page_info("PAGE 1 /MediaBox [0 0 144 72] /Rotate 90\n");
    let page = &pages[0];
    assert_eq!(page.view_size(), (72.0, 144.0));
    let media = page.view_box(CropTo::MediaBox);
    assert_eq!((media.width(), media.height()), (72.0, 144.0));
    // A user-space box maps through the rotation: (x, y) -> (y, W - x).
    let mut page = page.clone();
    page.crop = PdfRect::new(20.0, 20.0, 80.0, 50.0);
    assert_eq!(
        page.view_box(CropTo::CropBox),
        PdfRect::new(20.0, 64.0, 50.0, 124.0)
    );
    let turned = parse_page_info("PAGE 1 /MediaBox [0 0 144 72] /Rotate -90\n");
    assert_eq!(turned[0].rotate, 270);
}

#[test]
fn rotated_page_renders_portrait() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "rot.pdf",
        &one_page("/MediaBox [0 0 144 72] /Rotate 90", RED_RECT),
    );
    let pages = page_info(&gs, &path, None).unwrap();
    assert_eq!(pages[0].rotate, 90);
    let out = render_page(
        &gs,
        &path,
        None,
        1,
        &pages[0],
        CropTo::MediaBox,
        (72, 144),
        true,
    )
    .expect("render");
    assert_eq!((out.raster.width, out.raster.height), (72, 144));
    // View rect x 20..50, y 64..124 -> rows 20..80 from the top.
    assert_eq!(pixel(&out.raster, 35, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&out.raster, 5, 5)[3], 0);
}

#[test]
fn render_page_crops_box() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "page.pdf",
        &one_page("/MediaBox [0 0 144 72]", RED_RECT),
    );
    let mut pages = page_info(&gs, &path, None).unwrap();
    let full = render_page(
        &gs,
        &path,
        None,
        1,
        &pages[0],
        CropTo::MediaBox,
        (144, 72),
        true,
    )
    .expect("render");
    assert_eq!((full.raster.width, full.raster.height), (144, 72));
    assert_eq!(pixel(&full.raster, 50, 37), [255, 0, 0, 255]);
    assert_eq!(
        pixel(&full.raster, 5, 5)[3],
        0,
        "unpainted area stays transparent"
    );
    assert_eq!(pixel(&full.raster, 100, 37)[3], 0);

    query_bounding_boxes(&gs, &path, None, &mut pages).unwrap();
    let cropped = render_page(
        &gs,
        &path,
        None,
        1,
        &pages[0],
        CropTo::BoundingBox,
        (60, 30),
        true,
    )
    .expect("render");
    assert_eq!((cropped.raster.width, cropped.raster.height), (60, 30));
    for (x, y) in [(2, 2), (30, 15), (57, 27)] {
        assert_eq!(pixel(&cropped.raster, x, y), [255, 0, 0, 255], "({x}, {y})");
    }
}

#[test]
fn antialias_toggles_edge_coverage() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "tri.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0 0 1 rg 10 10 m 130 60 l 130 10 l f",
        ),
    );
    let pages = page_info(&gs, &path, None).unwrap();
    let partial = |aa| {
        let out = render_page(
            &gs,
            &path,
            None,
            1,
            &pages[0],
            CropTo::MediaBox,
            (144, 72),
            aa,
        )
        .expect("render");
        out.raster
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[3] != 0 && px[3] != 255)
            .count()
    };
    assert!(
        partial(true) > 0,
        "anti-aliased edges have partial coverage"
    );
    assert_eq!(partial(false), 0, "aliased edges are fully on or off");
}

/// A PDF whose page draws a 40×30 Flate RGB image twice (two names, one
/// object) and a 16×16 gray image inside a Form XObject.
pub(crate) fn images_pdf() -> (Vec<u8>, Vec<u8>) {
    let samples: Vec<u8> = (0..40 * 30 * 3).map(|i| (i * 7 % 251) as u8).collect();
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&samples).unwrap();
    let packed = z.finish().unwrap();
    let gray: Vec<u8> = (0..16 * 16).map(|i| i as u8).collect();
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /XObject << /Im0 5 0 R /Im1 5 0 R /Fm0 6 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>".to_vec(),
        stream("", b"q 40 0 0 30 0 0 cm /Im0 Do Q q 40 0 0 30 50 50 cm /Im1 Do Q /Fm0 Do"),
        stream(
            "/Type /XObject /Subtype /Image /Width 40 /Height 30 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
            &packed,
        ),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /XObject << /Im2 7 0 R >> >>",
            b"q 16 0 0 16 100 100 cm /Im2 Do Q",
        ),
        stream(
            "/Type /XObject /Subtype /Image /Width 16 /Height 16 /ColorSpace /DeviceGray /BitsPerComponent 8",
            &gray,
        ),
    ]);
    (bytes, samples)
}

#[test]
fn images_listed_once_through_forms() {
    let (bytes, _) = images_pdf();
    let found = PdfImages::read(&bytes).expect("readable");
    let sizes: Vec<(u32, u32, u32)> = found
        .images
        .iter()
        .map(|i| (i.page, i.width, i.height))
        .collect();
    assert_eq!(sizes, vec![(1, 40, 30), (1, 16, 16)]);
}

#[test]
fn image_renders_at_native_size() {
    let Some(gs) = gs() else { return };
    let (bytes, samples) = images_pdf();
    let found = PdfImages::read(&bytes).unwrap();
    let out = found.render(&gs, 0, None).expect("render");
    assert_eq!((out.raster.width, out.raster.height), (40, 30));
    // PDF images are drawn top row first.
    for (i, px) in out.raster.rgba.as_chunks::<4>().0.iter().enumerate() {
        let want = &samples[i * 3..i * 3 + 3];
        assert_eq!(&px[..3], want, "pixel {i}");
        assert_eq!(px[3], 255);
    }
    let thumb = found.render(&gs, 0, Some(20)).expect("thumbnail");
    assert_eq!((thumb.raster.width, thumb.raster.height), (20, 15));
}

#[test]
fn objects_survive_broken_xref() {
    let (mut bytes, _) = images_pdf();
    let at = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
    bytes.truncate(at);
    bytes.extend_from_slice(b"startxref\n999999999\n%%EOF\n");
    let found = PdfImages::read(&bytes).expect("objects found by scanning");
    assert_eq!(found.images.len(), 2);
}

#[test]
fn encrypted_pdf_disables_images() {
    let mut bytes = one_page("/MediaBox [0 0 10 10]", "");
    let at = bytes.windows(7).rposition(|w| w == b"/Root 1").unwrap();
    bytes.splice(at..at, b"/Encrypt << /Filter /Standard >> ".iter().copied());
    assert!(PdfImages::read(&bytes).is_none());
}

/// `plain` encrypted by Ghostscript's `pdfwrite` with user password `kooka`.
fn encrypted(gs: &Ghostscript, scratch: &ScratchDir) -> PathBuf {
    encrypted_with(gs, scratch, "kooka")
}

fn encrypted_with(gs: &Ghostscript, scratch: &ScratchDir, password: &str) -> PathBuf {
    let plain = write(
        scratch,
        "plain.pdf",
        &one_page("/MediaBox [0 0 144 72]", RED_RECT),
    );
    let out = scratch.path("locked.pdf");
    gs.run(
        &plain,
        None,
        &[
            "-sDEVICE=pdfwrite".into(),
            "-sOwnerPassword=owner".into(),
            format!("-sUserPassword={password}"),
            "-dEncryptionR=3".into(),
            "-dKeyLength=128".into(),
            format!("-sOutputFile={}", out.display()),
            "-f".into(),
            plain.display().to_string(),
        ],
    )
    .expect("encrypt");
    out
}

#[test]
fn password_required_is_reported() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let locked = encrypted(&gs, &scratch);
    assert_eq!(
        page_info(&gs, &locked, None).err(),
        Some(RasterError::PasswordRequired)
    );
    let pages = page_info(&gs, &locked, Some("kooka")).expect("opens with the password");
    assert_eq!(pages.len(), 1);
    let out = render_page(
        &gs,
        &locked,
        Some("kooka"),
        1,
        &pages[0],
        CropTo::MediaBox,
        (144, 72),
        true,
    )
    .expect("render");
    assert_eq!(pixel(&out.raster, 50, 37), [255, 0, 0, 255]);
}

#[test]
fn password_with_quotes_and_spaces_opens() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let password = "two \"words\" back\\slash";
    let locked = encrypted_with(&gs, &scratch, password);
    let pages = page_info(&gs, &locked, Some(password)).expect("the argument file quotes it");
    assert_eq!(pages.len(), 1);
    assert_eq!(
        page_info(&gs, &locked, Some("two words")).err(),
        Some(RasterError::WrongPassword)
    );
}

#[test]
fn wrong_password_is_rejected() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let locked = encrypted(&gs, &scratch);
    assert_eq!(
        page_info(&gs, &locked, Some("wrong")).err(),
        Some(RasterError::WrongPassword)
    );
}

#[test]
fn thumbnails_keep_relative_size() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 288 144] /Contents 4 0 R >>".to_vec(),
        stream("", RED_RECT.as_bytes()),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 144 72] /Contents 4 0 R >>".to_vec(),
    ]);
    let path = write(&scratch, "two.pdf", &bytes);
    let pages = page_info(&gs, &path, None).unwrap();
    assert_eq!(pages.len(), 2);
    let thumbs = render_thumbnails(&gs, &path, None, &pages, 64).expect("thumbnails");
    let sizes: Vec<(u32, u32)> = thumbs.iter().map(|t| (t.width, t.height)).collect();
    assert_eq!(sizes, vec![(64, 32), (32, 16)]);
}
