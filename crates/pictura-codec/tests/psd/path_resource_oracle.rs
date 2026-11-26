//! Independent `psd-tools` oracle for the Work Path (1025) and saved-path
//! (2000-2997) image resources `pictura-codec` writes.
//!
//! The document is built and written with `pictura-codec`, then read back with
//! `psd-tools` (its own path-record parser, `psd_tools.psd.vector.Path`), which
//! reports each path resource's id, name, and per-subpath
//! closed flag and anchors in document pixels.
//!
//! Needs `python3` with `psd-tools`; self-skips with a message otherwise.

use std::process::Command;

use pictura_core::path::{NamedPath, PathPoint, Subpath, VectorPath};
use pictura_core::{BitDepth, ColorMode, Document};

const SCRIPT: &str = r#"
import sys
from psd_tools import PSDImage
from psd_tools.psd.vector import Path

psd = PSDImage.open(sys.argv[1])
w, h = psd.width, psd.height
for key in sorted(psd.image_resources.keys()):
    rid = int(key)
    if not (rid == 1025 or 2000 <= rid <= 2997):
        continue
    resource = psd.image_resources[key]
    subpaths = []
    for item in Path.frombytes(resource.data):
        kind = type(item).__name__
        if kind not in ("ClosedPath", "OpenPath"):
            continue
        anchors = " ".join(
            f"{knot.anchor[1] * w:.3f},{knot.anchor[0] * h:.3f}" for knot in item)
        subpaths.append(("closed" if kind == "ClosedPath" else "open") + ":" + anchors)
    print("|".join([str(rid), resource.name] + subpaths))
"#;

fn psd_tools_available() -> bool {
    Command::new("python3")
        .args(["-c", "import psd_tools"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn subpath(points: &[(f64, f64)], closed: bool) -> Subpath {
    Subpath {
        points: points
            .iter()
            .map(|&anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .collect(),
        closed,
    }
}

fn path(subpaths: Vec<Subpath>) -> VectorPath {
    let mut path = VectorPath::default();
    for s in subpaths {
        path.add_subpath(s);
    }
    path
}

#[test]
fn psd_tools_reads_the_written_work_and_saved_paths() {
    if !psd_tools_available() {
        eprintln!("skipping: python3 with psd-tools is not available");
        return;
    }
    let mut doc = Document::new(200, 100, ColorMode::Rgb, BitDepth::Eight);
    doc.work_path = path(vec![subpath(
        &[(20.0, 10.0), (180.0, 10.0), (100.0, 90.0)],
        true,
    )]);
    doc.saved_paths = vec![
        NamedPath {
            name: "Outline".into(),
            path: path(vec![
                subpath(&[(50.0, 25.0), (150.0, 75.0)], false),
                subpath(
                    &[(10.0, 10.0), (40.0, 10.0), (40.0, 40.0), (10.0, 40.0)],
                    true,
                ),
            ]),
        },
        NamedPath {
            name: "Path 2".into(),
            path: path(vec![subpath(&[(0.0, 0.0), (200.0, 100.0)], false)]),
        },
    ];
    let bytes = pictura_codec::write_psd(&doc).expect("write");
    let dir = std::env::temp_dir().join(format!("pictura-path-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("paths.psd");
    std::fs::write(&file, &bytes).expect("write file");

    let out = Command::new("python3")
        .args(["-c", SCRIPT, file.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        rows,
        [
            "1025||closed:20.000,10.000 180.000,10.000 100.000,90.000",
            "2000|Outline|open:50.000,25.000 150.000,75.000\
             |closed:10.000,10.000 40.000,10.000 40.000,40.000 10.000,40.000",
            "2001|Path 2|open:0.000,0.000 200.000,100.000",
        ]
    );
}
