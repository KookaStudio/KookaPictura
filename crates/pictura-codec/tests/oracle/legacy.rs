use super::*;

/// The legacy-effects fixture: the psd-tools-authored `lrFX` (`EFFECTS_LAYER`)
/// fixed struct survives read and whole-document round-trip, and psd-tools
/// re-reads it as an `EffectsLayer` with the authored shadow and glow values.
///
/// The block is not a descriptor, so it stays opaque in `extra_blocks`; this
/// test pins that the codec frames it byte-for-byte against a second PSD
/// implementation.
#[test]
fn legacy_effects_layer_preserves_block() {
    let doc = load("legacy_effects.psd");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Legacy")
        .expect("Legacy layer");
    let block = layer
        .extra_block(b"lrFX")
        .expect("lrFX is preserved in extra_blocks");
    assert_eq!(&block.data[0..2], &[0, 0], "EffectsLayer version 0");
    assert_eq!(&block.data[2..4], &[0, 3], "three records");
    assert_eq!(&block.data[4..8], b"8BIM", "record signature");

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc, "the whole document round-trips");
    assert!(
        back.layers
            .iter()
            .find(|l| l.name == "Legacy")
            .and_then(|l| l.extra_block(b"lrFX"))
            .is_some(),
        "lrFX survives a round-trip"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("psd-legacy-effects");
    let path = dir.join("legacy_effects.psd");
    std::fs::write(
        &path,
        std::fs::read(fixture_dir().join("legacy_effects.psd")).unwrap(),
    )
    .unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
from psd_tools.constants import EffectOSType, Tag
psd = PSDImage.open(sys.argv[1], lazy=False)
layer = [l for l in psd if l.name == "Legacy"][0]
data = layer._record.tagged_blocks[Tag.EFFECTS_LAYER].data
cmn = data[EffectOSType.COMMON_STATE]
dsdw = data[EffectOSType.DROP_SHADOW]
oglw = data[EffectOSType.OUTER_GLOW]
print(1 if cmn.visible else 0)
print(1 if dsdw.enabled else 0)
print(dsdw.blur, dsdw.intensity, dsdw.angle, dsdw.distance)
print(dsdw.color.values[0], dsdw.color.values[1], dsdw.color.values[2])
print(dsdw.blend_mode.decode())
print(dsdw.opacity)
print(1 if oglw.enabled else 0)
print(oglw.blur, oglw.intensity)
print(oglw.color.values[0], oglw.color.values[1], oglw.color.values[2])
print(oglw.blend_mode.decode())
print(oglw.opacity)
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
        [
            "1",
            "1",
            "5 0 120 5",
            "2560 5120 7680",
            "mul",
            "255",
            "1",
            "6 0",
            "10240 20480 30720",
            "scrn",
            "191",
        ],
        "psd-tools reads the authored EffectsLayer; stdout={stdout}"
    );
}
