//! Mask density and feather (the layer-mask record's mask-parameters block):
//! a round trip through `pictura-codec`, and an independent `psd-tools` read of
//! what it writes.
//!
//! The psd-tools check needs `python3` with `psd-tools`; it self-skips with a
//! message otherwise.

use std::process::Command;

use pictura_codec::{read_psd, write_psd};
use pictura_core::{Document, LayerMask, PsdRect, RawChannel};

const SCRIPT: &str = r#"
import sys
from psd_tools.psd import PSD

with open(sys.argv[1], "rb") as f:
    psd = PSD.read(f)
for rec in psd.layer_and_mask_information.layer_info.layer_records:
    md = rec.mask_data
    if md is None:
        print(f"{rec.name}|none")
        continue
    p = md.parameters
    fields = [rec.name, str(md.flags.parameters_applied), str(md.real_flags is not None)]
    if p is None:
        fields += ["-"] * 4
    else:
        fields += [str(v) for v in (p.user_mask_density, p.user_mask_feather,
                                    p.vector_mask_density, p.vector_mask_feather)]
    print("|".join(fields))
"#;

fn rect(right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom,
        right,
    }
}

/// A background, a masked layer with density 128 and feather 12.5, and a
/// rectangle shape layer whose vector mask has feather 3 and no raster mask.
fn refined_document() -> Document {
    let mut doc = Document::from_rgba("Background", 8, 8, &[255; 8 * 8 * 4]);
    pictura_render::add_layer(&mut doc, 0, "Masked");
    let masked = doc.layers.last_mut().expect("added");
    let mut mask = LayerMask {
        rect: rect(8, 8),
        default_color: 0,
        data: Some(vec![255u8; 64].into()),
        density: 128,
        ..Default::default()
    };
    mask.set_feather(12.5);
    masked.mask = Some(mask);

    let options =
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3);
    let outline = pictura_core::shape::outline(options, (1.0, 2.0), (5.0, 7.0), false, false)
        .expect("a rectangle");
    pictura_render::add_shape_layer(&mut doc, "", [255, 0, 0, 255], "Shape", &outline, None);
    let shape = doc.layers.last_mut().expect("shape");
    shape
        .vector_mask
        .as_mut()
        .expect("a shape has a vector mask")
        .set_feather(3.0);
    doc
}

#[test]
fn density_and_feather_round_trip() {
    let doc = refined_document();
    let back = read_psd(&write_psd(&doc).expect("writes")).expect("reads");
    let masked = back.layers.iter().find(|l| l.name == "Masked").unwrap();
    let mask = masked.mask.as_ref().expect("raster mask");
    assert_eq!(mask.density, 128);
    assert_eq!(mask.feather(), 12.5);
    let shape = back.layers.iter().find(|l| l.name == "Shape 1").unwrap();
    assert!(
        shape.mask.is_none(),
        "a parameter-only record is not a raster mask"
    );
    let vector = shape.vector_mask.as_ref().expect("vector mask");
    assert_eq!(vector.feather(), 3.0);
    assert_eq!(vector.density, 255);
}

#[test]
fn defaults_leave_flag_bit_4_clear_and_reset_values_round_trip() {
    let mut doc = refined_document();
    let masked = doc.layers.iter_mut().find(|l| l.name == "Masked").unwrap();
    let mask = masked.mask.as_mut().unwrap();
    mask.density = 255;
    mask.set_feather(0.0);
    let shape = doc.layers.iter_mut().find(|l| l.name == "Shape 1").unwrap();
    shape.vector_mask.as_mut().unwrap().set_feather(0.0);
    let back = read_psd(&write_psd(&doc).expect("writes")).expect("reads");
    let masked = back.layers.iter().find(|l| l.name == "Masked").unwrap();
    let mask = masked.mask.as_ref().unwrap();
    assert_eq!(mask.flags & 0x10, 0, "no parameters block");
    assert_eq!((mask.density, mask.feather()), (255, 0.0));
    let shape = back.layers.iter().find(|l| l.name == "Shape 1").unwrap();
    assert!(shape.mask.is_none(), "no record is written for defaults");
}

/// A layer with a `-3` channel whose record carries the real-user-mask header
/// and density 64.
fn real_mask_document() -> (Document, Vec<u8>) {
    let mut doc = Document::from_rgba("Background", 2, 2, &[255; 2 * 2 * 4]);
    pictura_render::add_layer(&mut doc, 0, "Real");
    let layer = doc.layers.last_mut().unwrap();
    layer.rect = rect(2, 2);
    let real: Vec<u8> = [0u8, 255]
        .into_iter()
        .chain([0i32, 0, 2, 2].iter().flat_map(|v| v.to_be_bytes()))
        .collect();
    layer.raw_channels.push(RawChannel {
        id: -3,
        data: vec![0, 0, 9, 9, 9, 9],
    });
    layer.mask = Some(LayerMask {
        rect: rect(2, 2),
        data: Some(vec![200u8; 4].into()),
        extra: real.clone(),
        density: 64,
        ..Default::default()
    });
    (doc, real)
}

/// The real-user-mask header sits before the parameters and survives a
/// re-save.
#[test]
fn the_real_mask_header_survives_beside_parameters() {
    let (doc, real) = real_mask_document();
    let back = read_psd(&write_psd(&doc).expect("writes")).expect("reads");
    let mask = back.layers.last().unwrap().mask.as_ref().unwrap();
    assert_eq!(mask.extra, real);
    assert_eq!(mask.density, 64);
}

/// One `|`-separated row per layer record, as `SCRIPT` prints them; `None`
/// when psd-tools is unavailable.
fn psd_tools_rows(doc: &Document, tag: &str) -> Option<String> {
    let available = Command::new("python3")
        .args(["-c", "import psd_tools"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !available {
        eprintln!("skipping: python3 + psd-tools not available");
        return None;
    }
    let path = std::env::temp_dir().join(format!("pictura-{tag}-{}.psd", std::process::id()));
    std::fs::write(&path, write_psd(doc).expect("writes")).expect("write");
    let out = Command::new("python3")
        .args(["-c", SCRIPT, path.to_str().expect("utf-8 path")])
        .output()
        .expect("run python3");
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "psd-tools failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(stdout)
}

#[test]
fn psd_tools_reads_the_written_parameters() {
    let Some(stdout) = psd_tools_rows(&refined_document(), "mask-params") else {
        return;
    };
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.contains(&"Masked|True|False|128|12.5|None|None"),
        "{stdout}"
    );
    assert!(
        lines.contains(&"Shape 1|True|False|None|None|None|3.0"),
        "{stdout}"
    );
}

#[test]
fn psd_tools_reads_parameters_after_the_real_mask_header() {
    let Some(stdout) = psd_tools_rows(&real_mask_document().0, "real-mask") else {
        return;
    };
    assert!(
        stdout
            .lines()
            .any(|l| l == "Real|True|True|64|None|None|None"),
        "{stdout}"
    );
}
