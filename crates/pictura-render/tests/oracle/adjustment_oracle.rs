//! psd-tools structural oracle for `pictura_render` adjustment encoders.
//!
//! Unlike the fixture-based adjustment oracles, these do not touch a golden
//! file: they encode a block in Rust and let psd-tools' independent reader
//! decode the same bytes, asserting the fields match. The `mixr` check reads
//! only the red row because psd-tools' `ChannelMixer` reads a `2H` + `5h`
//! prefix and dumps the green/blue/gray channels into its opaque `unknown`
//! blob; ag-psd (`crates/pictura-codec/tests/psd/agpsd_oracle.rs`) is the
//! full-field oracle. Self-skips when `python3`/`psd_tools` is absent.

use pictura_adjust::{
    Adjustment, CurvesParams, SelectiveColorMethod, SelectiveColorParams, SelectiveRange,
};
use pictura_render::{decode_adjustment, encode_color_balance};
use pictura_testkit::oracle;

/// Print `shadows midtones highlights luminosity` as whitespace-separated
/// integers, reading the hex-encoded `blnc` payload from `argv[1]`.
const SCRIPT: &str = "\
import sys
from io import BytesIO
from psd_tools.psd.adjustments import ColorBalance
cb = ColorBalance.read(BytesIO(bytes.fromhex(sys.argv[1])))
print(*cb.shadows, *cb.midtones, *cb.highlights, int(cb.luminosity))
";

#[test]
fn psd_tools_reads_encoded_color_balance() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let encoded = encode_color_balance(
        [-100.0, 0.0, 40.0],
        [25.0, 0.0, -30.0],
        [100.0, 0.0, 0.0],
        true,
    );
    assert_eq!(encoded.key, *b"blnc");
    assert_eq!(encoded.data.len(), 20);

    let hex: String = encoded.data.iter().map(|b| format!("{b:02x}")).collect();
    let out = oracle::Command::new("python3")
        .args(["-c", SCRIPT, &hex])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools ColorBalance read failed:\n{stdout}\n{stderr}"
    );

    let fields: Vec<i64> = stdout
        .split_whitespace()
        .map(|token| token.parse::<i64>().expect("integer field"))
        .collect();
    assert_eq!(
        fields,
        vec![-100, 0, 40, 25, 0, -30, 100, 0, 0, 1],
        "psd-tools read {fields:?}: {stdout}"
    );
}

/// Print `version monochrome` then the five shorts psd-tools reads from the
/// hex-encoded `mixr` payload in `argv[1]`.
const MIXR_SCRIPT: &str = "\
import sys
from io import BytesIO
from psd_tools.psd.adjustments import ChannelMixer
cm = ChannelMixer.read(BytesIO(bytes.fromhex(sys.argv[1])))
print(cm.version, cm.monochrome, *cm.data)
";

#[test]
fn psd_tools_reads_fixture_channel_mixer_prefix() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/channel_mixer.psd"
    );
    let bytes = std::fs::read(path).expect("read channel_mixer.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Channel Mixer")
        .expect("Channel Mixer layer");
    let block = layer.adjustment().expect("mixr block");
    assert_eq!(block.key, *b"mixr");
    assert_eq!(block.data.len(), 44);

    let hex: String = block.data.iter().map(|b| format!("{b:02x}")).collect();
    let out = oracle::Command::new("python3")
        .args(["-c", MIXR_SCRIPT, &hex])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools ChannelMixer read failed:\n{stdout}\n{stderr}"
    );

    // psd-tools sees only the red row plus the reserved pair and its constant:
    // version, monochrome, red.red, red.green, red.blue, reserved, red.constant.
    let fields: Vec<i64> = stdout
        .split_whitespace()
        .map(|token| token.parse::<i64>().expect("integer field"))
        .collect();
    assert_eq!(
        fields,
        vec![1, 0, 30, -10, 50, 0, 5],
        "psd-tools read {fields:?}: {stdout}"
    );
}

/// `version count_map` then the first curve's `(output, input)` pairs, read from
/// the hex-encoded `curv` payload in `argv[1]`.
const CURVES_SCRIPT: &str = "\
import sys
from io import BytesIO
from psd_tools.psd.adjustments import Curves
c = Curves.read(BytesIO(bytes.fromhex(sys.argv[1])))
print(c.version, c.count_map, *[v for p in c.data[0] for v in p])
";

/// The committed `curves.psd` decodes through `decode_adjustment` into the
/// authored composite-only and per-channel curves. This is a pure-Rust fixture
/// check; psd-tools and ag-psd carry the independent proofs.
#[test]
fn fixture_curves_decodes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/curves.psd"
    );
    let bytes = std::fs::read(path).expect("read curves.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");

    let composite = doc
        .layers
        .iter()
        .find(|l| l.name == "Curves")
        .expect("Curves layer");
    assert_eq!(
        decode_adjustment(composite.adjustment().expect("curv block")),
        Some(Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (64, 32), (192, 224), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }))
    );

    let channels = doc
        .layers
        .iter()
        .find(|l| l.name == "Curves Channels")
        .expect("Curves Channels layer");
    assert_eq!(
        decode_adjustment(channels.adjustment().expect("curv block")),
        Some(Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (255, 255)],
            red: Some(vec![(0, 0), (128, 255), (255, 255)]),
            green: Some(vec![(0, 0), (64, 16), (255, 255)]),
            blue: Some(vec![(0, 255), (255, 0)]),
        }))
    );
}

/// psd-tools reads the version-1 prefix (version, the low-16-bit bitmask) and
/// the first curve's points, but does not name the per-channel rows, so this is
/// a partial check; the ag-psd oracle in `pictura-codec` is the full-field one.
#[test]
fn psd_tools_reads_fixture_curves_prefix() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/curves.psd"
    );
    let bytes = std::fs::read(path).expect("read curves.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Curves")
        .expect("Curves layer");
    let block = layer.adjustment().expect("curv block");
    assert_eq!(block.key, *b"curv");

    let hex: String = block.data.iter().map(|b| format!("{b:02x}")).collect();
    let out = oracle::Command::new("python3")
        .args(["-c", CURVES_SCRIPT, &hex])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools Curves read failed:\n{stdout}\n{stderr}"
    );

    let fields: Vec<i64> = stdout
        .split_whitespace()
        .map(|token| token.parse::<i64>().expect("integer field"))
        .collect();
    assert_eq!(fields[0], 1, "version must be 1");
    assert_eq!(fields[1], 1, "count_map low bits are the rgb bitmask");
    assert_eq!(
        &fields[2..],
        &[0, 0, 32, 64, 224, 192, 255, 255],
        "first curve is the authored rgb curve in (output, input) order"
    );
}

/// The nine relative `selc` plates authored in `selective_color.psd` (reds
/// through blacks; the reserved plate is omitted).
const RELATIVE_PLATES: [[i16; 4]; 9] = [
    [10, -20, 30, 0],
    [0, 0, 0, 5],
    [-10, 0, 0, 0],
    [0, 15, 0, 0],
    [0, 0, -25, 0],
    [5, 0, 0, 0],
    [0, 0, 0, 0],
    [20, -10, 0, 0],
    [0, 0, 0, -40],
];

/// The absolute plates: reds `(1, 2, 3, 4)` through blacks `(33, 34, 35, 36)`.
const ABSOLUTE_PLATES: [[i16; 4]; 9] = [
    [1, 2, 3, 4],
    [5, 6, 7, 8],
    [9, 10, 11, 12],
    [13, 14, 15, 16],
    [17, 18, 19, 20],
    [21, 22, 23, 24],
    [25, 26, 27, 28],
    [29, 30, 31, 32],
    [33, 34, 35, 36],
];

fn selective_params(method: SelectiveColorMethod, plates: [[i16; 4]; 9]) -> SelectiveColorParams {
    SelectiveColorParams {
        method,
        ranges: plates.map(|[c, m, y, k]| SelectiveRange { c, m, y, k }),
    }
}

/// The committed `selective_color.psd` decodes through `decode_adjustment` into
/// the authored relative and absolute params. Pure Rust; psd-tools and ag-psd
/// carry the independent proofs.
#[test]
fn fixture_selective_color_decodes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/selective_color.psd"
    );
    let bytes = std::fs::read(path).expect("read selective_color.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");

    let relative = doc
        .layers
        .iter()
        .find(|l| l.name == "Selective Color")
        .expect("Selective Color layer");
    assert_eq!(
        decode_adjustment(relative.adjustment().expect("selc block")),
        Some(Adjustment::SelectiveColor(selective_params(
            SelectiveColorMethod::Relative,
            RELATIVE_PLATES
        )))
    );

    let absolute = doc
        .layers
        .iter()
        .find(|l| l.name == "Selective Color Abs")
        .expect("Selective Color Abs layer");
    assert_eq!(
        decode_adjustment(absolute.adjustment().expect("selc block")),
        Some(Adjustment::SelectiveColor(selective_params(
            SelectiveColorMethod::Absolute,
            ABSOLUTE_PLATES
        )))
    );
}

/// `version method` then the ten plates flattened, read from the hex-encoded
/// `selc` payload in `argv[1]`.
const SELECTIVE_COLOR_SCRIPT: &str = "\
import sys
from io import BytesIO
from psd_tools.psd.adjustments import SelectiveColor
sc = SelectiveColor.read(BytesIO(bytes.fromhex(sys.argv[1])))
print(sc.version, sc.method, *[v for plate in sc.data for v in plate])
";

/// psd-tools reads the framing (version, method) and the ten plates, but names
/// no plate, so this is a field check; the ag-psd oracle in `pictura-codec` is
/// the full-field, labelled one.
#[test]
fn psd_tools_reads_fixture_selective_color_prefix() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/selective_color.psd"
    );
    let bytes = std::fs::read(path).expect("read selective_color.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Selective Color")
        .expect("Selective Color layer");
    let block = layer.adjustment().expect("selc block");
    assert_eq!(block.key, *b"selc");

    let hex: String = block.data.iter().map(|b| format!("{b:02x}")).collect();
    let out = oracle::Command::new("python3")
        .args(["-c", SELECTIVE_COLOR_SCRIPT, &hex])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools SelectiveColor read failed:\n{stdout}\n{stderr}"
    );

    let fields: Vec<i64> = stdout
        .split_whitespace()
        .map(|token| token.parse::<i64>().expect("integer field"))
        .collect();
    assert_eq!(fields[0], 1, "version must be 1");
    assert_eq!(fields[1], 0, "method must be relative");
    let expected: Vec<i64> = RELATIVE_PLATES
        .iter()
        .flatten()
        .map(|&value| value as i64)
        .collect();
    assert_eq!(
        &fields[6..],
        &expected[..],
        "the nine relative plates (reds..blacks) after the reserved plate"
    );
}

/// The committed `color_lookup.psd` decodes through `decode_adjustment` into a
/// 3-D LUT whose embedded identity cube parses. Pure Rust; psd-tools and ag-psd
/// carry the independent block proofs.
#[test]
fn fixture_color_lookup_decodes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/color_lookup.psd"
    );
    let bytes = std::fs::read(path).expect("read color_lookup.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Color Lookup")
        .expect("Color Lookup layer");
    let block = layer.adjustment().expect("clrL block");
    assert_eq!(block.key, *b"clrL");
    let Some(Adjustment::ColorLookup(params)) = decode_adjustment(block) else {
        panic!("clrL must decode to ColorLookup");
    };
    assert_eq!(params.kind, pictura_adjust::ColorLookupKind::ThreeDLut);
    let lut = params.lookup.expect("identity cube parses");
    assert_eq!(lut.size, 2);
    assert_eq!(lut.points.len(), 8);
    assert_eq!(lut.points[0], [0.0, 0.0, 0.0]);
    assert_eq!(lut.points[7], [1.0, 1.0, 1.0]);
}

/// `data_version lookupType LUTFormat dither lut_bytes_len` read from the
/// hex-encoded `clrL` payload in `argv[1]`.
const COLOR_LOOKUP_SCRIPT: &str = "\
import sys
from io import BytesIO
from psd_tools.psd.adjustments import ColorLookup
cl = ColorLookup.read(BytesIO(bytes.fromhex(sys.argv[1])))
print(cl.data_version, cl[b'lookupType'].enum.decode(), cl[b'LUTFormat'].enum.decode(),
      int(cl[b'Dthr'].value), len(cl[b'LUT3DFileData'].value))
";

/// psd-tools independently reads the framing and the lookup fields, proving the
/// block our decoder accepts is a real PSD-shaped `clrL`.
#[test]
fn psd_tools_reads_fixture_color_lookup_prefix() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pictura-codec/tests/fixtures/color_lookup.psd"
    );
    let bytes = std::fs::read(path).expect("read color_lookup.psd");
    let doc = pictura_codec::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Color Lookup")
        .expect("Color Lookup layer");
    let block = layer.adjustment().expect("clrL block");
    let hex: String = block.data.iter().map(|b| format!("{b:02x}")).collect();

    let out = oracle::Command::new("python3")
        .args(["-c", COLOR_LOOKUP_SCRIPT, &hex])
        .output()
        .expect("run python3");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "psd-tools ColorLookup read failed:\n{stdout}\n{stderr}"
    );
    let fields: Vec<String> = stdout.split_whitespace().map(str::to_string).collect();
    assert_eq!(fields[0], "16", "data version");
    assert_eq!(fields[1], "3DLUT", "lookupType");
    assert_eq!(fields[2], "LUTFormatCUBE", "LUTFormat");
    assert_eq!(fields[3], "0", "dither off");
    assert_eq!(
        fields[4].parse::<usize>().expect("cube length"),
        pictura_render::identity_cube().len(),
        "embedded identity cube length"
    );
}
