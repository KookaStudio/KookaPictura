//! Independent `psd-tools` oracle for the `vmsk` vector-mask fixture and for
//! the shape layers the shape tools author.
//!
//! `oracle.rs` is at its 1400-line cap, so this check lives in its own binary.
//! It reads a PSD with `psd-tools` (not with `pictura-codec`) and reports each
//! vector mask's version, flags, closed subpath, and the even-odd pixel
//! coverage of its anchors' polygon.
//!
//! Needs `python3` with `psd-tools`; self-skips with a message otherwise.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1], lazy=False)
w, h = psd.width, psd.height
lines = []
for layer in psd:
    blocks = layer.tagged_blocks
    block = blocks.get(Tag.VECTOR_MASK_SETTING1) if blocks else None
    if block is None:
        continue
    mask = block.data
    closed = [item for item in mask.path if type(item).__name__ == "ClosedPath"]
    knots = []
    for subpath in closed:
        for knot in subpath:
            knots.append((knot.anchor[1] * w, knot.anchor[0] * h))
    inside = 0
    for py in range(h):
        for px in range(w):
            cx, cy = px + 0.5, py + 0.5
            crossings = 0
            for i in range(len(knots)):
                x0, y0 = knots[i]
                x1, y1 = knots[(i + 1) % len(knots)]
                if (y0 > cy) != (y1 > cy):
                    xint = x0 + (cy - y0) * (x1 - x0) / (y1 - y0)
                    if cx < xint:
                        crossings += 1
            if crossings % 2 == 1:
                inside += 1
    operation = closed[0].operation if closed else -99
    fields = [layer.name, str(mask.version), str(mask.flags), str(len(closed)),
              str(operation), str(inside)]
    fields.append(" ".join(f"{x:g},{y:g}" for x, y in knots))
    lines.append("|".join(fields))
print("\n".join(lines))
"#;

fn psd_tools_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args(["-c", "import psd_tools"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// One `|`-separated row per vector-mask layer, as `SCRIPT` prints them.
fn read_with_psd_tools(path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    run_psd_tools(SCRIPT, path)
}

fn run_psd_tools(script: &str, path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    let out = Command::new("python3")
        .args(["-c", script, path.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{stderr}"
    );
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split('|').map(str::to_owned).collect())
        .collect();
    (stdout, lines)
}

#[test]
fn psd_tools_reads_vector_mask_fixture() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_mask.psd");
    let (stdout, lines) = read_with_psd_tools(&fixture);
    assert_eq!(lines.len(), 2, "two vector-mask layers:\n{stdout}");

    assert_eq!(
        lines[0],
        vec![
            "Shape Inverted",
            "3",
            "1",
            "1",
            "1",
            "36",
            "0,0 6,0 6,6 0,6"
        ],
        "inverted (0,0)-(6,6) rectangle: {stdout}"
    );
    assert_eq!(
        lines[1],
        vec!["Shape", "3", "0", "1", "1", "4", "1,1 3,1 3,3 1,3"],
        "normal (1,1)-(3,3) rectangle: {stdout}"
    );
}

const AUTHORED_SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage

psd = PSDImage.open(sys.argv[1], lazy=False)
for layer in psd:
    f = layer._record.flags
    raw = ((f.transparency_protected * 1)
           | ((not f.visible) * 2)
           | (f.obsolete * 4)
           | (f.photoshop_v5_later * 8)
           | (f.pixel_data_irrelevant * 16)
           | (f.undocumented_1 * 32)
           | (f.undocumented_2 * 64)
           | (f.undocumented_3 * 128))
    print("|".join([layer.name, layer.kind, "%02x" % raw]))
"#;

#[test]
fn psd_tools_reads_an_authored_shape_layer() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = pictura_core::Document::from_rgba("Background", 8, 8, &[255; 8 * 8 * 4]);
    let options =
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3);
    let outline = pictura_core::shape::outline(options, (1.0, 2.0), (5.0, 7.0), false, false)
        .expect("a rectangle");
    pictura_render::add_shape_layer(&mut doc, "", [255, 0, 0, 255], "Rectangle", &outline, None);
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let path = std::env::temp_dir().join(format!("pictura-shape-{}.psd", std::process::id()));
    std::fs::write(&path, bytes).expect("write temp PSD");
    let (stdout, lines) = read_with_psd_tools(&path);
    let (kind_stdout, kind_lines) = run_psd_tools(AUTHORED_SCRIPT, &path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        lines,
        vec![vec![
            "Rectangle 1",
            "3",
            "0",
            "1",
            "1",
            "20",
            "1,2 5,2 5,7 1,7"
        ]],
        "one 4x5 rectangle at (1,2): {stdout}"
    );
    let shape = kind_lines
        .iter()
        .find(|fields| {
            fields
                .first()
                .is_some_and(|name| name.as_str() == "Rectangle 1")
        })
        .unwrap_or_else(|| panic!("no authored shape row in:\n{kind_stdout}"));
    assert_eq!(shape[1], "shape");
    assert_eq!(shape[2], "18", "flags 0x18: {kind_stdout}");
}

const VOGK_SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1])
for layer in psd:
    blocks = layer.tagged_blocks
    block = blocks.get(Tag.VECTOR_ORIGINATION_DATA) if blocks else None
    if block is None:
        continue
    shape = block.data[b"keyDescriptorList"][0]
    box = shape[b"keyOriginShapeBBox"]
    radii = shape[b"keyOriginRRectRadii"]
    print("|".join([layer.name, str(int(shape[b"keyOriginType"])),
                    " ".join(f"{float(box[k]):g}" for k in (b"Left", b"Top ", b"Rght", b"Btom")),
                    " ".join(f"{float(radii[k]):g}" for k in
                             (b"topLeft", b"topRight", b"bottomRight", b"bottomLeft"))]))
"#;

#[test]
fn psd_tools_reads_an_authored_live_shape() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = pictura_core::Document::from_rgba("Background", 8, 8, &[255; 8 * 8 * 4]);
    let options = pictura_core::shape::ShapeOptions::new(
        pictura_core::shape::ShapeKind::RoundedRectangle,
        2.0,
        3,
    );
    let outline = pictura_core::shape::outline_in_box(options, (1.0, 2.0, 5.0, 4.0)).unwrap();
    let live = pictura_codec::LiveShape {
        origin_type: pictura_codec::ORIGIN_ROUNDED_RECTANGLE,
        bounds: (1.0, 2.0, 6.0, 6.0),
        radii: [2.0, 2.0, 2.0, 1.0],
    };
    pictura_render::add_shape_layer(
        &mut doc,
        "",
        [255, 0, 0, 255],
        "Rounded Rectangle",
        &outline,
        Some(&live),
    );
    let path = std::env::temp_dir().join(format!("pictura-live-{}.psd", std::process::id()));
    std::fs::write(&path, pictura_codec::write_psd(&doc).expect("writes")).expect("write");
    let out = Command::new("python3")
        .args(["-c", VOGK_SCRIPT, path.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout.trim(),
        "Rounded Rectangle 1|2|1 2 6 6|2 2 2 1",
        "the live rounded rectangle's origin data"
    );
}

const STROKE_SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import Tag

psd = PSDImage.open(sys.argv[1])
for layer in psd:
    blocks = layer.tagged_blocks
    block = blocks.get(Tag.OBJECT_BASED_EFFECTS_LAYER_INFO) if blocks else None
    if block is None:
        continue
    fx = block.data[b"FrFX"]
    color = fx[b"Clr "]
    print("|".join([layer.name, str(bool(fx[b"enab"])), str(fx[b"Styl"].enum),
                    f"{float(fx[b'Sz  ']):g}",
                    " ".join(f"{float(color[k]):g}" for k in (b"Rd  ", b"Grn ", b"Bl  ")),
                    str(layer.fill_opacity if hasattr(layer, "fill_opacity") else "")]))
"#;

#[test]
fn psd_tools_reads_a_shape_stroke_and_no_fill() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 + psd-tools not available");
        return;
    }

    let mut doc = pictura_core::Document::from_rgba("Background", 8, 8, &[255; 8 * 8 * 4]);
    let options =
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3);
    let outline = pictura_core::shape::outline_in_box(options, (1.0, 1.0, 6.0, 6.0)).unwrap();
    let path = pictura_render::add_shape_layer(
        &mut doc,
        "",
        [0, 0, 255, 255],
        "Rectangle",
        &outline,
        None,
    );
    let layer = pictura_render::resolve_path_mut(&mut doc, &path).unwrap();
    assert!(pictura_render::set_shape_fill(layer, None));
    let stroke = pictura_render::ShapeStroke {
        color: [10, 200, 30],
        width: 3,
        position: pictura_render::StrokePosition::Inside,
    };
    assert!(pictura_render::set_shape_stroke(layer, Some(&stroke)));
    let file = std::env::temp_dir().join(format!("pictura-stroke-{}.psd", std::process::id()));
    std::fs::write(&file, pictura_codec::write_psd(&doc).expect("writes")).expect("write");
    let out = Command::new("python3")
        .args(["-c", STROKE_SCRIPT, file.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&file);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let fields: Vec<&str> = stdout.trim().split('|').collect();
    assert_eq!(
        &fields[..5],
        &["Rectangle 1", "True", "b'InsF'", "3", "10 200 30"],
        "the stroke effect: {stdout}"
    );
    assert_eq!(fields[5], "0", "Fill None is fill opacity 0: {stdout}");
}
