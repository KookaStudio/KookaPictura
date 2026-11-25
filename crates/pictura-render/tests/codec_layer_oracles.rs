use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use pictura_codec::{read_psd, write_psb, write_psd};
use pictura_core::Document;
use pictura_render::{
    add_shape_layer, add_solid_fill, add_vector_mask, rasterize_fill_content, rasterize_shape,
    resolve_path, resolve_path_mut, set_layer_shape_paths, set_shape_fill, set_shape_stroke,
    ShapeStroke, StrokePosition, VectorMaskKind,
};

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
    if block is None and blocks:
        block = blocks.get(Tag.VECTOR_MASK_SETTING2)
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

const KIND_SCRIPT: &str = r#"
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

fn read_with_psd_tools(path: &std::path::Path) -> (String, Vec<Vec<String>>) {
    run_psd_tools(SCRIPT, path)
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pictura-codec/tests/fixtures")
        .join(name)
}

fn read_fixture(name: &str) -> Document {
    let bytes = std::fs::read(fixture_path(name)).expect("read fixture");
    read_psd(&bytes).expect("fixture parses")
}

fn row(lines: &[Vec<String>], name: &str, stdout: &str) -> Vec<String> {
    lines
        .iter()
        .find(|line| line.first().is_some_and(|field| field.as_str() == name))
        .unwrap_or_else(|| panic!("no {name} row in:\n{stdout}"))
        .clone()
}

fn expected(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|field| (*field).to_owned()).collect()
}

fn write_temp(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pictura-{tag}-{}-{}.psd",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::write(&path, bytes).expect("write temp PSD");
    path
}

fn rectangle() -> pictura_core::path::Subpath {
    let options =
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3);
    pictura_core::shape::outline(options, (1.0, 2.0), (5.0, 7.0), false, false)
        .expect("a rectangle")
}

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
    add_shape_layer(&mut doc, "", [255, 0, 0, 255], "Rectangle", &outline, None);
    let bytes = write_psd(&doc).expect("writes");
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
    add_shape_layer(
        &mut doc,
        "",
        [255, 0, 0, 255],
        "Rounded Rectangle",
        &outline,
        Some(&live),
    );
    let path = std::env::temp_dir().join(format!("pictura-live-{}.psd", std::process::id()));
    std::fs::write(&path, write_psd(&doc).expect("writes")).expect("write");
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
    let path = add_shape_layer(&mut doc, "", [0, 0, 255, 255], "Rectangle", &outline, None);
    let layer = resolve_path_mut(&mut doc, &path).unwrap();
    assert!(set_shape_fill(layer, None));
    let stroke = ShapeStroke::solid([10, 200, 30], 3, StrokePosition::Inside);
    assert!(set_shape_stroke(layer, Some(&stroke)));
    let file = std::env::temp_dir().join(format!("pictura-stroke-{}.psd", std::process::id()));
    std::fs::write(&file, write_psd(&doc).expect("writes")).expect("write");
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

#[test]
fn an_authored_vector_mask_survives_save_and_reopen() {
    let bytes = std::fs::read(fixture_path("derived_raster_mask.psd"))
        .expect("read derived_raster_mask.psd");
    let mut doc = read_psd(&bytes).expect("derived_raster_mask.psd parses");
    assert!(
        add_vector_mask(&mut doc, "0", VectorMaskKind::RevealAll),
        "the suppressed layer accepts a live vector mask"
    );

    let back = read_psd(&write_psd(&doc).expect("writes")).expect("re-reads");
    assert!(
        back.layers[0].vector_mask.is_some(),
        "the authored live mask survives a save/reopen"
    );
    assert_eq!(
        back.layers[0].mask.as_ref().expect("mask").flags & 0x08,
        0,
        "the writer clears the render-source marker for a live mask"
    );
}

#[test]
fn codec_does_not_add_the_flag_to_ordinary_fill_or_vector_masked_pixel_layers() {
    let mut doc = Document::from_rgba("Base", 8, 8, &[255; 8 * 8 * 4]);
    let fill = add_solid_fill(&mut doc, "", [255, 0, 0, 255]);
    assert!(!resolve_path(&doc, &fill).unwrap().pixel_data_irrelevant);

    let base_path = doc
        .layers
        .iter()
        .position(|layer| layer.name == "Base")
        .expect("Base layer")
        .to_string();
    let (width, height) = (doc.width, doc.height);
    let outline = rectangle();
    {
        let base = resolve_path_mut(&mut doc, &base_path).expect("Base layer");
        assert!(base.adjustment().is_none(), "no fill adjustment");
        set_layer_shape_paths(base, std::slice::from_ref(&outline), width, height);
        assert!(!base.pixel_data_irrelevant);
    }

    for bytes in [
        write_psd(&doc).expect("writes PSD"),
        write_psb(&doc).expect("writes PSB"),
    ] {
        let back = read_psd(&bytes).expect("re-reads");
        assert!(back.layers.iter().all(|layer| !layer.pixel_data_irrelevant));
    }
}

#[test]
fn psd_tools_reports_a_rasterized_shape_as_pixels() {
    let mut doc = read_fixture("vector_fill.psd");
    let shape_path = doc
        .layers
        .iter()
        .position(|layer| layer.name == "Shape")
        .expect("Shape layer")
        .to_string();
    assert!(rasterize_shape(&mut doc, &shape_path));
    assert!(
        !resolve_path(&doc, &shape_path)
            .unwrap()
            .pixel_data_irrelevant,
        "Rasterize Shape clears the flag"
    );

    if psd_tools_available() {
        let path = write_temp("shape-flag-raster-shape", &write_psd(&doc).unwrap());
        let (stdout, lines) = run_psd_tools(KIND_SCRIPT, &path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            row(&lines, "Shape", &stdout),
            expected(&["Shape", "pixel", "00"])
        );
    }

    let mut authored = Document::from_rgba("Background", 8, 8, &[255; 8 * 8 * 4]);
    let outline = rectangle();
    let path = add_shape_layer(
        &mut authored,
        "",
        [255, 0, 0, 255],
        "Rectangle",
        &outline,
        None,
    );
    assert!(
        resolve_path(&authored, &path)
            .unwrap()
            .pixel_data_irrelevant
    );
    assert!(rasterize_fill_content(&mut authored, &path));
    assert!(
        !resolve_path(&authored, &path)
            .unwrap()
            .pixel_data_irrelevant,
        "Rasterize Fill Content clears the flag"
    );

    if psd_tools_available() {
        let bytes = write_psd(&authored).unwrap();
        let path = write_temp("shape-flag-raster-fill", &bytes);
        let (stdout, lines) = run_psd_tools(KIND_SCRIPT, &path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            row(&lines, "Rectangle 1", &stdout),
            expected(&["Rectangle 1", "pixel", "00"])
        );
    }
}
