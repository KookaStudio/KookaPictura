//! `eps` tests; renders self-skip when Ghostscript is absent.

use super::*;
use crate::ghostscript::ScratchDir;

const BLUE_EPS: &str = "%!PS-Adobe-3.0 EPSF-3.0\n\
%%BoundingBox: 10 10 154 82\n\
%%EndComments\n\
0 0 1 setrgbcolor 10 10 144 72 rectfill\n\
showpage\n\
%%EOF\n";

/// `ps` inside a DOS binary EPS header with a dummy TIFF preview after it.
fn dos_wrapped(ps: &[u8]) -> Vec<u8> {
    let tiff = b"II*\0dummy-preview";
    let ps_at = 30u32;
    let tiff_at = ps_at + ps.len() as u32;
    let mut out = DOS_EPS_MAGIC.to_vec();
    for word in [ps_at, ps.len() as u32, 0, 0, tiff_at, tiff.len() as u32] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(ps);
    out.extend_from_slice(tiff);
    out
}

fn assert_all_blue(raster: &crate::Raster) {
    assert_eq!((raster.width, raster.height), (144, 72));
    for (x, y) in [(0, 0), (143, 0), (0, 71), (143, 71), (72, 36)] {
        let at = (y * 144 + x) * 4;
        assert_eq!(&raster.rgba[at..at + 4], &[0, 0, 255, 255], "({x}, {y})");
    }
}

#[test]
fn eps_bounding_box_prefers_hires() {
    assert_eq!(
        bounding_box(BLUE_EPS.as_bytes()),
        Some(PdfRect::new(10.0, 10.0, 154.0, 82.0))
    );
    let hires = "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 145 73\n%%HiResBoundingBox: 0.5 0.25 144.5 72.25\n";
    assert_eq!(
        bounding_box(hires.as_bytes()),
        Some(PdfRect::new(0.5, 0.25, 144.5, 72.25))
    );
    let atend =
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: (atend)\n%%Trailer\n%%BoundingBox: 0 0 20 10\n";
    assert_eq!(
        bounding_box(atend.as_bytes()),
        Some(PdfRect::new(0.0, 0.0, 20.0, 10.0))
    );
    assert_eq!(bounding_box(b"%!PS-Adobe-3.0\n"), None);
}

#[test]
fn eps_renders_bounding_box() {
    let Ok(gs) = Ghostscript::locate() else {
        eprintln!("skipping: Ghostscript (gs) not found");
        return;
    };
    let scratch = ScratchDir::new().unwrap();
    let path = scratch.path("blue.eps");
    std::fs::write(&path, BLUE_EPS).unwrap();
    let bounds = bounding_box(BLUE_EPS.as_bytes()).unwrap();
    let out = render_eps(&gs, &path, bounds, (144, 72), true).expect("render");
    assert_all_blue(&out.raster);
}

#[test]
fn dos_eps_header_is_unwrapped() {
    let wrapped = dos_wrapped(BLUE_EPS.as_bytes());
    assert_eq!(postscript_section(&wrapped), Some(BLUE_EPS.as_bytes()));
    assert_eq!(
        bounding_box(&wrapped),
        Some(PdfRect::new(10.0, 10.0, 154.0, 82.0))
    );
    assert_eq!(postscript_section(&wrapped[..20]), None, "truncated header");
    let Ok(gs) = Ghostscript::locate() else {
        eprintln!("skipping: Ghostscript (gs) not found");
        return;
    };
    let scratch = ScratchDir::new().unwrap();
    let path = scratch.path("dos.eps");
    std::fs::write(&path, &wrapped).unwrap();
    let out =
        render_eps(&gs, &path, bounding_box(&wrapped).unwrap(), (144, 72), true).expect("render");
    assert_all_blue(&out.raster);
}
