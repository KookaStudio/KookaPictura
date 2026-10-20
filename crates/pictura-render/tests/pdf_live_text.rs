//! Photoshop PDF live text (`interop/pdf-export`, design D13), end to end: the
//! layout here and the writer in `pictura_codec::pdf`, as the app saves. The
//! poppler oracles (`pdftotext`, `pdffonts`, `pdftoppm`) self-skip when absent;
//! CI installs `poppler-utils`.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::Command;

use pictura_codec::pdf::{
    write_pdf, ColorConversion, Compatibility, Destination, JpegRequest, LiveText, PdfEnv,
    PdfOptions, ProfileInclusion,
};
use pictura_core::{Document, PixelBuffer, TypeSpec};

fn has_tool(name: &str) -> bool {
    let found = std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()));
    if !found {
        eprintln!("SKIP: `{name}` is not installed");
    }
    found
}

fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pictura-live-{}-{name}", std::process::id()));
    std::fs::write(&path, bytes).expect("temp file");
    path
}

/// A noisy pixel layer under red `Kooka Pictura`, its composite stored.
fn type_doc() -> Document {
    let (w, h) = (320usize, 100usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        rgba.extend([(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, 255]);
    }
    let mut doc = Document::from_rgba("photo", w as u32, h as u32, &rgba);
    let mut spec = TypeSpec::new("Kooka Pictura", "Liberation Sans", 40.0);
    spec.origin = (12.0, 64.0);
    spec.character.fill_color = [1.0, 1.0, 0.0, 0.0];
    assert!(!pictura_render::add_type_layer(&mut doc, "0", &spec).is_empty());
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

fn options() -> PdfOptions {
    PdfOptions {
        icc_profile: false,
        embed_thumbnails: false,
        preserve_editing: false,
        ..PdfOptions::default()
    }
}

/// Write `doc` as the app does: live text where the options allow it, over the
/// composite of the document without it, which is also returned.
fn write_live(doc: &Document, options: &PdfOptions) -> (Vec<u8>, Option<PixelBuffer>) {
    let mut options = options.clone();
    options.normalize();
    let live_text =
        pictura_render::pdf_live_type(doc, options.text_policy()).map(|live| LiveText {
            background: pictura_render::composite_rgba(&live.hidden),
            fonts: live.fonts,
            texts: live.texts,
        });
    let background = live_text.as_ref().map(|l| l.background.clone());
    let env = PdfEnv {
        jpeg: &|_: &JpegRequest| None,
        now: 1_792_240_496,
        entropy: [9; 32],
        title: "live".into(),
        live_text,
    };
    let pdf = write_pdf(doc, &doc.composite, &options, &env).expect("writes");
    (pdf, background)
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

/// Every stream's dictionary text and raw data, in file order.
fn streams(pdf: &[u8]) -> Vec<(String, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = find(pdf, b" 0 obj\n", at) {
        let Some(end) = find(pdf, b"endobj", start) else {
            break;
        };
        at = end;
        if let Some(stream_at) = find(&pdf[..end], b"\nstream\n", start) {
            if let Some(data_end) = find(pdf, b"\nendstream", stream_at + 8) {
                let dict = String::from_utf8_lossy(&pdf[start + 7..stream_at]).into_owned();
                out.push((dict, &pdf[stream_at + 8..data_end]));
            }
        }
    }
    out
}

fn inflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = flate2::read::ZlibDecoder::new(data).read_to_end(&mut out);
    out
}

/// The page image's interleaved samples (ZIP).
fn image(pdf: &[u8]) -> Vec<u8> {
    let (_, data) = streams(pdf)
        .into_iter()
        .find(|(d, _)| d.contains("/Subtype /Image"))
        .expect("an image");
    inflate(data)
}

/// The page's content stream, inflated.
fn content(pdf: &[u8]) -> String {
    streams(pdf)
        .into_iter()
        .map(|(_, data)| String::from_utf8_lossy(&inflate(data)).into_owned())
        .find(|s| s.contains("/Im0 Do"))
        .expect("a content stream")
}

fn page_text(pdf: &[u8]) -> String {
    String::from_utf8_lossy(pdf).into_owned()
}

fn same_rgb(samples: &[u8], planar: &PixelBuffer) -> bool {
    let plane = planar.pixel_count();
    (0..plane).all(|i| (0..3).all(|c| samples[i * 3 + c] == planar.data[c * plane + i]))
}

// pdf_live_text
#[test]
fn pdf_type_layer_is_live_text() {
    let doc = type_doc();
    let (pdf, background) = write_live(&doc, &options());
    let background = background.expect("the type layer qualifies");
    let page = page_text(&pdf);
    assert!(page.contains("/Subtype /Type0"), "{page}");
    assert!(page.contains("/Encoding /Identity-H"));
    assert!(page.contains("/Subtype /CIDFontType2"));
    assert!(page.contains("/CIDToGIDMap /Identity"));
    assert!(page.contains("/BaseFont /LiberationSans"));
    assert!(page.contains("/FontFile2 "));
    assert!(page.contains("/ToUnicode "));
    assert!(page.contains("/ProcSet [/PDF /ImageC /Text]"));
    let ops = content(&pdf);
    assert!(ops.contains("BT\n/F0 40 Tf\n"), "{ops}");
    assert!(ops.contains("1 0 0 rg"), "red fill: {ops}");
    assert!(ops.contains("] TJ"));

    assert!(
        !same_rgb(&image(&pdf), &doc.composite),
        "the image no longer holds the type"
    );
    assert!(
        same_rgb(&image(&pdf), &background),
        "the page image is the composite without the type"
    );
}

// pdf_live_text
#[test]
fn pdf_live_text_extracts() {
    if !has_tool("pdftotext") || !has_tool("pdffonts") {
        return;
    }
    let (pdf, _) = write_live(&type_doc(), &options());
    let file = temp_file("extract.pdf", &pdf);
    let words = Command::new("pdftotext")
        .arg(&file)
        .arg("-")
        .output()
        .expect("pdftotext");
    let fonts = Command::new("pdffonts")
        .arg(&file)
        .output()
        .expect("pdffonts");
    let _ = std::fs::remove_file(&file);
    assert_eq!(
        String::from_utf8_lossy(&words.stdout).trim(),
        "Kooka Pictura"
    );
    let fonts = String::from_utf8_lossy(&fonts.stdout).into_owned();
    let row = fonts
        .lines()
        .find(|l| l.starts_with("LiberationSans"))
        .unwrap_or_else(|| panic!("font listed: {fonts}"));
    assert!(row.contains("CID TrueType"), "{row}");
    assert!(row.contains("Identity-H"), "{row}");
    // Embedded, not a subset, with a ToUnicode map.
    assert!(row.contains("yes no  yes"), "{row}");
}

// pdf_live_text_renders
#[test]
fn pdf_live_text_renders_like_composite() {
    if !has_tool("pdftoppm") {
        return;
    }
    let doc = type_doc();
    let (pdf, _) = write_live(&doc, &options());
    let file = temp_file("render.pdf", &pdf);
    let out = file.with_extension("out");
    let status = Command::new("pdftoppm")
        .args(["-singlefile", "-r", "288"])
        .arg(&file)
        .arg(&out)
        .status()
        .expect("pdftoppm");
    assert!(status.success());
    let ppm = std::fs::read(out.with_extension("out.ppm")).expect("rendered page");
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_file(out.with_extension("out.ppm"));
    // A binary P6 with a one-line `w h` and a `255` maxval header. Each
    // document pixel renders as a 4 × 4 block whose centre is sampled: poppler
    // filters an image drawn 1:1, so the centres are the exact samples.
    let header: Vec<&[u8]> = ppm.splitn(4, |&b| b == b'\n').take(3).collect();
    assert_eq!(header[1], b"1280 400");
    let offset = header.iter().map(|l| l.len() + 1).sum::<usize>();
    let rgb: Vec<u8> = (0..100)
        .flat_map(|y| (0..320).map(move |x| ((y * 4 + 2) * 1280 + x * 4 + 2) * 3))
        .flat_map(|at| ppm[offset + at..offset + at + 3].to_vec())
        .collect();

    let plane = doc.composite.pixel_count();
    let expected: Vec<u8> = (0..plane)
        .flat_map(|i| (0..3).map(move |c| (i, c)))
        .map(|(i, c)| doc.composite.data[c * plane + i])
        .collect();
    let mean = rgb
        .iter()
        .zip(&expected)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / expected.len() as f64;
    assert!(
        mean < 3.0,
        "glyph edges aside, the page matches (mean {mean})"
    );
    let ink: Vec<usize> = (0..plane)
        .filter(|&i| expected[i * 3..i * 3 + 3] == [255, 0, 0])
        .collect();
    assert!(ink.len() > 200, "the type has solid ink");
    let red = ink
        .iter()
        .filter(|&&i| rgb[i * 3] > 200 && rgb[i * 3 + 1] < 60 && rgb[i * 3 + 2] < 60)
        .count();
    assert!(
        red * 10 >= ink.len() * 9,
        "{red} of {} ink pixels render red",
        ink.len()
    );
}

// pdf_live_text_covered
#[test]
fn pdf_covered_type_stays_in_image() {
    let mut doc = type_doc();
    let mut cover = doc.layers[0].clone();
    cover.name = "cover".into();
    let _ = doc.layers.push(cover);
    doc.composite = pictura_render::composite_rgba(&doc);
    let (pdf, background) = write_live(&doc, &options());
    assert!(background.is_none());
    assert!(!page_text(&pdf).contains("/Font"));
    assert!(same_rgb(&image(&pdf), &doc.composite));
}

// pdf_live_text_opacity
#[test]
fn pdf_faded_type_follows_transparency() {
    let mut doc = type_doc();
    let index = doc.layers.iter().position(|l| l.is_type()).unwrap();
    doc.layers[index].opacity = 128;
    doc.composite = pictura_render::composite_rgba(&doc);
    let at = |compatibility| PdfOptions {
        compatibility,
        ..options()
    };
    let (pdf, _) = write_live(&doc, &at(Compatibility::Acrobat6));
    let page = page_text(&pdf);
    assert!(
        page.contains("/ExtGState <</GS0 <</Type /ExtGState/ca 0.502/CA 0.502>>>>"),
        "{page}"
    );
    assert!(content(&pdf).contains("/GS0 gs"));
    assert!(page.contains("/FontFile2 "));
    let (pdf, background) = write_live(&doc, &at(Compatibility::Acrobat4));
    assert!(background.is_none(), "Acrobat 4 keeps no transparency");
    assert!(!page_text(&pdf).contains("/Font"));
}

// pdf_live_text_colour
#[test]
fn pdf_live_text_colour_follows_conversion() {
    let options = PdfOptions {
        color_conversion: ColorConversion::ToDestination,
        destination: Destination::CoatedCmyk,
        profile_inclusion: ProfileInclusion::IncludeDestination,
        ..options()
    };
    let (pdf, _) = write_live(&type_doc(), &options);
    assert!(page_text(&pdf).contains("/ColorSpace <</CS0 [/ICCBased "));
    let ops = content(&pdf);
    let fill = ops
        .lines()
        .find_map(|l| l.strip_prefix("/CS0 cs "))
        .and_then(|l| l.strip_suffix(" scn"))
        .unwrap_or_else(|| panic!("an ICC-based fill: {ops}"));
    let cmyk: Vec<f64> = fill.split(' ').map(|v| v.parse().unwrap()).collect();
    assert_eq!(cmyk.len(), 4, "{fill}");
    assert!(
        cmyk[0] < 0.2 && cmyk[1] > 0.6 && cmyk[2] > 0.6,
        "red is magenta and yellow ink: {fill}"
    );
}
