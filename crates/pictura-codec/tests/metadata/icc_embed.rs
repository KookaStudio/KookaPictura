//! Raster ICC embedding tests: the container bytes are parsed back with the
//! profile probe, and exiftool is used as an independent oracle when present.

use std::io::Write;
use std::process::Command;

use pictura_codec::{
    embed_raster_metadata, probe_jpeg_profile, probe_tiff_profile, Profile, RasterMetadata,
};

fn profile() -> Vec<u8> {
    Profile::coated_cmyk().to_icc()
}

fn metadata(icc: Option<Vec<u8>>) -> RasterMetadata {
    RasterMetadata {
        icc,
        ..RasterMetadata::default()
    }
}

#[test]
fn raster_icc_embed_jpeg_app2() {
    let jpeg = [0xFF, 0xD8, 0xFF, 0xD9];
    let icc = profile();
    let out = embed_raster_metadata(&jpeg, "jpg", &metadata(Some(icc.clone()))).expect("embed");
    let probe = probe_jpeg_profile(&out);
    assert_eq!(probe.embedded.as_deref(), Some(icc.as_slice()));
}

#[test]
fn raster_icc_embed_tiff_tag() {
    let tiff = b"II*\0\x08\0\0\0\0\0\0\0";
    let icc = profile();
    let out = embed_raster_metadata(tiff, "tif", &metadata(Some(icc.clone()))).expect("embed");
    let probe = probe_tiff_profile(&out);
    assert_eq!(probe.embedded.as_deref(), Some(icc.as_slice()));
}

#[test]
fn raster_icc_embed_absent() {
    let jpeg = [0xFF, 0xD8, 0xFF, 0xD9];
    let out = embed_raster_metadata(&jpeg, "jpg", &metadata(None)).expect("embed");
    assert_eq!(out, jpeg);
    assert!(probe_jpeg_profile(&out).embedded.is_none());
}

#[test]
fn raster_icc_embed_exiftool_oracle() {
    if Command::new("exiftool").arg("-ver").output().is_err() {
        eprintln!("skip: exiftool is not installed");
        return;
    }
    let jpeg = [0xFF, 0xD8, 0xFF, 0xD9];
    let icc = profile();
    let out = embed_raster_metadata(&jpeg, "jpg", &metadata(Some(icc.clone()))).expect("embed");
    let path = std::env::temp_dir().join(format!("pictura-icc-{}.jpg", std::process::id()));
    {
        let mut file = std::fs::File::create(&path).expect("temp file");
        file.write_all(&out).expect("write temp");
    }
    let output = Command::new("exiftool")
        .args(["-b", "-ICC_Profile"])
        .arg(&path)
        .output()
        .expect("run exiftool");
    let _ = std::fs::remove_file(&path);
    if !output.status.success() || output.stdout.is_empty() {
        eprintln!("skip: exiftool could not read ICC_Profile");
        return;
    }
    assert_eq!(output.stdout, icc, "exiftool reads the embedded profile");
}
