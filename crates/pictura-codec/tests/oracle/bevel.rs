use super::*;

/// The bevel & emboss fixture: the psd-tools-authored `lfx2` object-based
/// effects block survives read and whole-document round-trip, and psd-tools
/// reads the effect back as a `BevelEmboss` with the authored values, including
/// the `hglM`/`hglC`/`hglO` and `sdwM`/`sdwC`/`sdwO` key pairs and the
/// `InrB`/`SfBL`/`In  ` style/technique/direction enums.
#[test]
fn bevel_layer_preserves_effect_block() {
    let doc = load("bevel.psd");
    let layer = doc
        .layers
        .iter()
        .find(|l| l.name == "Beveled")
        .expect("Beveled layer");
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
            .find(|l| l.name == "Beveled")
            .and_then(|l| l.extra_block(b"lfx2"))
            .is_some(),
        "lfx2 survives a round-trip"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("psd-bevel");
    let path = dir.join("bevel.psd");
    std::fs::write(
        &path,
        std::fs::read(fixture_dir().join("bevel.psd")).unwrap(),
    )
    .unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
layer = [l for l in psd if l.name == "Beveled"][0]
effect = layer.effects.items[0]
print(type(effect).__name__)
print(1 if effect.enabled else 0)
print(1 if effect.present else 0)
print(effect.highlight_mode.decode())
print(effect.highlight_opacity)
print(int(round(float(effect.highlight_color[b"Rd  "]))), int(round(float(effect.highlight_color[b"Grn "]))), int(round(float(effect.highlight_color[b"Bl  "]))))
print(effect.shadow_mode.decode())
print(effect.shadow_opacity)
print(int(round(float(effect.shadow_color[b"Rd  "]))), int(round(float(effect.shadow_color[b"Grn "]))), int(round(float(effect.shadow_color[b"Bl  "]))))
print(effect.bevel_style.decode())
print(effect.bevel_type.decode())
print(effect.direction.decode())
print(effect.altitude)
print(effect.depth)
print(effect.size)
print(effect.soften)
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
            "BevelEmboss",
            "1",
            "1",
            "Scrn",
            "80.0",
            "250 240 230",
            "Mltp",
            "70.0",
            "10 20 30",
            "InrB",
            "SfBL",
            "In",
            "30.0",
            "250.0",
            "7.0",
            "3.0",
        ],
        "psd-tools reads the authored ebbl effect; stdout={stdout}"
    );
}
