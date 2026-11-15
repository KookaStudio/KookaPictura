//! `pdf::import` tests. Ghostscript-backed tests self-skip when `gs` is absent
//! (CI installs it); fixtures are written here by hand.

use std::io::Write as _;
use std::path::{Path, PathBuf};

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
    let found = PdfImages::read(&bytes, "").expect("readable");
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
    let found = PdfImages::read(&bytes, "").unwrap();
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
    let found = PdfImages::read(&bytes, "").expect("objects found by scanning");
    assert_eq!(found.images.len(), 2);
}

#[test]
fn encrypted_pdf_disables_images() {
    let mut bytes = one_page("/MediaBox [0 0 10 10]", "");
    let at = bytes.windows(7).rposition(|w| w == b"/Root 1").unwrap();
    bytes.splice(at..at, b"/Encrypt << /Filter /Standard >> ".iter().copied());
    assert!(PdfImages::read(&bytes, "").is_none());
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

// ---------------------------------------------------------------------------
// Native mode/depth renders
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn native_render(
    gs: &Ghostscript,
    path: &Path,
    area: PdfRect,
    view_height: f64,
    size: (u32, u32),
    aa: bool,
    mode: pictura_core::ColorMode,
    depth: pictura_core::BitDepth,
) -> super::native::NativeRender {
    super::native::render_native_area(gs, path, None, 1, area, view_height, size, aa, mode, depth)
        .expect("native render")
}

fn is_non_257(values: &[u16]) -> bool {
    values.iter().any(|v| *v % 257 != 0)
}

#[test]
fn native_psd_device_output_reads() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "native.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0.123 0.456 0.789 rg 0 0 144 72 re f",
        ),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Sixteen,
    );
    assert_eq!((out.width, out.height), (144, 72));
    assert!(matches!(out.samples, pictura_core::Samples::U16(_)));
    assert_eq!(out.depth, pictura_core::BitDepth::Sixteen);
}

#[test]
fn native_rgb16_render_keeps_more_than_eight_bits() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "rgb16.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0.123 0.456 0.789 rg 0 0 144 72 re f",
        ),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Sixteen,
    );
    let pictura_core::Samples::U16(samples) = &out.samples else {
        panic!("not 16-bit");
    };
    assert!(is_non_257(samples), "the native samples stayed 8-bit");
}

#[test]
fn native_gray16_keeps_more_than_eight_bits() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "gray16.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0.123 0.456 0.789 rg 0 0 144 72 re f",
        ),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Sixteen,
    );
    let pictura_core::Samples::U16(v) = &out.samples else {
        panic!("not 16-bit");
    };
    let plane = out.width as usize * out.height as usize;
    let gray: Vec<u16> = (0..plane)
        .map(|i| {
            ((299 * u32::from(v[i])
                + 587 * u32::from(v[plane + i])
                + 114 * u32::from(v[2 * plane + i]))
                / 1000) as u16
        })
        .collect();
    assert!(is_non_257(&gray), "the gray luma stayed 8-bit");
}

/// A page with a dark `k` patch and a known red `rg` patch: the red gives an
/// absolute hue the native CMYK render must keep, so an inverted polarity
/// cannot pass the sampled-colour checks.
const CMYK_HUE_PAGE: &str = "0.2 0.4 0.6 0.1 k 20 20 60 30 re f \
                             1 0 0 rg 90 20 30 30 re f";

fn display_at(out: &super::native::NativeRender, x: usize, y: usize) -> (u8, u8, u8) {
    let plane = out.width as usize * out.height as usize;
    let at = y * out.width as usize + x;
    (
        out.display[at],
        out.display[plane + at],
        out.display[2 * plane + at],
    )
}

fn assert_red_display(out: &super::native::NativeRender, x: usize, y: usize) {
    let (r, g, b) = display_at(out, x, y);
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    assert!(
        r > 120 && r > g + 60 && r > b + 60,
        "the red CMYK patch displayed as ({r}, {g}, {b})"
    );
}

#[test]
fn native_cmyk16_keeps_rendered_inks() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "cmyk16.pdf",
        &one_page("/MediaBox [0 0 144 72]", CMYK_HUE_PAGE),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Cmyk,
        pictura_core::BitDepth::Sixteen,
    );
    assert_eq!(out.mode, pictura_core::ColorMode::Cmyk);
    assert_eq!(out.depth, pictura_core::BitDepth::Sixteen);
    let pictura_core::Samples::U16(samples) = &out.samples else {
        panic!("not 16-bit");
    };
    assert_eq!(samples.len(), 4 * 144 * 72);
    assert!(is_non_257(samples), "the CMYK plates stayed 8-bit");
    assert_red_display(&out, 100, 37);
    assert!(
        out.samples.narrow_to_u8().iter().any(|v| *v < 200),
        "the stored CMYK plates are paper-only"
    );
}

#[test]
fn native_cmyk8_keeps_rendered_inks() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "cmyk8.pdf",
        &one_page("/MediaBox [0 0 144 72]", CMYK_HUE_PAGE),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Cmyk,
        pictura_core::BitDepth::Eight,
    );
    assert_eq!(out.mode, pictura_core::ColorMode::Cmyk);
    assert_eq!(out.depth, pictura_core::BitDepth::Eight);
    assert!(matches!(&out.samples, pictura_core::Samples::U8(_)));
    let narrowed = out.samples.narrow_to_u8();
    assert_eq!(narrowed.len(), 4 * 144 * 72);
    assert_red_display(&out, 100, 37);
    assert!(
        narrowed.iter().any(|v| *v < 200),
        "the stored CMYK plates are paper-only"
    );
}

fn gcr_display(display: &[u8], plane: usize) -> Vec<u8> {
    let mut out = vec![255u8; plane * 4];
    for i in 0..plane {
        let (r, g, b) = (display[i], display[plane + i], display[2 * plane + i]);
        let max = r.max(g).max(b);
        out[3 * plane + i] = max;
        if max == 0 {
            continue;
        }
        out[i] = ((u32::from(r) * 255 + u32::from(max) / 2) / u32::from(max)) as u8;
        out[plane + i] = ((u32::from(g) * 255 + u32::from(max) / 2) / u32::from(max)) as u8;
        out[2 * plane + i] = ((u32::from(b) * 255 + u32::from(max) / 2) / u32::from(max)) as u8;
    }
    out
}

#[test]
fn native_cmyk16_is_not_the_gcr_conversion() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "cmyk16.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0.2 0.4 0.6 0.1 k 20 20 60 30 re f",
        ),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Cmyk,
        pictura_core::BitDepth::Sixteen,
    );
    let plane = out.width as usize * out.height as usize;
    let narrowed = out.samples.narrow_to_u8();
    let gcr = gcr_display(&out.display, plane);
    assert_ne!(
        &narrowed[..plane * 4],
        &gcr[..],
        "the native plates are the GCR conversion"
    );
}

#[test]
fn native_render_keeps_unpainted_areas_transparent() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "native.pdf",
        &one_page("/MediaBox [0 0 144 72]", RED_RECT),
    );
    let out = native_render(
        &gs,
        &path,
        PdfRect::new(0.0, 0.0, 144.0, 72.0),
        72.0,
        (144, 72),
        true,
        pictura_core::ColorMode::Cmyk,
        pictura_core::BitDepth::Sixteen,
    );
    let at = |x: usize, y: usize| out.alpha[y * out.width as usize + x];
    assert_eq!(at(5, 5), 0);
    assert_eq!(at(50, 37), 255);
}

#[test]
fn native_antialias_toggles_edge_coverage() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "native-tri.pdf",
        &one_page(
            "/MediaBox [0 0 144 72]",
            "0 0 0 rg 10 10 m 130 60 l 130 10 l f",
        ),
    );
    let render = |aa| {
        native_render(
            &gs,
            &path,
            PdfRect::new(0.0, 0.0, 144.0, 72.0),
            72.0,
            (144, 72),
            aa,
            pictura_core::ColorMode::Cmyk,
            pictura_core::BitDepth::Sixteen,
        )
    };
    let antialiased = render(true);
    let width = antialiased.width as usize;
    let partial: Vec<usize> = antialiased
        .alpha
        .iter()
        .enumerate()
        .filter(|(_, a)| **a != 0 && **a != 255)
        .map(|(at, _)| at)
        .collect();
    assert!(!partial.is_empty());
    let plane = antialiased.width as usize * antialiased.height as usize;
    let narrowed = antialiased.samples.narrow_to_u8();
    for at in partial {
        let (r, g, b) = display_at(&antialiased, at % width, at / width);
        assert!(
            !(r == 255 && g == 255 && b == 255),
            "a partially covered black edge displayed as paper white"
        );
        let stored = [
            narrowed[at],
            narrowed[plane + at],
            narrowed[2 * plane + at],
            narrowed[3 * plane + at],
        ];
        let shown = crate::cmyk_to_rgb(&stored);
        assert_eq!(
            (r, g, b),
            (shown[0], shown[1], shown[2]),
            "the display is not the stored CMYK conversion at edge pixel {at}"
        );
    }
    assert_eq!(
        render(false)
            .alpha
            .iter()
            .filter(|a| **a != 0 && **a != 255)
            .count(),
        0
    );
}

#[test]
fn native_crop_matches_the_rgb8_import() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = write(
        &scratch,
        "native-crop.pdf",
        &one_page("/MediaBox [0 0 144 72]", RED_RECT),
    );
    let area = PdfRect::new(20.0, 20.0, 80.0, 50.0);
    let legacy = render_area(&gs, &path, None, 1, area, 72.0, (60, 30), true).expect("legacy");
    let native = native_render(
        &gs,
        &path,
        area,
        72.0,
        (60, 30),
        true,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Sixteen,
    );
    assert_eq!(
        (legacy.raster.width, legacy.raster.height),
        (native.width, native.height)
    );
    let legacy_alpha: Vec<u8> = legacy
        .raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| px[3])
        .collect();
    assert_eq!(legacy_alpha, native.alpha);

    // A rotated page cropped to a Crop Box wider than the painted rectangle:
    // the native path must use the same view-space box the 8-bit path crops,
    // so the transparent margin lines up too.
    let rotated = write(
        &scratch,
        "native-crop-rot.pdf",
        &one_page(
            "/MediaBox [0 0 144 72] /CropBox [20 20 110 60] /Rotate 90",
            RED_RECT,
        ),
    );
    let pages = page_info(&gs, &rotated, None).expect("rotated page info");
    let page = &pages[0];
    let (_, view_height) = page.view_size();
    let crop = page.view_box(CropTo::CropBox);
    let size = (crop.width().round() as u32, crop.height().round() as u32);
    let legacy =
        render_area(&gs, &rotated, None, 1, crop, view_height, size, true).expect("legacy rotated");
    let native = native_render(
        &gs,
        &rotated,
        crop,
        view_height,
        size,
        true,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Sixteen,
    );
    assert_eq!(
        (legacy.raster.width, legacy.raster.height),
        (native.width, native.height)
    );
    assert!(
        native.alpha.contains(&0) && native.alpha.contains(&255),
        "the rotated crop mixes painted and unpainted pixels"
    );
    let legacy_alpha: Vec<u8> = legacy
        .raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| px[3])
        .collect();
    assert_eq!(legacy_alpha, native.alpha);
}

#[test]
fn native_image_render_keeps_more_than_eight_bits() {
    let Some(gs) = gs() else { return };
    let samples: Vec<u8> = (0..2 * 2 * 3 * 2)
        .map(|i| {
            if i % 2 == 0 {
                (i * 37) as u8
            } else {
                (i * 11 + 3) as u8
            }
        })
        .collect();
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 2 2] /Contents 4 0 R >>".to_vec(),
        stream("", b"q 2 0 0 2 0 0 cm /Im0 Do Q"),
        stream(
            "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 16",
            &samples,
        ),
    ]);
    let found = PdfImages::read(&bytes, "").expect("readable");
    let out = found
        .render_native(
            &gs,
            0,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Sixteen,
        )
        .expect("native image");
    let pictura_core::Samples::U16(values) = &out.samples else {
        panic!("not 16-bit");
    };
    assert!(is_non_257(values));
}

#[test]
fn native_eps_render_keeps_cmyk_inks() {
    let Some(gs) = gs() else { return };
    let eps = b"%!PS-Adobe-3.0 EPSF-3.0\n\
%%BoundingBox: 0 0 40 30\n\
0 0 1 setrgbcolor 10 10 20 10 rectfill\n\
showpage\n";
    let scratch = ScratchDir::new().unwrap();
    let path = write(&scratch, "native.eps", eps);
    let bounds = crate::eps::bounding_box(eps).unwrap();
    let out = crate::eps::render_native_eps(
        &gs,
        &path,
        bounds,
        (40, 30),
        true,
        pictura_core::ColorMode::Cmyk,
        pictura_core::BitDepth::Sixteen,
    )
    .expect("native EPS");
    assert_eq!(out.mode, pictura_core::ColorMode::Cmyk);
    assert_eq!(out.depth, pictura_core::BitDepth::Sixteen);
    assert_eq!(out.alpha[0], 0);
    assert_eq!(out.alpha[15 * 40 + 15], 255);
}

// ---------------------------------------------------------------------------
// Encrypted Images mode
// ---------------------------------------------------------------------------

fn encrypted_images(gs: &Ghostscript, scratch: &ScratchDir) -> PathBuf {
    let plain = write(scratch, "pictures-plain.pdf", &images_pdf().0);
    let out = scratch.path("pictures-locked.pdf");
    gs.run(
        &plain,
        None,
        &[
            "-sDEVICE=pdfwrite".into(),
            "-sOwnerPassword=owner".into(),
            "-sUserPassword=kooka".into(),
            "-dEncryptionR=3".into(),
            "-dKeyLength=128".into(),
            format!("-sOutputFile={}", out.display()),
            "-f".into(),
            plain.display().to_string(),
        ],
    )
    .expect("encrypt images");
    out
}

#[test]
fn encrypted_images_open_with_the_password() {
    let Some(gs) = gs() else { return };
    let scratch = ScratchDir::new().unwrap();
    let path = encrypted_images(&gs, &scratch);
    let bytes = std::fs::read(&path).unwrap();
    assert!(PdfImages::read(&bytes, "wrong").is_none());
    let found = PdfImages::read(&bytes, "kooka").expect("password opens images");
    assert!(
        found.images.len() >= 2,
        "Ghostscript may duplicate the drawn image; got {}",
        found.images.len()
    );
    let out = found.render(&gs, 0, None).expect("render image");
    assert_eq!((out.raster.width, out.raster.height), (40, 30));
}

// ---------------------------------------------------------------------------
// Inline images
// ---------------------------------------------------------------------------

fn inline_pdf() -> Vec<u8> {
    let xobject = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
    let inline = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut content = b"q 2 0 0 2 0 0 cm /Im0 Do Q BI /W 2 /H 2 /BPC 8 /CS /RGB ID ".to_vec();
    content.extend_from_slice(&inline);
    content.extend_from_slice(b" EI");
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4 4] /Contents 4 0 R >>".to_vec(),
        stream("", &content),
        stream(
            "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8",
            &xobject,
        ),
    ])
}

#[test]
fn inline_images_are_listed_after_xobjects() {
    let bytes = inline_pdf();
    let found = PdfImages::read(&bytes, "").expect("readable");
    assert_eq!(found.images.len(), 2);
    assert_eq!(found.images[0].object, Some(5));
    assert_eq!(found.images[0].width, 2);
    assert_eq!(found.images[1].object, None);
    assert_eq!(found.images[1].width, 2);
}

#[test]
fn inline_image_renders_at_its_native_size() {
    let Some(gs) = gs() else { return };
    let found = PdfImages::read(&inline_pdf(), "").unwrap();
    let out = found.render(&gs, 1, None).expect("inline render");
    assert_eq!((out.raster.width, out.raster.height), (2, 2));
    for (i, px) in out.raster.rgba.as_chunks::<4>().0.iter().enumerate() {
        let want = &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12][i * 3..i * 3 + 3];
        assert_eq!(&px[..3], want, "pixel {i}");
        assert_eq!(px[3], 255);
    }
}

#[test]
fn inline_images_skip_unsupported_content_filters() {
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4 4] /Contents 4 0 R >>".to_vec(),
        stream(
            "/Filter /LZWDecode",
            b"q 2 0 0 2 0 0 cm /Im0 Do Q BI /W 2 /H 2 /BPC 8 /CS /RGB ID 123456789012 EI",
        ),
        stream(
            "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8",
            &[0u8; 12],
        ),
    ]);
    let found = PdfImages::read(&bytes, "").expect("readable");
    assert_eq!(found.images.len(), 1);
    assert_eq!(found.images[0].object, Some(5));
}

#[test]
fn inline_indexed_image_renders() {
    let Some(gs) = gs() else { return };
    let content = b"BI /W 2 /H 2 /BPC 1 /CS [/I /DeviceRGB 1 <000000ffffff>] /F /AHx ID 0000> EI";
    let bytes = pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4 4] /Contents 4 0 R >>".to_vec(),
        stream("", content),
    ]);
    let found = PdfImages::read(&bytes, "").expect("readable");
    assert_eq!(found.images.len(), 1);
    assert_eq!(found.images[0].object, None);
    let out = found.render(&gs, 0, None).expect("indexed inline render");
    assert_eq!((out.raster.width, out.raster.height), (2, 2));
    for px in out.raster.rgba.as_chunks::<4>().0 {
        assert_eq!(&px[..3], &[0, 0, 0], "index 0 is the black palette entry");
    }
}

#[test]
fn inline_image_scan_is_bounded_and_never_panics() {
    let content = "BI /W 999999999 /H 999999999 /BPC 8 /CS /RGB ID not-a-terminated-image";
    let bytes = one_page("/MediaBox [0 0 4 4]", content);
    let found = PdfImages::read(&bytes, "").expect("readable");
    assert!(found.images.is_empty());
}
