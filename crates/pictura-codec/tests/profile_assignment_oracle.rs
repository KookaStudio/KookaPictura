//! Profile-assignment oracle: an sRGB→Adobe RGB Convert matches an independent
//! PIL/lcms2 conversion within one LSB per channel, an Assign leaves every pixel
//! byte unchanged, and the assigned profile survives `write_psd` as resource
//! 1039 that PIL can open and name.

use std::path::PathBuf;
use std::process::Command;

use pictura_codec::{
    assign_document_profile, convert_document, decode_image_resources, write_psd, Profile,
    ICC_PROFILE,
};
use pictura_core::{Document, PixelBuffer};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pictura_profile_{name}_{}.tmp", std::process::id()))
}

fn pil_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args(["-c", "import PIL.ImageCms"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// The first three planes of a planar 3- or 4-channel buffer, per pixel.
fn rgb_pixels(buf: &PixelBuffer) -> Vec<[u8; 3]> {
    let n = buf.pixel_count();
    (0..n)
        .map(|i| [buf.data[i], buf.data[n + i], buf.data[2 * n + i]])
        .collect()
}

fn sample_rgba() -> [u8; 12] {
    [200, 100, 50, 255, 10, 240, 130, 255, 0, 0, 0, 255]
}

#[test]
fn convert_srgb_to_adobe_matches_lcms2() {
    if !pil_available() {
        eprintln!("skipping lcms2 comparison: PIL not available");
        return;
    }
    let rgba = sample_rgba();
    let mut doc = Document::from_rgba("px", 3, 1, &rgba);
    let source: Vec<[u8; 3]> = rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect();

    let profile = temp_path("adobe.icc");
    std::fs::write(&profile, Profile::adobe_rgb().to_icc()).expect("write profile");
    assert!(
        convert_document(&mut doc, &Profile::adobe_rgb()),
        "conversion runs"
    );
    let got = rgb_pixels(doc.composite.buffer());

    let script = r#"
import sys
from PIL import Image, ImageCms
pixels = [tuple(int(v) for v in row.split(",")) for row in sys.argv[2].split(";")]
img = Image.new("RGB", (len(pixels), 1))
img.putdata(pixels)
src = ImageCms.createProfile("sRGB")
dst = ImageCms.getOpenProfile(sys.argv[1])
out = ImageCms.profileToProfile(img, src, dst, renderingIntent=1, outputMode="RGB")
print(";".join(",".join(str(c) for c in px) for px in out.getdata()))
"#;
    let arg = source
        .iter()
        .map(|p| format!("{},{},{}", p[0], p[1], p[2]))
        .collect::<Vec<_>>()
        .join(";");
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&profile)
        .arg(&arg)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&profile);
    assert!(
        out.status.success(),
        "PIL conversion failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let expected: Vec<[u8; 3]> = String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(';')
        .map(|row| {
            let values: Vec<u8> = row.split(',').map(|t| t.parse().unwrap()).collect();
            [values[0], values[1], values[2]]
        })
        .collect();

    assert_eq!(got.len(), expected.len());
    for (i, (got, want)) in got.iter().zip(&expected).enumerate() {
        for c in 0..3 {
            assert!(
                (got[c] as i32 - want[c] as i32).abs() <= 1,
                "pixel {i} channel {c}: converted {got:?} differs from lcms2 {want:?}"
            );
        }
    }
}

#[test]
fn assign_leaves_composite_and_layers_byte_identical() {
    let rgba = sample_rgba();
    let mut doc = Document::from_rgba("px", 3, 1, &rgba);
    let before = doc.clone();
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert_eq!(doc.composite, before.composite, "composite bytes change");
    assert_eq!(
        doc.layers[0].channels(),
        before.layers[0].channels(),
        "layer channel bytes change"
    );
}

#[test]
fn assigned_profile_round_trips_through_saved_psd() {
    if !pil_available() {
        eprintln!("skipping PIL profile check: PIL not available");
        return;
    }
    let mut doc = Document::from_rgba("px", 3, 1, &sample_rgba());
    let assigned = Profile::adobe_rgb().to_icc();
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    let bytes = write_psd(&doc).expect("write assigned document");

    let psd = temp_path("assigned.psd");
    let icc = temp_path("assigned.icc");
    std::fs::write(&psd, &bytes).expect("write psd");
    let script = r#"
import sys
from PIL import Image, ImageCms
im = Image.open(sys.argv[1])
data = im.info.get("icc_profile")
if not data:
    print("NOICC", file=sys.stderr)
    sys.exit(1)
open(sys.argv[2], "wb").write(data)
print(ImageCms.getProfileDescription(ImageCms.getOpenProfile(sys.argv[2])))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&psd)
        .arg(&icc)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&psd);
    assert!(
        out.status.success(),
        "PIL could not open the saved profile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let saved = std::fs::read(&icc).expect("PIL extracted the profile");
    let _ = std::fs::remove_file(&icc);
    assert_eq!(
        saved, assigned,
        "the saved 1039 data equals the assigned profile's bytes"
    );
    assert_ne!(
        saved,
        Profile::srgb().to_icc(),
        "the harness rejects a mismatched expected profile"
    );
    let description = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let expected =
        pictura_codec::profile_description(&assigned).expect("assigned profile has a description");
    assert_eq!(description, expected, "PIL names the assigned profile");
}

#[test]
fn convert_back_to_srgb_leaves_the_document_untagged() {
    let mut doc = Document::from_rgba("px", 3, 1, &sample_rgba());
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert!(convert_document(&mut doc, &Profile::srgb()));
    assert!(
        doc.source.working_profile().is_none(),
        "the working profile is sRGB"
    );
    assert!(
        decode_image_resources(&doc)
            .iter()
            .all(|r| r.id != ICC_PROFILE),
        "no 1039 survives a convert back to the sRGB working space"
    );
}
