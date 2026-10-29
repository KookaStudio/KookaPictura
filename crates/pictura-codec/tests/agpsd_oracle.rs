//! Independent ag-psd structural oracle for `mixr` (Channel Mixer), `curv`
//! (Curves), `selc` (Selective Color), and `clrL` (Color Lookup) payloads.
//!
//! psd-tools' typed `ChannelMixer` reads only the red row (the rest lands in
//! its `unknown` blob), so it cannot see the green/blue/gray channels; its
//! `Curves` reads the v1 prefix but does not name the per-channel rows; its
//! `SelectiveColor` names no plate; its `ColorLookup` exposes the fields but not
//! a typed API. ag-psd reads all four fully, so these tests run it over the
//! committed `channel_mixer.psd`, `curves.psd`, `selective_color.psd`, and
//! `color_lookup.psd` fixtures and assert the authored values.
//!
//! Needs `node` with the `ag-psd` npm package resolvable from the crate's
//! parents: `npm i ag-psd` at the repo root, or point `NODE_PATH` at an existing
//! install. The tests self-skip with a message when either is absent, mirroring
//! the psd-tools/magick oracles.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const a = layer.adjustment;
    if (a && a.type === 'channel mixer') {
      if (a.monochrome) {
        const g = a.gray;
        lines.push([1, g.red, g.green, g.blue, g.constant].join(' '));
      } else {
        const r = a.red, g = a.green, b = a.blue, y = a.gray;
        lines.push([0, r.red, r.green, r.blue, r.constant, g.red, g.green, g.blue, g.constant,
          b.red, b.green, b.blue, b.constant, y.red, y.green, y.blue, y.constant].join(' '));
      }
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

fn node_ag_psd_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("node")
            .args(["-e", "require('ag-psd')"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

#[test]
fn ag_psd_reads_channel_mixer_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/channel_mixer.psd");
    let out = Command::new("node")
        .args(["-e", SCRIPT, fixture.to_str().expect("utf-8 fixture path")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<Vec<i64>> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|token| token.parse::<i64>().expect("integer field"))
                .collect()
        })
        .collect();
    assert_eq!(lines.len(), 2, "two channel mixer layers:\n{stdout}");
    assert_eq!(
        lines[0],
        vec![0, 30, -10, 50, 5, 10, 90, 0, -20, 0, 20, 110, 40, 100, 0, 0, 0],
        "non-monochrome red/green/blue/gray: {stdout}"
    );
    assert_eq!(
        lines[1],
        vec![1, 20, 40, 60, -15],
        "monochrome gray: {stdout}"
    );
}

const CURVES_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const a = layer.adjustment;
    if (a && a.type === 'curves') {
      const parts = [];
      // ag-psd <= 32 exposes rgb/red/green/blue; 33 replaced them with a
      // channels array that omits absent channels.
      const channels = Array.isArray(a.channels)
        ? [a.channels[0] || [], a.channels[1] || [], a.channels[2] || [], a.channels[3] || []]
        : [a.rgb || [], a.red || [], a.green || [], a.blue || []];
      for (const c of channels) {
        parts.push(c.length);
        for (const p of c) parts.push(p.output, p.input);
      }
      lines.push(parts.join(' '));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_curves_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/curves.psd");
    let out = Command::new("node")
        .args([
            "-e",
            CURVES_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<Vec<i64>> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|token| token.parse::<i64>().expect("integer field"))
                .collect()
        })
        .collect();
    assert_eq!(lines.len(), 2, "two curves layers:\n{stdout}");
    assert_eq!(
        lines[0],
        vec![4, 0, 0, 32, 64, 224, 192, 255, 255, 0, 0, 0],
        "composite rgb (output, input) then empty red/green/blue: {stdout}"
    );
    assert_eq!(
        lines[1],
        vec![
            2, 0, 0, 255, 255, 3, 0, 0, 255, 128, 255, 255, 3, 0, 0, 16, 64, 255, 255, 2, 255, 0,
            0, 255
        ],
        "per-channel rgb/red/green/blue (output, input): {stdout}"
    );
}

const SELECTIVE_COLOR_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const ranges = ['reds', 'yellows', 'greens', 'cyans', 'blues', 'magentas', 'whites', 'neutrals', 'blacks'];
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const a = layer.adjustment;
    if (a && a.type === 'selective color') {
      const parts = [a.mode === 'absolute' ? 1 : 0];
      for (const name of ranges) {
        const r = a[name];
        parts.push(r.c, r.m, r.y, r.k);
      }
      lines.push(parts.join(' '));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_selective_color_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/selective_color.psd");
    let out = Command::new("node")
        .args([
            "-e",
            SELECTIVE_COLOR_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<Vec<i64>> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|token| token.parse::<i64>().expect("integer field"))
                .collect()
        })
        .collect();
    assert_eq!(lines.len(), 2, "two selective color layers:\n{stdout}");
    assert_eq!(
        lines[0],
        vec![
            0, 10, -20, 30, 0, 0, 0, 0, 5, -10, 0, 0, 0, 0, 15, 0, 0, 0, 0, -25, 0, 5, 0, 0, 0, 0,
            0, 0, 0, 20, -10, 0, 0, 0, 0, 0, -40
        ],
        "relative reds..blacks: {stdout}"
    );
    assert_eq!(
        lines[1],
        vec![
            1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36
        ],
        "absolute reds..blacks: {stdout}"
    );
}

const VECTOR_MASK_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const vm = layer.vectorMask;
    if (vm) {
      const path = vm.paths[0];
      const parts = [layer.name, String(vm.invert), String(path.open), path.fillRule, path.operation];
      for (const knot of path.knots) parts.push(knot.points.join(','));
      lines.push(parts.join('|'));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_vector_mask_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_mask.psd");
    let out = Command::new("node")
        .args([
            "-e",
            VECTOR_MASK_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "two vector-mask layers:\n{stdout}");
    assert_eq!(
        lines[0],
        "Shape Inverted|true|false|even-odd|combine|0,0,0,0,0,0|6,0,6,0,6,0|6,6,6,6,6,6|0,6,0,6,0,6",
        "inverted rectangle: {stdout}"
    );
    assert_eq!(
        lines[1],
        "Shape|false|false|even-odd|combine|1,1,1,1,1,1|3,1,3,1,3,1|3,3,3,3,3,3|1,3,1,3,1,3",
        "normal rectangle: {stdout}"
    );
}

const VECTOR_FILL_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const vf = layer.vectorFill;
    if (vf && vf.type === 'color') {
      lines.push([layer.name, vf.color.r, vf.color.g, vf.color.b].join('|'));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_vector_fill_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vector_fill.psd");
    let out = Command::new("node")
        .args([
            "-e",
            VECTOR_FILL_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one vector-fill layer:\n{stdout}");
    assert_eq!(
        lines[0], "Shape|255|0|0",
        "solid color vector fill: {stdout}"
    );
}

const COLOR_LOOKUP_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const a = layer.adjustment;
    if (a && a.type === 'color lookup') {
      const d = a.lut3DFileData || new Uint8Array();
      lines.push([a.lookupType, a.lutFormat, a.dataOrder, a.tableOrder, a.dither ? 1 : 0,
        Buffer.from(d).toString('hex')].join(' '));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_color_lookup_fixture() {
    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/color_lookup.psd");
    let out = Command::new("node")
        .args([
            "-e",
            COLOR_LOOKUP_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );

    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one color lookup layer:\n{stdout}");
    let fields: Vec<&str> = lines[0].split_whitespace().collect();
    assert_eq!(fields[0], "3dlut", "lookupType: {stdout}");
    assert_eq!(fields[1], "cube", "lutFormat: {stdout}");
    assert_eq!(fields[2], "rgb", "dataOrder: {stdout}");
    assert_eq!(fields[3], "rgb", "tableOrder: {stdout}");
    assert_eq!(fields[4], "0", "dither: {stdout}");
    let expected: String = CUBE_IDENTITY.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        fields[5], expected,
        "embedded identity cube bytes: {stdout}"
    );
}

/// The identity cube embedded by `scripts/generate-fixtures.py`; must match
/// `pictura_render::identity_cube()` byte-for-byte.
const CUBE_IDENTITY: &[u8] = b"TITLE \"Identity\"\nLUT_3D_SIZE 2\n\
0.0 0.0 0.0\n1.0 0.0 0.0\n0.0 1.0 0.0\n1.0 1.0 0.0\n\
0.0 0.0 1.0\n1.0 0.0 1.0\n0.0 1.0 1.0\n1.0 1.0 1.0\n";

const TEXT_PATH_SCRIPT: &str = r#"
const fs = require('fs');
const ag = require('ag-psd');
const psd = ag.readPsd(fs.readFileSync(process.argv[1]), {
  skipLayerImageData: true,
  skipCompositeImageData: true,
});
const lines = [];
(function walk(layers) {
  for (const layer of layers || []) {
    const p = layer.text && layer.text.textPath;
    if (p) {
      lines.push([p.data.type, p.bezierCurve.controlPoints.length, p.data.textRange[0]].join(' '));
    }
    walk(layer.children);
  }
})(psd.children);
console.log(lines.join('\n'));
"#;

#[test]
fn ag_psd_reads_authored_text_path() {
    use pictura_core::path::{PathPoint, Subpath};
    use pictura_core::{
        BitDepth, Channel, ColorMode, Document, Layer, LayerBlock, PsdRect, TextPath, TextPathKind,
        TypeSpec,
    };

    if !node_ag_psd_available() {
        eprintln!(
            "skipping ag-psd check: node + ag-psd not available (run `npm i ag-psd`, or set NODE_PATH)"
        );
        return;
    }

    let path = TextPath {
        kind: TextPathKind::OnPath,
        subpath: Subpath {
            points: vec![
                PathPoint {
                    anchor: (10.0, 50.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (150.0, 50.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed: false,
        },
        start: 20.0,
    };
    let mut spec = TypeSpec::new("Path", "Liberation Sans", 20.0);
    spec.path = Some(path);
    let tool = pictura_codec::author_type_tool(&spec, [0.0, -20.0, 60.0, 10.0], None, None);
    let rect = PsdRect {
        top: 20,
        left: 10,
        bottom: 70,
        right: 150,
    };
    let mut doc = Document::new(160, 80, ColorMode::Rgb, BitDepth::Eight);
    doc.layers
        .push(Layer {
            name: "Path".into(),
            rect,
            channels: [0, 1, 2, -1]
                .into_iter()
                .map(|id| Channel {
                    id,
                    data: vec![0; 140 * 50].into(),
                })
                .collect(),
            extra_blocks: vec![LayerBlock {
                key: *b"TySh",
                data: pictura_codec::encode_type_tool(&tool).into(),
            }],
            type_tool: Some(tool),
            ..Default::default()
        })
        .expect("push the type layer");
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let fixture =
        std::env::temp_dir().join(format!("pictura-text-path-{}.psd", std::process::id()));
    std::fs::write(&fixture, bytes).expect("write fixture");
    let out = Command::new("node")
        .args([
            "-e",
            TEXT_PATH_SCRIPT,
            fixture.to_str().expect("utf-8 fixture path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run node");
    let _ = std::fs::remove_file(&fixture);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "ag-psd read failed:\n{stdout}\n{stderr}"
    );
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one text path layer:\n{stdout}");
    let fields: Vec<&str> = lines[0].split_whitespace().collect();
    assert_eq!(fields[0], "2", "on-path type: {stdout}");
    assert_eq!(fields[1], "8", "one cubic segment: {stdout}");
    let start: f64 = fields[2].parse().expect("start fraction");
    assert!(
        (start - 20.0 / 140.0).abs() < 0.001,
        "start fraction: {stdout}"
    );
}
