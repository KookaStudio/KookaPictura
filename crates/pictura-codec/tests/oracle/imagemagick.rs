use super::*;

/// ImageMagick opens a written RLE document and reports its dimensions.
#[test]
fn imagemagick_reads_written_rle() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }

    let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .edit_data(|data| {
            for (i, b) in data.iter_mut().enumerate() {
                *b = (i * 7 + 1) as u8;
            }
        })
        .expect("pixel edit preserves geometry");

    let dir = scratch_dir("psd-rle-magick");
    let path = dir.join("rle.psd");
    std::fs::write(&path, write_psd(&doc).unwrap()).unwrap();

    let out = std::process::Command::new("magick")
        .arg("identify")
        .arg("-format")
        .arg("%wx%h")
        .arg(&path)
        .output()
        .expect("run magick");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "magick failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "4x2");
}
