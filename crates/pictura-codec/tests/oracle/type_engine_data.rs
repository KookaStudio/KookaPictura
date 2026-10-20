use super::*;

use pictura_codec::{
    author_type_tool, encode_type_tool, parse_engine_data, write_descriptor, EngineValue,
};
use pictura_core::{AntiAlias, TypeSpec};

/// The committed `engine_data.bin` is the raw EngineData blob of a reference
/// 2021 text layer. Parsing and deriving the style here proves the Rust parser
/// agrees with the psd-tools 1.19 EngineData reader on the same bytes.
#[test]
fn engine_data_fixture_decodes_to_expected_style() {
    let path = fixture_dir().join("engine_data.bin");
    let bytes = std::fs::read(&path).expect("read engine_data.bin");
    let root = pictura_codec::parse_engine_data(&bytes).expect("parses");

    let fonts = pictura_codec::extract_fonts(&root);
    assert_eq!(fonts, ["AdobeInvisFont", "MyriadPro-Regular"]);
    let style = pictura_codec::extract_style(&root, &fonts);
    assert_eq!(style.font.as_deref(), Some("MyriadPro-Regular"));
    assert_eq!(style.character.size, 150.0);
    assert_eq!(style.character.fill_color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(style.character.tracking, 0.0);
    assert_eq!(style.paragraph.justify, pictura_core::Justify::Left);
    assert_eq!(
        get(&root, "EngineDict")
            .and_then(|v| get(v, "Editor"))
            .and_then(|v| get(v, "Text")),
        Some(&EngineValue::String("hello world\r".into()))
    );

    if !psd_tools_available() {
        eprintln!("skipping psd-tools cross-check: python3 + psd-tools not available");
        return;
    }
    assert_eq!(ours(&fonts, &style, &root), psd_tools_values(&path));
}

fn get<'a>(value: &'a EngineValue, key: &str) -> Option<&'a EngineValue> {
    match value {
        EngineValue::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// Same fields the Python script prints, in the same order.
fn ours(fonts: &[String], style: &pictura_core::TextStyle, root: &EngineValue) -> String {
    let text = match get(root, "EngineDict")
        .and_then(|v| get(v, "Editor"))
        .and_then(|v| get(v, "Text"))
    {
        Some(EngineValue::String(s)) => s.as_str(),
        _ => "",
    };
    let text_hex: String = text.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let font = style.font.as_deref().unwrap_or("-");
    let fill = style
        .character
        .fill_color
        .iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "text_hex={text_hex}\nfonts={}\nfont={font}\nsize={:?}\nfill={fill}\ntracking={:?}\njust={}",
        fonts.join(","),
        style.character.size,
        style.character.tracking,
        style.paragraph.justify.index(),
    )
}

fn psd_tools_values(path: &std::path::Path) -> String {
    let script = r#"
import sys
from psd_tools.psd.engine_data import EngineData

d = EngineData.frombytes(open(sys.argv[1], "rb").read())
def g(o, *ks):
    for k in ks:
        o = o[k]
    return o
def s(v):
    return v.value if hasattr(v, "value") else v

text = g(d, "EngineDict", "Editor", "Text")
fonts = [s(e["Name"]) for e in g(d, "ResourceDict", "FontSet")]
run = g(d, "EngineDict", "StyleRun", "RunArray", 0, "StyleSheet", "StyleSheetData")
default = g(d, "ResourceDict", "StyleSheetSet", 0, "StyleSheetData")
idx = int(run.get("Font", default.get("Font", 0)))
font = fonts[idx] if 0 <= idx < len(fonts) else "-"
size = float(run.get("FontSize", default.get("FontSize", 0.0)))
fillc = run.get("FillColor", default.get("FillColor"))
vals = [float(x) for x in fillc["Values"]] if fillc is not None and "Values" in fillc else [0.0, 0.0, 0.0, 1.0]
tracking = float(run.get("Tracking", default.get("Tracking", 0.0)))
just = int(g(d, "EngineDict", "ParagraphRun", "RunArray", 0, "ParagraphSheet", "Properties", "Justification"))

print("text_hex=" + s(text).encode("utf-8").hex())
print("fonts=" + ",".join(fonts))
print("font=" + font)
print("size=" + repr(size))
print("fill=" + ",".join(repr(v) for v in vals))
print("tracking=" + repr(tracking))
print("just=" + str(just))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The `AntA` descriptor value and `EngineDict/AntiAlias` integer decoded from a
/// `TySh` text descriptor, for the grounded-anti-aliasing check.
fn descriptor_ant_a(text_desc: &[u8]) -> (Option<Vec<u8>>, Option<i64>) {
    use pictura_codec::DescValue;
    let DescValue::Object { items, .. } = read_descriptor(text_desc).expect("descriptor") else {
        panic!("text descriptor is an object");
    };
    let spelling = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"AntA")
        .and_then(|(_, v)| match v {
            DescValue::Enum { value, .. } => Some(value.clone()),
            _ => None,
        });
    let raw = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"EngineData")
        .and_then(|(_, v)| match v {
            DescValue::Raw(bytes) => Some(bytes.clone()),
            _ => None,
        })
        .expect("EngineData raw");
    let payload = if raw.len() >= 8 && &raw[..4] == b"tdta" {
        let len = u32::from_be_bytes(raw[4..8].try_into().unwrap()) as usize;
        raw[8..8 + len].to_vec()
    } else {
        raw
    };
    let root = parse_engine_data(&payload).expect("engine data parses");
    let flag = match get(&root, "EngineDict").and_then(|e| get(e, "AntiAlias")) {
        Some(EngineValue::Int(n)) => Some(*n),
        Some(EngineValue::Double(n)) => Some(*n as i64),
        _ => None,
    };
    (spelling, flag)
}

/// 7.1 — the `AntA` spellings (`antiAliasNone|Sharp|Crisp|Strong|Smooth`) and
/// the `/AntiAlias` integer width are inferred from `type_write.rs`, not read
/// from a CS6-authored file. This test validates them against grounded CS6
/// fixtures that a human drops into `tests/fixtures/type-aa/` (one per method:
/// `none.psd`, `sharp.psd`, `crisp.psd`, `strong.psd`, `smooth.psd`). No such
/// fixture ships with the repo and psd-tools cannot author a `TySh` `AntA`, so
/// this test self-skips: the inference remains unvalidated until a real
/// Photoshop file is supplied. When one is, a disagreement fails here and the
/// codec mapping is corrected.
#[test]
fn grounded_cs6_antialias_fixtures_match_the_confirmed_mapping() {
    let dir = fixture_dir().join("type-aa");
    if !dir.is_dir() {
        eprintln!(
            "skipping 7.1 AntA grounding: no CS6-authored fixtures in {}",
            dir.display()
        );
        return;
    }
    let cases = [
        ("none.psd", AntiAlias::None, "antiAliasNone", 0i64),
        ("sharp.psd", AntiAlias::Sharp, "antiAliasSharp", 1),
        ("crisp.psd", AntiAlias::Crisp, "antiAliasCrisp", 2),
        ("strong.psd", AntiAlias::Strong, "antiAliasStrong", 3),
        ("smooth.psd", AntiAlias::Smooth, "antiAliasSmooth", 4),
    ];
    let mut checked = 0;
    for (file, method, spelling, flag) in cases {
        let path = dir.join(file);
        if !path.exists() {
            continue;
        }
        let doc = read_psd(&std::fs::read(&path).unwrap()).expect("read fixture");
        let tool = doc
            .layers
            .iter()
            .find_map(|layer| layer.type_tool.as_ref())
            .unwrap_or_else(|| panic!("{file}: no type layer"));
        let (ant_a, raw_flag) = descriptor_ant_a(&tool.text_desc);
        assert_eq!(
            ant_a.as_deref(),
            Some(spelling.as_bytes()),
            "{file}: descriptor `AntA` spelling"
        );
        assert_eq!(raw_flag, Some(flag), "{file}: `/AntiAlias` integer width");
        assert_eq!(
            tool.style.as_ref().map(|s| s.character.anti_alias),
            Some(method),
            "{file}: decoded anti-aliasing method"
        );
        checked += 1;
    }
    assert!(checked > 0, "fixture dir present but held no expected file");
}

/// 7.2 — author a type layer carrying modelled attributes and one unmodeled
/// EngineData key, write the document with the codec, and read it back through
/// psd-tools: it must accept the file, classify the layer as type, and report
/// the modelled values and the preserved unmodeled key. Self-skips when
/// psd-tools is absent.
#[test]
fn authored_type_layer_psd_round_trips_through_psd_tools() {
    if !psd_tools_available() {
        eprintln!("skipping 7.2 psd-tools round-trip: python3 + psd-tools not available");
        return;
    }

    // Existing EngineData with a modelled value to overwrite (`/Tracking 5`) and
    // an unmodeled key (`/PicturaMarker 7`) that must survive the re-set.
    let engine = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00P\x00i\x00n\x00\r) >> \
        /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << \
        /Font 0 /FontSize 20.0 /Tracking 5 /PicturaMarker 7 \
        /FillColor << /Values [ 1.0 1.0 1.0 1.0 ] >> >> >> >> ] >> \
        /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
        >> /ResourceDict << /FontSet [ << /Name (LiberationSans) >> ] \
        /StyleSheetSet [ << /StyleSheetData << /Font 0 /Tracking 9 >> >> ] >> >>";
    let mut raw = b"tdta".to_vec();
    raw.extend_from_slice(&(engine.len() as u32).to_be_bytes());
    raw.extend_from_slice(engine);
    let existing = write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"TxLr".to_vec(),
        items: vec![
            (b"Txt ".to_vec(), DescValue::Text("Pin\0".into())),
            (
                b"AntA".to_vec(),
                DescValue::Enum {
                    kind: b"Annt".to_vec(),
                    value: b"antiAliasCrisp".to_vec(),
                },
            ),
            (b"EngineData".to_vec(), DescValue::Raw(raw)),
        ],
    });

    let mut spec = TypeSpec::new("Pin", "Liberation Sans", 24.0);
    spec.character.tracking = 120.0;
    spec.character.horizontal_scale = 85.0;
    spec.character.anti_alias = AntiAlias::Crisp;
    let tool = author_type_tool(&spec, [0.0, 0.0, 40.0, 30.0], Some(&existing), None);
    let tysh = encode_type_tool(&tool);

    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "Pin".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![0u8; 64].into(),
            },
            Channel {
                id: 1,
                data: vec![0u8; 64].into(),
            },
            Channel {
                id: 2,
                data: vec![0u8; 64].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; 64].into(),
            },
        ],
        extra_blocks: vec![LayerBlock {
            key: *b"TySh",
            data: tysh,
        }],
        ..Default::default()
    }]);

    let bytes = write_psd(&doc).expect("write_psd");
    let out = psd_tools_type_values(&bytes);
    assert_eq!(
        out, "kind=type\ntracking=120\nhscale=0.85\nsize=24.0\nmarker=7",
        "psd-tools values must match the authored type layer"
    );
}

/// Write `bytes` to a scratch file and have psd-tools report the authored type
/// layer's kind, modelled values, and preserved unmodeled key.
fn psd_tools_type_values(bytes: &[u8]) -> String {
    let dir = scratch_dir("type-layer-roundtrip");
    let path = dir.join("doc.psd");
    std::fs::write(&path, bytes).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage

image = PSDImage.open(sys.argv[1])
layer = image[0]
run = layer.engine_dict["StyleRun"]["RunArray"][0]["StyleSheet"]["StyleSheetData"]
print("kind=" + layer.kind)
print("tracking=" + str(run["Tracking"].value))
print("hscale=" + str(float(run["HorizontalScale"].value)))
print("size=" + str(float(run["FontSize"].value)))
print("marker=" + str(run["PicturaMarker"].value))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "psd-tools rejected the authored PSD:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
