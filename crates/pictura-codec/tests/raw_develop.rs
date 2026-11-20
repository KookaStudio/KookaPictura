//! The raw open's decode and develop (`codec/raw-develop`, #400) on synthetic
//! DNGs and CR2s and on hand-built camera images.

use pictura_codec::{
    as_shot_transform, crs_settings_from_xmp, decode_raw_source, develop_raw, downscale_camera,
    raw_xmp, read_dng, CameraRgb, DngColor, DngCrop, ImageBudget, PsdError, RawLook, WhiteBalance,
};
use pictura_color::chroma;

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "raw/cr2_builder.rs"]
mod cr2_builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use builder::{dng, noise_plane, Options, Val};
use cr2_builder::{cr2, Cr2};

const D50: u16 = 23;

/// An orientation, its output size, and where the marked pixel lands.
type Placement = (u16, (usize, usize), (usize, usize));

/// A camera whose native RGB is linear ProPhoto: `ColorMatrix1` is
/// XYZ-D50-to-ProPhoto at D50.
fn pro_photo_color() -> DngColor {
    let m = chroma::xyz_d50_to_pro_photo();
    DngColor {
        color_matrix: [Some(std::array::from_fn(|i| m[i / 3][i % 3])), None],
        calibration_illuminant: [Some(D50), None],
        as_shot_neutral: Some([1.0, 1.0, 1.0]),
        ..DngColor::default()
    }
}

fn look(width: usize, height: usize) -> RawLook {
    RawLook {
        color: pro_photo_color(),
        crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        },
        orientation: 1,
    }
}

fn as_shot() -> WhiteBalance {
    as_shot_transform(&pro_photo_color()).unwrap().white_balance
}

fn uniform(width: usize, height: usize, value: f64) -> CameraRgb {
    CameraRgb {
        width,
        height,
        rgb: vec![(value * 65535.0).round() as u16; width * height * 3],
    }
}

fn srgb_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn mean_channels(rgb: &[u16]) -> [f64; 3] {
    let n = (rgb.len() / 3) as f64;
    let mut sum = [0f64; 3];
    for px in rgb.as_chunks::<3>().0.iter() {
        for c in 0..3 {
            sum[c] += px[c] as f64;
        }
    }
    sum.map(|s| s / n)
}

fn srational(m: [f64; 9]) -> Val {
    Val::SRational(
        m.iter()
            .map(|&v| ((v * 10000.0).round() as i32, 10000))
            .collect(),
    )
}

fn colour_tags() -> Vec<(u16, Val)> {
    let color = pro_photo_color();
    vec![
        (50721, srational(color.color_matrix[0].unwrap())),
        (50778, Val::Short(vec![D50])),
        (50728, Val::Rational(vec![(1, 1), (1, 1), (1, 1)])),
    ]
}

fn test_dng(extra: Vec<(u16, Val)>, with_colour: bool) -> Vec<u8> {
    let mut tags = if with_colour {
        colour_tags()
    } else {
        Vec::new()
    };
    tags.extend(extra);
    dng(
        &noise_plane(16, 12, 65535, 9),
        Options {
            raw_tags: tags,
            ..Options::default()
        },
    )
}

#[test]
fn dng_decodes_to_raw_source() {
    let bytes = test_dng(
        vec![
            (50719, Val::Long(vec![2, 2])),
            (50720, Val::Long(vec![12, 8])),
            (274, Val::Short(vec![6])),
        ],
        true,
    );
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("decodes");
    assert_eq!((source.camera.width, source.camera.height), (16, 12));
    assert_eq!(source.camera.rgb.len(), 16 * 12 * 3);
    assert_eq!(
        source.look.crop,
        DngCrop {
            x: 2.0,
            y: 2.0,
            width: 12.0,
            height: 8.0
        }
    );
    assert_eq!(source.look.orientation, 6);
    let image = read_dng(&bytes, &ImageBudget::default()).unwrap();
    assert_eq!(
        source.as_shot,
        as_shot_transform(&image.color).unwrap().white_balance
    );
}

#[test]
fn colourless_files_are_refused_before_demosaic() {
    let bytes = test_dng(vec![(50708, Val::Ascii("Test Cam"))], false);
    match decode_raw_source(&bytes, &ImageBudget::default()) {
        Err(PsdError::Unsupported(message)) => {
            assert!(message.contains("camera colour"), "{message}");
            assert!(message.contains("Test Cam"), "{message}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    let cr2_bytes = cr2(&Cr2::new(noise_plane(32, 16, 4095, 4), 12));
    assert!(matches!(
        decode_raw_source(&cr2_bytes, &ImageBudget::default()),
        Err(PsdError::Unsupported(_))
    ));
}

#[test]
fn non_raw_bytes_are_refused() {
    let tiff = dng(
        &noise_plane(8, 8, 4095, 1),
        Options {
            dng_version: false,
            ..Options::default()
        },
    );
    for bytes in [tiff, Vec::new()] {
        assert!(matches!(
            decode_raw_source(&bytes, &ImageBudget::default()),
            Err(PsdError::Unsupported(_))
        ));
    }
}

#[test]
fn neutral_grey_develops_to_srgb_grey() {
    let out = develop_raw(uniform(6, 4, 0.18), &look(6, 4), as_shot()).unwrap();
    assert_eq!((out.width, out.height), (6, 4));
    let want = srgb_encode(0.18) * 65535.0;
    for px in out.rgb.as_chunks::<3>().0.iter() {
        let (lo, hi) = (*px.iter().min().unwrap(), *px.iter().max().unwrap());
        assert!(hi - lo <= 64, "not grey: {px:?}");
        assert!(
            (px[1] as f64 - want).abs() <= 0.01 * 65535.0,
            "{px:?} vs {want}"
        );
    }
}

#[test]
fn warmer_temperature_is_redder() {
    let ratio = |kelvin: f64| {
        let wb = WhiteBalance {
            temperature: kelvin,
            tint: 0.0,
        };
        let out = develop_raw(uniform(4, 4, 0.18), &look(4, 4), wb).unwrap();
        let [r, _, b] = mean_channels(&out.rgb);
        r / b.max(1.0)
    };
    assert!(ratio(9000.0) > ratio(3000.0));
}

#[test]
fn baseline_exposure_scales_linear_light() {
    let mut boosted = look(4, 4);
    boosted.color.baseline_exposure = 1.0;
    let plain = develop_raw(uniform(4, 4, 0.1), &look(4, 4), as_shot()).unwrap();
    let doubled = develop_raw(uniform(4, 4, 0.1), &boosted, as_shot()).unwrap();
    let tol = 0.01 * 65535.0;
    assert!((plain.rgb[1] as f64 - srgb_encode(0.1) * 65535.0).abs() <= tol);
    assert!((doubled.rgb[1] as f64 - srgb_encode(0.2) * 65535.0).abs() <= tol);
    let clipped = develop_raw(uniform(4, 4, 0.8), &boosted, as_shot()).unwrap();
    assert!(
        clipped.rgb.iter().all(|&v| v >= 65535 - 64),
        "{:?}",
        &clipped.rgb[..3]
    );
}

#[test]
fn default_crop_is_applied() {
    let mut camera = uniform(12, 10, 0.1);
    let marked = (12 + 2) * 3;
    camera.rgb[marked..marked + 3].fill(60000);
    let mut cropped = look(12, 10);
    cropped.crop = DngCrop {
        x: 2.0,
        y: 1.0,
        width: 8.0,
        height: 6.0,
    };
    let out = develop_raw(camera, &cropped, as_shot()).unwrap();
    assert_eq!((out.width, out.height), (8, 6));
    let first = out.rgb[1];
    assert!(out
        .rgb
        .as_chunks::<3>()
        .0
        .iter()
        .skip(1)
        .all(|px| px[1] < first));
}

#[test]
fn orientation_rotates_and_mirrors() {
    let expected: [Placement; 8] = [
        (1, (4, 2), (0, 0)),
        (2, (4, 2), (3, 0)),
        (3, (4, 2), (3, 1)),
        (4, (4, 2), (0, 1)),
        (5, (2, 4), (0, 0)),
        (6, (2, 4), (1, 0)),
        (7, (2, 4), (1, 3)),
        (8, (2, 4), (0, 3)),
    ];
    for (orientation, size, at) in expected {
        let mut camera = uniform(4, 2, 0.1);
        camera.rgb[..3].fill(60000);
        let mut oriented = look(4, 2);
        oriented.orientation = orientation;
        let out = develop_raw(camera, &oriented, as_shot()).unwrap();
        assert_eq!((out.width, out.height), size, "orientation {orientation}");
        let brightest = out
            .rgb
            .as_chunks::<3>()
            .0
            .iter()
            .enumerate()
            .max_by_key(|(_, px)| px[1])
            .map(|(i, _)| (i % out.width, i / out.width))
            .unwrap();
        assert_eq!(brightest, at, "orientation {orientation}");
    }
}

#[test]
fn downscale_averages_and_bounds() {
    let (w, h) = (40, 20);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let v = if (x / 2 + y / 2) % 2 == 0 { 0 } else { 4000 };
            rgb.extend([v, v, v]);
        }
    }
    let image = CameraRgb {
        width: w,
        height: h,
        rgb,
    };
    let small = downscale_camera(&image, 10);
    assert_eq!((small.width, small.height), (10, 5));
    assert!(small.rgb.iter().all(|&v| v.abs_diff(2000) <= 1));
    let tiny = uniform(6, 4, 0.3);
    assert_eq!(downscale_camera(&tiny, 10), tiny);
}

const PACKET: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Temperature="4300" crs:Exposure2012="+0.50"/></rdf:RDF></x:xmpmeta>"#;

#[test]
fn embedded_crs_settings_are_read() {
    let bytes = test_dng(vec![(700, Val::Byte(PACKET.as_bytes().to_vec()))], true);
    let packet = raw_xmp(&bytes).expect("embedded XMP");
    assert_eq!(packet, PACKET.as_bytes());
    let settings = crs_settings_from_xmp(&packet);
    assert_eq!(settings.temperature, Some(4300.0));
    assert_eq!(settings.exposure, Some(0.5));
}

#[test]
fn missing_xmp_reads_none() {
    assert_eq!(raw_xmp(&test_dng(Vec::new(), true)), None);
    assert_eq!(raw_xmp(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"), None);
}
