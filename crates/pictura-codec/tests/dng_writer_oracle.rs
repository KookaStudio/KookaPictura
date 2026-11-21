//! DNG writer oracle (`codec/dng-writer`, #402): DNGs written by `write_dng`
//! must open in LibRaw (through `rawpy`) and ExifTool. Each tool self-skips
//! when it is unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

use pictura_codec::{
    write_dng, CfaPattern, DngColor, DngCompatibility, DngCrop, DngPixels, DngRgb8, DngWrite,
};

const W: usize = 64;
const H: usize = 48;

fn available(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pictura-dng-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn color() -> DngColor {
    DngColor {
        color_matrix: [Some([0.8, -0.2, 0.0, -0.3, 1.2, 0.1, 0.0, 0.2, 0.6]), None],
        calibration_illuminant: [Some(21), None],
        as_shot_neutral: Some([0.5, 1.0, 0.75]),
        ..DngColor::default()
    }
}

fn write(path: &Path, pixels: DngPixels, original: Option<(&str, &[u8])>) {
    let look = vec![128u8; W * H * 3];
    let color = color();
    let request = DngWrite {
        width: W,
        height: H,
        pixels,
        color: &color,
        crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: W as f64,
            height: H as f64,
        },
        orientation: 1,
        make: Some("Kooka"),
        model: Some("Kooka Test"),
        look: DngRgb8 {
            width: W,
            height: H,
            rgb: &look,
        },
        preview: Some(DngRgb8 {
            width: W,
            height: H,
            rgb: &look,
        }),
        xmp: None,
        original,
        compatibility: DngCompatibility::CameraRaw6_6,
    };
    let bytes = write_dng(&request, &|_| {}, &|| false).unwrap().unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn rawpy_size(path: &Path) -> Option<(usize, usize)> {
    let out = Command::new("python3")
        .args([
            "-c",
            "import rawpy, sys\nr = rawpy.imread(sys.argv[1])\nprint(r.sizes.raw_width, r.sizes.raw_height)",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        panic!(
            "LibRaw refused {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.split_whitespace().map(|v| v.parse().ok());
    Some((it.next()??, it.next()??))
}

fn exiftool(args: &[&str], path: &Path) -> Vec<u8> {
    let out = Command::new("exiftool")
        .args(args)
        .arg(path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "exiftool failed on {}",
        path.display()
    );
    out.stdout
}

// dng_write_reference_tools
#[test]
fn reference_tools_read_written_dngs() {
    let dir = scratch();
    let mosaic_path = dir.join("mosaic.dng");
    let linear_path = dir.join("linear.dng");
    let samples: Vec<u16> = (0..W * H).map(|i| (i * 131 % 60000) as u16).collect();
    let rgb: Vec<u16> = (0..W * H * 3).map(|i| (i * 71 % 60000) as u16).collect();
    let original: Vec<u8> = (0..150_000u32).map(|i| (i * 13 % 251) as u8).collect();
    let cfa = CfaPattern {
        width: 2,
        height: 2,
        colors: vec![0, 1, 1, 2],
    };
    write(
        &mosaic_path,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        Some(("shot.cr2", &original)),
    );
    write(&linear_path, DngPixels::Linear { rgb: &rgb }, None);

    if available("python3", &["-c", "import rawpy"]) {
        assert_eq!(rawpy_size(&mosaic_path), Some((W, H)));
        assert_eq!(rawpy_size(&linear_path), Some((W, H)));
    } else {
        eprintln!("skipping LibRaw check: python3 with rawpy is not available");
    }

    if available("exiftool", &["-ver"]) {
        for path in [&mosaic_path, &linear_path] {
            let version = exiftool(&["-s3", "-DNGVersion"], path);
            assert_eq!(String::from_utf8_lossy(&version).trim(), "1.4.0.0");
        }
        assert_eq!(
            exiftool(&["-b", "-OriginalRawImage"], &mosaic_path),
            original
        );
    } else {
        eprintln!("skipping ExifTool check: exiftool is not available");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
