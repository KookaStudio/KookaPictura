use super::*;

/// Opaque sections survive an open→save round-trip authoritatively: the exact
/// image-resource, color-mode-data, global-mask and trailing layer-section bytes
/// are re-emitted, and the result is still a valid PSD that psd-tools opens and
/// reads the original layer tree from.
#[test]
fn opaque_resources_survive_write_and_psd_tools_still_opens() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }

    let doc = load("two_layers.psd");
    assert!(
        !doc.image_resources.is_empty(),
        "two_layers.psd carries a non-empty image-resource section"
    );

    let out = write_psd(&doc).unwrap();
    let back = read_psd(&out).unwrap();
    assert_eq!(back.image_resources, doc.image_resources, "image resources");
    assert_eq!(back.color_mode_data, doc.color_mode_data, "color-mode data");
    assert_eq!(back.global_layer_mask, doc.global_layer_mask, "global mask");
    assert_eq!(
        back.layer_section_extra, doc.layer_section_extra,
        "layer-section trailing bytes"
    );

    let dir = scratch_dir("opaque-resources");
    let path = dir.join("opaque.psd");
    std::fs::write(&path, &out).unwrap();

    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
print(len(list(psd)))
for layer in psd:
    print(layer.name)
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
        "psd-tools failed on the re-emitted file:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut lines = stdout.lines();
    let count: usize = lines
        .next()
        .expect("layer count line")
        .trim()
        .parse()
        .unwrap();
    let names: Vec<&str> = lines.collect();
    assert_eq!(count, 2, "psd-tools sees two layers; stdout={stdout}");
    assert_eq!(names, ["Red", "Blue"], "psd-tools layer names");
}
