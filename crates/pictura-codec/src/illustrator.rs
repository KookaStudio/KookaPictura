//! Paths to Illustrator: an Illustrator-openable `.ai` file in two encodings.
//!
//! The EPS flavour is a PostScript EPS with a hairline stroke per path; the PDF
//! flavour is a minimal one-page PDF with the same stroked path operators. Both
//! preserve anchors, handles, and open/closed subpaths in document pixels
//! (one pixel = one point, y flipped to the page's bottom-left origin). Saved
//! names are recorded in ASCII-safe comments, not as Illustrator layer names.

use pictura_core::path::VectorPath;

/// One path to write: its recorded name and geometry.
pub struct AiPath<'a> {
    pub name: &'a str,
    pub path: &'a VectorPath,
}

/// Escape `name` so the output stays ASCII and cannot inject a PostScript
/// comment line. Non-ASCII and control bytes become three-digit octal escapes;
/// `\` and `%` are escaped too.
pub fn escape_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        match byte {
            0x20..=0x7e if byte != b'\\' && byte != b'%' => out.push(byte as char),
            b'\\' => out.push_str("\\\\"),
            b'%' => out.push_str("\\045"),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out
}

fn number(value: f64) -> String {
    let mut text = format!("{value:.3}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

fn point(x: f64, y: f64, height: u32) -> String {
    format!("{} {}", number(x), number(f64::from(height) - y))
}

/// The path operators for one `VectorPath`, in EPS or PDF spelling.
fn path_ops(path: &VectorPath, height: u32, pdf: bool) -> Option<String> {
    let (move_to, line_to, curve_to, close, stroke) = if pdf {
        ("m", "l", "c", "h", "S")
    } else {
        ("moveto", "lineto", "curveto", "closepath", "stroke")
    };
    let mut out = String::new();
    let mut wrote = false;
    let discard = if pdf { "n" } else { "newpath" };
    for subpath in &path.subpaths {
        let Some(first) = subpath.points.first() else {
            continue;
        };
        if !pdf {
            out.push_str("newpath\n");
        }
        out.push_str(&point(first.anchor.0, first.anchor.1, height));
        out.push(' ');
        out.push_str(move_to);
        out.push('\n');
        for index in 0..subpath.segment_count() {
            let Some([_, c1, c2, b]) = subpath.segment(index) else {
                continue;
            };
            let first_handle = subpath.points[index].out_handle.is_some();
            let second_handle = subpath.points[(index + 1) % subpath.points.len()]
                .in_handle
                .is_some();
            if first_handle || second_handle {
                out.push_str(&point(c1.0, c1.1, height));
                out.push(' ');
                out.push_str(&point(c2.0, c2.1, height));
                out.push(' ');
                out.push_str(&point(b.0, b.1, height));
                out.push(' ');
                out.push_str(curve_to);
            } else {
                out.push_str(&point(b.0, b.1, height));
                out.push(' ');
                out.push_str(line_to);
            }
            out.push('\n');
            wrote = true;
        }
        if subpath.closed {
            out.push_str(close);
            out.push('\n');
        }
        if subpath.segment_count() == 0 && !subpath.closed {
            // A one-point subpath has no stroked mark; discard it in the
            // dialect's own path syntax.
            out.push_str(discard);
            out.push('\n');
            continue;
        }
        out.push_str(stroke);
        out.push('\n');
        wrote = true;
    }
    wrote.then_some(out)
}

/// Write the Illustrator-8 EPS-based `.ai` bytes.
pub fn write_eps(width: u32, height: u32, paths: &[AiPath<'_>]) -> Vec<u8> {
    let mut out = format!(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%Creator: Kooka Pictura\n%%Title: Paths\n%%Pages: 1\n%%BoundingBox: 0 0 {width} {height}\n%%EndComments\n0 setgray\n0.25 setlinewidth\n"
    );
    for entry in paths {
        out.push_str(&format!("%%Path: {}\n", escape_name(entry.name)));
        if let Some(ops) = path_ops(entry.path, height, false) {
            out.push_str(&ops);
        }
    }
    out.push_str("showpage\n%%EOF\n");
    out.into_bytes()
}

fn object(out: &mut Vec<u8>, offsets: &mut [usize; 5], number: usize, body: &str) {
    offsets[number] = out.len();
    out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
}

/// Write the PDF-based `.ai` bytes.
pub fn write_pdf(width: u32, height: u32, paths: &[AiPath<'_>]) -> Vec<u8> {
    let mut content = String::from("q\n0 G\n0.25 w\n");
    for entry in paths {
        content.push_str(&format!("% {}\n", escape_name(entry.name)));
        if let Some(ops) = path_ops(entry.path, height, true) {
            content.push_str(&ops);
        }
    }
    content.push_str("Q\n");

    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = [0usize; 5];
    object(
        &mut out,
        &mut offsets,
        1,
        "<< /Type /Catalog /Pages 2 0 R >>",
    );
    object(
        &mut out,
        &mut offsets,
        2,
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    object(
        &mut out,
        &mut offsets,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R /Resources << >> >>"
        ),
    );
    object(
        &mut out,
        &mut offsets,
        4,
        &format!(
            "<< /Length {} >>\nstream\n{}endstream",
            content.len(),
            content
        ),
    );

    let xref = out.len();
    out.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::{PathPoint, Subpath};

    fn path(points: Vec<PathPoint>, closed: bool) -> VectorPath {
        let mut path = VectorPath::default();
        path.add_subpath(Subpath { points, closed });
        path
    }

    fn corner(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: (x, y),
            in_handle: None,
            out_handle: None,
            smooth: false,
        }
    }

    fn sample() -> (VectorPath, VectorPath) {
        let work = path(vec![corner(0.0, 0.0), corner(100.0, 0.0)], false);
        let saved = path(
            vec![
                PathPoint {
                    anchor: (0.0, 0.0),
                    in_handle: None,
                    out_handle: Some((10.0, 0.0)),
                    smooth: true,
                },
                PathPoint {
                    anchor: (100.0, 0.0),
                    in_handle: Some((90.0, 0.0)),
                    out_handle: None,
                    smooth: true,
                },
            ],
            true,
        );
        (work, saved)
    }

    #[test]
    fn illustrator_eps_carries_all_paths() {
        let (work, saved) = sample();
        let paths = [
            AiPath {
                name: "Work Path",
                path: &work,
            },
            AiPath {
                name: "Outline",
                path: &saved,
            },
        ];
        let eps = String::from_utf8(write_eps(200, 100, &paths)).unwrap();
        assert!(eps.contains("%%Path: Work Path"));
        assert!(eps.contains("%%Path: Outline"));
        assert!(eps.contains("moveto"));
    }

    #[test]
    fn illustrator_escapes_non_ascii_names() {
        let escaped = escape_name("Café\nX%");
        assert!(escaped.is_ascii());
        assert!(!escaped.contains('\n'));
        assert!(!escaped.contains('%'));
        assert!(escaped.contains("\\012"));
    }

    #[test]
    fn illustrator_pdf_discards_one_point_subpaths_with_n() {
        let mut both = path(vec![corner(0.0, 0.0), corner(50.0, 0.0)], false);
        both.add_subpath(Subpath {
            points: vec![corner(10.0, 10.0)],
            closed: false,
        });
        let pdf = String::from_utf8(write_pdf(
            100,
            100,
            &[AiPath {
                name: "Both",
                path: &both,
            }],
        ))
        .unwrap();
        assert!(!pdf.contains("newpath"));
        assert!(pdf.contains("\nn\n"));
    }

    #[test]
    fn illustrator_keeps_open_and_closed_subpaths() {
        let mut both = path(vec![corner(0.0, 0.0), corner(50.0, 0.0)], false);
        both.add_subpath(Subpath {
            points: vec![corner(0.0, 50.0), corner(20.0, 50.0), corner(20.0, 70.0)],
            closed: true,
        });
        let paths = [AiPath {
            name: "Both",
            path: &both,
        }];
        let eps = String::from_utf8(write_eps(100, 100, &paths)).unwrap();
        assert_eq!(eps.matches("closepath").count(), 1);
        let pdf = String::from_utf8(write_pdf(100, 100, &paths)).unwrap();
        assert_eq!(pdf.matches("\nh\n").count(), 1);
    }

    #[test]
    fn illustrator_keeps_cubic_handles() {
        let (_, saved) = sample();
        let paths = [AiPath {
            name: "Curve",
            path: &saved,
        }];
        let eps = String::from_utf8(write_eps(200, 100, &paths)).unwrap();
        assert!(eps.contains("curveto"));
        assert!(eps.contains("10 100"));
        assert!(eps.contains("90 100"));
    }

    #[test]
    fn illustrator_page_matches_document() {
        let (work, _) = sample();
        let paths = [AiPath {
            name: "Work Path",
            path: &work,
        }];
        let eps = String::from_utf8(write_eps(640, 480, &paths)).unwrap();
        assert!(eps.contains("%%BoundingBox: 0 0 640 480"));
        let pdf = String::from_utf8(write_pdf(640, 480, &paths)).unwrap();
        assert!(pdf.contains("/MediaBox [0 0 640 480]"));
    }

    #[test]
    fn illustrator_eps_header_and_trailer() {
        let (work, _) = sample();
        let eps = String::from_utf8(write_eps(
            100,
            100,
            &[AiPath {
                name: "Work Path",
                path: &work,
            }],
        ))
        .unwrap();
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"));
        assert!(eps.contains("%%Pages: 1\n"));
        assert!(eps.ends_with("showpage\n%%EOF\n"));
    }

    #[test]
    fn illustrator_eps_strokes_paths() {
        let (work, _) = sample();
        let eps = String::from_utf8(write_eps(
            100,
            100,
            &[AiPath {
                name: "Work Path",
                path: &work,
            }],
        ))
        .unwrap();
        assert!(eps.contains("stroke"));
        assert!(eps.contains("0.25 setlinewidth"));
    }

    #[test]
    fn illustrator_eps_is_deterministic() {
        let (work, saved) = sample();
        let paths = [
            AiPath {
                name: "Work Path",
                path: &work,
            },
            AiPath {
                name: "Outline",
                path: &saved,
            },
        ];
        assert_eq!(write_eps(640, 480, &paths), write_eps(640, 480, &paths));
    }

    #[test]
    fn illustrator_pdf_is_one_page() {
        let (work, _) = sample();
        let pdf = String::from_utf8(write_pdf(
            640,
            480,
            &[AiPath {
                name: "Work Path",
                path: &work,
            }],
        ))
        .unwrap();
        assert!(pdf.starts_with("%PDF-1.4\n"));
        assert!(pdf.contains("/Type /Page"));
        assert!(pdf.contains("/Count 1"));
        assert!(pdf.contains(" m\n"));
        assert!(pdf.contains("\nS\n"));
        assert!(pdf.ends_with("%%EOF\n"));
    }

    #[test]
    fn illustrator_pdf_has_no_raster() {
        let (work, _) = sample();
        let pdf = String::from_utf8(write_pdf(
            640,
            480,
            &[AiPath {
                name: "Work Path",
                path: &work,
            }],
        ))
        .unwrap();
        assert!(!pdf.contains("/XObject"));
        assert!(!pdf.contains("/Image"));
    }
}
