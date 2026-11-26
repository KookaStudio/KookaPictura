use super::*;

/// The pattern-stroke fixture: the psd-tools-authored `lfx2` block survives
/// read and whole-document round-trip, and psd-tools reads the effect back as a
/// `Stroke` with `fill_type` `Ptrn` and the authored pattern content.
#[test]
fn stroke_pattern_layer_preserves_effect_block() {
    let doc = load("stroke_pattern.psd");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Stroked")
        .expect("Stroked layer");
    let block = layer
        .extra_block(b"lfx2")
        .expect("lfx2 is preserved in extra_blocks");
    assert!(block.data.len() > 8, "DescriptorBlock2 payload");
    assert_eq!(&block.data[0..4], &[0, 0, 0, 1], "version 1");
    assert_eq!(&block.data[4..8], &[0, 0, 0, 16], "data version 16");

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc, "the whole document round-trips");
    assert!(
        back.layers
            .iter()
            .find(|l| l.name == "Stroked")
            .and_then(|l| l.extra_block(b"lfx2"))
            .is_some(),
        "lfx2 survives a round-trip"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("psd-stroke-pattern");
    let path = dir.join("stroke_pattern.psd");
    std::fs::write(
        &path,
        std::fs::read(fixture_dir().join("stroke_pattern.psd")).unwrap(),
    )
    .unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
layer = [l for l in psd if l.name == "Stroked"][0]
effect = layer.effects.items[0]
print(type(effect).__name__)
print(1 if effect.enabled else 0)
print(1 if effect.present else 0)
print(effect.fill_type.decode())
print(1 if effect.pattern is not None else 0)
print(1 if effect.linked else 0)
print(int(round(effect.angle)))
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "psd-tools failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let got: Vec<&str> = stdout.lines().map(str::trim).collect();
    assert_eq!(
        got,
        ["Stroke", "1", "1", "Ptrn", "1", "0", "0",],
        "psd-tools reads the authored pattern FrFX; stdout={stdout}"
    );
}
