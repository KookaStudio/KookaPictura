//! External-tool oracles: Ghostscript and poppler render the PDF for a pixel
//! comparison with the composite; qpdf checks structure, linearization, and
//! encryption. Each self-skips when its tool is not on `PATH` (CI installs
//! `ghostscript`, `poppler-utils`, and `qpdf`).

use std::path::PathBuf;
use std::process::Command;

use pictura_core::{Document, MarkerKind};

use super::{plain_options, rgb_doc, write};
use crate::pdf::{Compatibility, Compression, PdfOptions, Security};

pub(super) fn has_tool(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

fn skip(tool: &str) -> bool {
    if has_tool(tool) {
        return false;
    }
    eprintln!("SKIP: `{tool}` is not installed");
    true
}

pub(super) fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pictura-pdf-{}-{name}", std::process::id()));
    std::fs::write(&path, bytes).expect("temp file");
    path
}

/// Parse a binary PPM (P6) into `(width, height, rgb)`.
fn parse_ppm(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut fields = Vec::new();
    let mut at = 2;
    while fields.len() < 3 {
        while bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if bytes[at] == b'#' {
            while bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        let start = at;
        while !bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        fields.push(
            std::str::from_utf8(&bytes[start..at])
                .unwrap()
                .parse::<usize>()
                .unwrap(),
        );
    }
    let (w, h) = (fields[0], fields[1]);
    (w, h, bytes[at + 1..at + 1 + w * h * 3].to_vec())
}

/// Each document pixel renders as a `SCALE × SCALE` block whose centre is
/// sampled: renderers upsample an image by replication but may filter it at
/// 1:1 (poppler does), so the centres are the exact samples.
const SCALE: usize = 4;

/// Render page 1 at `ppi` with Ghostscript (`gs`: profile-free device colour
/// via `-dUseFastColor`; `gs-icc`: colour-managed) or poppler (`pdftoppm`),
/// optionally with a user password; returns the sampled `(width, height, rgb)`.
pub(super) fn render(
    pdf: &[u8],
    ppi: f64,
    engine: &str,
    password: Option<&str>,
) -> (usize, usize, Vec<u8>) {
    let tag = format!("{engine}-{}", pdf.len());
    let input = temp_file(&format!("{tag}.pdf"), pdf);
    let output = std::env::temp_dir().join(format!("pictura-pdf-{}-{tag}", std::process::id()));
    let program = if engine.starts_with("gs") {
        "gs"
    } else {
        engine
    };
    let mut command = Command::new(program);
    let dpi = ppi * SCALE as f64;
    let ppm = if program == "gs" {
        let ppm = output.with_extension("ppm");
        command.args(["-q", "-dNOPAUSE", "-dBATCH", "-dSAFER", "-sDEVICE=ppmraw"]);
        if engine == "gs" {
            command.arg("-dUseFastColor");
        }
        command.arg(format!("-r{dpi}"));
        if let Some(password) = password {
            command.arg(format!("-sPDFPassword={password}"));
        }
        command.arg(format!("-sOutputFile={}", ppm.display()));
        command.arg(&input);
        ppm
    } else {
        command.args(["-singlefile", "-r", &dpi.to_string()]);
        if let Some(password) = password {
            command.args(["-upw", password]);
        }
        command.arg(&input).arg(&output);
        output.with_extension("ppm")
    };
    let status = command.output().expect("runs the renderer");
    assert!(
        status.status.success(),
        "{engine} failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let bytes = std::fs::read(&ppm).expect("rendered page");
    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&ppm);
    let (w, h, rgb) = parse_ppm(&bytes);
    let (nw, nh) = (w / SCALE, h / SCALE);
    let mut sampled = Vec::with_capacity(nw * nh * 3);
    for y in 0..nh {
        for x in 0..nw {
            let at = ((y * SCALE + SCALE / 2) * w + x * SCALE + SCALE / 2) * 3;
            sampled.extend_from_slice(&rgb[at..at + 3]);
        }
    }
    (nw, nh, sampled)
}

/// The composite flattened onto white as interleaved RGB.
pub(super) fn expected_rgb(doc: &Document) -> Vec<u8> {
    let c = &doc.composite;
    let plane = c.pixel_count();
    let color = if c.channels >= 3 { 3 } else { 1 };
    let mut out = Vec::with_capacity(plane * 3);
    for i in 0..plane {
        let a = if c.channels as usize > color {
            u32::from(c.data[color * plane + i])
        } else {
            255
        };
        for k in 0..3 {
            let v = u32::from(c.data[(k % color) * plane + i]);
            out.push(((v * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    out
}

/// The largest per-sample difference and the mean difference.
pub(super) fn difference(a: &[u8], b: &[u8]) -> (u8, f64) {
    let max = a
        .iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0);
    let sum: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    (max, sum as f64 / a.len().max(1) as f64)
}

/// Render with Ghostscript and require every sample within `tolerance` of
/// the composite flattened onto white. Skips without `gs`.
pub(super) fn assert_renders_like(pdf: &[u8], doc: &Document, ppi: f64, tolerance: u8) {
    if skip("gs") {
        return;
    }
    let (w, h, rgb) = render(pdf, ppi, "gs", None);
    assert_eq!((w, h), (doc.width as usize, doc.height as usize));
    let (max, _) = difference(&rgb, &expected_rgb(doc));
    assert!(max <= tolerance, "rendered pixels differ by up to {max}");
}

fn qpdf(args: &[&str], pdf: &[u8]) -> (i32, String) {
    let file = temp_file(&format!("qpdf-{}.pdf", pdf.len()), pdf);
    let out = Command::new("qpdf")
        .args(args)
        .arg(&file)
        .output()
        .expect("runs qpdf");
    let _ = std::fs::remove_file(&file);
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn pdf_render_matches_composite() {
    let doc = rgb_doc(12, 8, 72.0, Some(5));
    let pdf = write(&doc, &plain_options());
    for engine in ["gs", "pdftoppm"] {
        if skip(engine) {
            continue;
        }
        let (w, h, rgb) = render(&pdf, 72.0, engine, None);
        assert_eq!((w, h), (12, 8), "{engine}");
        let (max, _) = difference(&rgb, &expected_rgb(&doc));
        assert!(max <= 2, "{engine}: rendered pixels differ by up to {max}");
    }
}

fn varied_documents() -> Vec<(&'static str, Document, PdfOptions)> {
    let mut noted = rgb_doc(30, 20, 150.0, Some(3));
    let index = noted.annotations.add(MarkerKind::Note, 4, 4).unwrap();
    noted.annotations.set_note_text(index, "note é");
    let rich = PdfOptions {
        embed_thumbnails: true,
        preserve_editing: true,
        ..plain_options()
    };
    vec![
        ("plain", rgb_doc(9, 7, 72.0, None), plain_options()),
        ("rich", noted, rich),
        ("bitmap", super::bitmap_doc(), plain_options()),
        ("indexed", super::indexed_doc(), plain_options()),
    ]
}

#[test]
fn pdf_qpdf_check_passes() {
    if skip("qpdf") {
        return;
    }
    for compatibility in Compatibility::ALL {
        for (name, doc, options) in varied_documents() {
            let options = PdfOptions {
                compatibility,
                ..options
            };
            let pdf = write(&doc, &options);
            let header = format!("%PDF-1.{}", compatibility.minor());
            assert!(
                pdf.starts_with(header.as_bytes()),
                "{name} {compatibility:?}"
            );
            let (code, out) = qpdf(&["--check"], &pdf);
            assert_eq!(code, 0, "{name} {compatibility:?}: {out}");
            assert!(out.contains("No syntax or stream encoding errors"), "{out}");
        }
    }
}

#[test]
fn pdf_fast_web_view_linearizes() {
    if skip("qpdf") {
        return;
    }
    for (name, doc, options) in varied_documents() {
        for password in ["", "kooka"] {
            for compatibility in [Compatibility::Acrobat5, Compatibility::AcrobatX] {
                let options = PdfOptions {
                    fast_web_view: true,
                    compatibility,
                    security: Security {
                        open_password: password.into(),
                        ..Security::default()
                    },
                    ..options.clone()
                };
                let pdf = write(&doc, &options);
                let mut args = vec!["--check-linearization".to_string()];
                if !password.is_empty() {
                    args.push(format!("--password={password}"));
                }
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                let (code, out) = qpdf(&args, &pdf);
                assert_eq!(code, 0, "{name} {password:?} {compatibility:?}: {out}");
                assert!(out.contains("no linearization errors"), "{name}: {out}");
            }
        }
    }
}

#[test]
fn pdf_open_password_each_level() {
    let doc = rgb_doc(6, 4, 72.0, None);
    for (compatibility, expected) in [
        (Compatibility::Acrobat4, "R = 2"),
        (Compatibility::Acrobat5, "R = 3"),
        (Compatibility::Acrobat6, "R = 3"),
        (Compatibility::Acrobat7, "R = 4"),
        (Compatibility::AcrobatX, "R = 6"),
    ] {
        let options = PdfOptions {
            compatibility,
            compression: Compression::Zip,
            security: Security {
                open_password: "kooka".into(),
                ..Security::default()
            },
            ..plain_options()
        };
        let pdf = write(&doc, &options);
        if !skip("qpdf") {
            let (code, out) = qpdf(&["--check"], &pdf);
            assert_ne!(code, 0, "{compatibility:?} opens without a password: {out}");
            assert!(out.contains("invalid password"), "{out}");
            let (code, out) = qpdf(&["--show-encryption", "--password=kooka"], &pdf);
            assert_eq!(code, 0, "{compatibility:?}: {out}");
            assert!(out.contains(expected), "{compatibility:?}: {out}");
            let (code, out) = qpdf(&["--check", "--password=kooka"], &pdf);
            assert_eq!(code, 0, "{compatibility:?}: {out}");
        }
        if !skip("pdftoppm") {
            let (_, _, rgb) = render(&pdf, 72.0, "pdftoppm", Some("kooka"));
            let (max, _) = difference(&rgb, &expected_rgb(&doc));
            assert!(
                max <= 2,
                "{compatibility:?}: decrypted render differs by {max}"
            );
        }
    }
}

#[test]
fn pdf_permissions_restrict() {
    if skip("qpdf") {
        return;
    }
    let options = PdfOptions {
        security: Security {
            permissions_password: "owner".into(),
            printing: crate::pdf::Printing::None,
            copying: false,
            ..Security::default()
        },
        ..plain_options()
    };
    let pdf = write(&rgb_doc(4, 4, 72.0, None), &options);
    let (code, out) = qpdf(&["--show-encryption"], &pdf);
    assert_eq!(code, 0, "opens without a password: {out}");
    assert!(out.contains("print low resolution: not allowed"), "{out}");
    assert!(
        out.contains("extract for any purpose: not allowed"),
        "{out}"
    );
    assert!(out.contains("extract for accessibility: allowed"), "{out}");
}
