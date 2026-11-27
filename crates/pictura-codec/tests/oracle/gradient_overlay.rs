use super::*;

/// The gradient-overlay fixture: the psd-tools-authored `lfx2` object-based
/// effects block survives read and whole-document round-trip, and psd-tools
/// reads the effect back as a `GradientOverlay` with the authored `GrFl`
/// values (kind, angle, reverse, scale, alignment).
#[test]
fn gradient_overlay_layer_preserves_effect_block() {
    let doc = load("gradient_overlay.psd");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Gradient")
        .expect("Gradient layer");
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
            .find(|l| l.name == "Gradient")
            .and_then(|l| l.extra_block(b"lfx2"))
            .is_some(),
        "lfx2 survives a round-trip"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("psd-gradient-overlay");
    let path = dir.join("gradient_overlay.psd");
    std::fs::write(
        &path,
        std::fs::read(fixture_dir().join("gradient_overlay.psd")).unwrap(),
    )
    .unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
layer = [l for l in psd if l.name == "Gradient"][0]
effect = layer.effects.items[0]
print(type(effect).__name__)
print(1 if effect.enabled else 0)
print(1 if effect.present else 0)
print(effect.opacity)
print(effect.blend_mode.decode())
print(effect.type.decode())
print(effect.angle)
print(1 if effect.reversed else 0)
print(effect.scale)
print(1 if effect.aligned else 0)
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
    let got = oracle_tokens(&stdout);
    assert_eq!(
        got,
        [
            "GradientOverlay",
            "1",
            "1",
            "80.0",
            "Mltp",
            "Lnr",
            "45.0",
            "1",
            "150.0",
            "0",
        ],
        "psd-tools reads the authored GrFl effect; stdout={stdout}"
    );
}
