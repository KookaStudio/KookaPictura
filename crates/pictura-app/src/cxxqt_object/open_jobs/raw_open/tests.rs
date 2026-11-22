//! The raw open's seed, preview and develop (`document/raw-open`, #400) on a
//! minimal uncompressed DNG written here.

use super::imp::*;
use super::store::{RawStores, StoreKind};
use cxx_qt_lib::QString;
use pictura_codec::ImageBudget;
use pictura_color::chroma;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fresh, empty directory for one test's files.
pub(super) fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("pictura-raw-open-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) const PACKET_4300: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Temperature="4300"/></rdf:RDF></x:xmpmeta>"#;

enum Value {
    Short(Vec<u16>),
    Long(Vec<u32>),
    Byte(Vec<u8>),
    Ascii(&'static str),
    Rational(Vec<(u32, u32)>),
    SRational(Vec<(i32, i32)>),
}

/// A little-endian, single-IFD, uncompressed 16-bit RGGB DNG, `width ×
/// height`, a smooth ramp; `ColorMatrix1` (XYZ-D50-to-ProPhoto at D50) when
/// `with_colour`, and an XMP packet when given.
pub(super) fn fixture_dng(
    width: u32,
    height: u32,
    with_colour: bool,
    xmp: Option<&str>,
) -> Vec<u8> {
    let mut samples = Vec::with_capacity((width * height * 2) as usize);
    for y in 0..height {
        for x in 0..width {
            let v = 8000 + (x * 40000 / width.max(1)) as u16 + (y * 9000 / height.max(1)) as u16;
            samples.extend(v.to_le_bytes());
        }
    }
    let strip_at = 8u32;
    let ifd_at = strip_at + samples.len() as u32;
    let mut tags: Vec<(u16, Value)> = vec![
        (254, Value::Long(vec![0])),
        (256, Value::Long(vec![width])),
        (257, Value::Long(vec![height])),
        (258, Value::Short(vec![16])),
        (259, Value::Short(vec![1])),
        (262, Value::Short(vec![32803])),
        (273, Value::Long(vec![strip_at])),
        (277, Value::Short(vec![1])),
        (278, Value::Long(vec![height])),
        (279, Value::Long(vec![samples.len() as u32])),
        (33421, Value::Short(vec![2, 2])),
        (33422, Value::Byte(vec![0, 1, 1, 2])),
        (50706, Value::Byte(vec![1, 4, 0, 0])),
        (50708, Value::Ascii("Test Cam")),
        (50728, Value::Rational(vec![(1, 1), (1, 1), (1, 1)])),
        (50778, Value::Short(vec![23])),
    ];
    if with_colour {
        let m = chroma::xyz_d50_to_pro_photo();
        let values = m
            .iter()
            .flatten()
            .map(|&v| ((v * 10000.0).round() as i32, 10000))
            .collect();
        tags.push((50721, Value::SRational(values)));
    }
    if let Some(packet) = xmp {
        tags.push((700, Value::Byte(packet.as_bytes().to_vec())));
    }
    tags.sort_by_key(|(tag, _)| *tag);

    let mut out = b"II".to_vec();
    out.extend(42u16.to_le_bytes());
    out.extend(ifd_at.to_le_bytes());
    out.extend(&samples);
    let table_len = 2 + tags.len() as u32 * 12 + 4;
    let mut blob_at = ifd_at + table_len;
    let mut table = Vec::new();
    let mut blobs = Vec::new();
    table.extend((tags.len() as u16).to_le_bytes());
    for (tag, value) in &tags {
        let (typ, count, bytes) = match value {
            Value::Short(v) => (
                3u16,
                v.len(),
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Value::Long(v) => (4, v.len(), v.iter().flat_map(|x| x.to_le_bytes()).collect()),
            Value::Byte(v) => (1, v.len(), v.clone()),
            Value::Ascii(s) => {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                (2, b.len(), b)
            }
            Value::Rational(v) => (
                5,
                v.len(),
                v.iter()
                    .flat_map(|(n, d)| [n.to_le_bytes(), d.to_le_bytes()].concat())
                    .collect(),
            ),
            Value::SRational(v) => (
                10,
                v.len(),
                v.iter()
                    .flat_map(|(n, d)| [n.to_le_bytes(), d.to_le_bytes()].concat())
                    .collect::<Vec<u8>>(),
            ),
        };
        table.extend(tag.to_le_bytes());
        table.extend(typ.to_le_bytes());
        table.extend((count as u32).to_le_bytes());
        if bytes.len() <= 4 {
            let mut inline = bytes.clone();
            inline.resize(4, 0);
            table.extend(inline);
        } else {
            table.extend(blob_at.to_le_bytes());
            blob_at += bytes.len() as u32;
            blobs.extend(bytes);
        }
    }
    table.extend(0u32.to_le_bytes());
    out.extend(table);
    out.extend(blobs);
    out
}

pub(super) fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Stores with `preferred` first and the database in `dir/db`.
pub(super) fn stores(dir: &Path, preferred: StoreKind) -> RawStores {
    RawStores {
        preferred,
        database: Some(dir.join("db")),
    }
}

pub(super) fn pending_with(path: &Path, stores: &RawStores) -> PendingRaw {
    decode_pending(
        path.to_str().unwrap(),
        &ImageBudget::default(),
        stores,
        &|_, _| {},
    )
    .expect("fixture decodes")
}

fn pending(path: &Path) -> PendingRaw {
    pending_with(path, &stores(path.parent().unwrap(), StoreKind::Database))
}

/// A packet whose one description carries `attributes`.
pub(super) fn crs_packet(attributes: &str) -> String {
    format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" {attributes}/></rdf:RDF></x:xmpmeta>"#
    )
}

#[test]
fn raw_sniff_detects_by_signature() {
    let dir = temp_dir();
    let dng = write(&dir, "shot.tif", &fixture_dng(16, 12, true, None));
    let mut plain = fixture_dng(16, 12, true, None);
    // Rename the DNGVersion tag so the file is a plain TIFF.
    let at = plain
        .windows(4)
        .position(|w| w == [0x12, 0xC6, 1, 0])
        .expect("DNGVersion entry");
    plain[at..at + 2].copy_from_slice(&0xC5FFu16.to_le_bytes());
    let tiff = write(&dir, "plain.tif", &plain);
    // A TIFF whose IFD0 names make SONY is a Sony ARW, whatever its name.
    let mut sony =
        b"II*\0\x08\0\0\0\x01\0\x0F\x01\x02\0\x05\0\0\0\x1A\0\0\0\0\0\0\0SONY\0".to_vec();
    let arw = write(&dir, "sony.tif", &sony);
    sony[26..30].copy_from_slice(b"SANY");
    let other = write(&dir, "other.tif", &sony);
    // An Olympus header is enough to name an ORF.
    let olympus = write(&dir, "olympus.tif", b"IIRO\x08\0\0\0\0\0\0\0\0\0");
    let sniff = |p: &Path| raw_sniff_path(&QString::from(p.to_str().unwrap()));
    assert!(sniff(&dng));
    assert!(sniff(&arw));
    assert!(sniff(&olympus));
    assert!(!sniff(&other));
    assert!(!sniff(&tiff));
    assert!(!sniff(&dir.join("missing.dng")));
}

#[test]
fn develop_matches_codec_at_seed() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    let pending = pending(&path);
    let seed = pending.seed.clone();
    let codec = pictura_codec::develop_raw(
        pending.source.camera.clone(),
        &pending.source.look,
        pictura_codec::WhiteBalance {
            temperature: seed.basic.temperature.unwrap(),
            tint: seed.basic.tint.unwrap(),
        },
    )
    .unwrap();
    let app = develop_pixels(
        pending.source.camera.clone(),
        &pending.source.look,
        pending.source.as_shot,
        &seed,
    )
    .unwrap();
    assert_eq!((app.width, app.height), (16, 12));
    assert_eq!(app.rgb, codec.rgb);
}

#[test]
fn seed_uses_as_shot_white_balance() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    let pending = pending(&path);
    let as_shot = pending.source.as_shot;
    assert_eq!(
        pending.seed.basic.temperature,
        Some(as_shot.temperature.round())
    );
    assert_eq!(pending.seed.basic.tint, Some(as_shot.tint.round()));
    assert_eq!(pending.seed.basic.exposure, None);
}

#[test]
fn sidecar_overrides_seed() {
    let dir = temp_dir();
    let path = write(
        &dir,
        "shot.dng",
        &fixture_dng(16, 12, true, Some(PACKET_4300)),
    );
    write(
        &dir,
        "shot.xmp",
        br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Temperature="3200" crs:Exposure2012="+1.00"/></rdf:RDF></x:xmpmeta>"#,
    );
    let pending = pending(&path);
    assert_eq!(pending.seed.basic.temperature, Some(3200.0));
    assert_eq!(pending.seed.basic.exposure, Some(1.0));
}

#[test]
fn embedded_xmp_seeds_without_sidecar() {
    let dir = temp_dir();
    let path = write(
        &dir,
        "shot.dng",
        &fixture_dng(16, 12, true, Some(PACKET_4300)),
    );
    assert_eq!(pending(&path).seed.basic.temperature, Some(4300.0));
}

#[test]
fn preview_follows_temperature() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(32, 24, true, None));
    let pending = pending(&path);
    let ratio = |kelvin: f64| {
        let mut settings = pending.seed.clone();
        settings.basic.temperature = Some(kelvin);
        let (rgba, w, h) = preview_rgba(&pending, &settings, 16).expect("preview");
        assert!(w.max(h) <= 16);
        let (mut r, mut b) = (0f64, 0f64);
        for px in rgba.as_chunks::<4>().0.iter() {
            r += px[0] as f64;
            b += px[2] as f64;
        }
        r / b.max(1.0)
    };
    assert!(ratio(9000.0) > ratio(3000.0));
}

#[test]
fn preferred_store_is_read_first() {
    let dir = temp_dir();
    let bytes = fixture_dng(16, 12, true, None);
    let path = write(&dir, "shot.dng", &bytes);
    write(
        &dir,
        "shot.xmp",
        crs_packet(r#"crs:Exposure2012="+1.00""#).as_bytes(),
    );
    let db = dir.join("db");
    std::fs::create_dir_all(&db).unwrap();
    let key = pictura_codec::raw_settings_key(&bytes);
    write(
        &db,
        &format!("{key}.xmp"),
        crs_packet(r#"crs:Exposure2012="-1.00""#).as_bytes(),
    );
    let exposure = |kind| pending_with(&path, &stores(&dir, kind)).seed.basic.exposure;
    assert_eq!(exposure(StoreKind::Database), Some(-1.0));
    assert_eq!(exposure(StoreKind::Sidecar), Some(1.0));
}

#[test]
fn record_seeds_every_tab() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    write(
        &dir,
        "shot.xmp",
        crs_packet(r#"crs:Sharpness="55" crs:SplitToningShadowHue="210""#).as_bytes(),
    );
    let seed = pending(&path).seed;
    assert_eq!(seed.detail.sharpen_amount, Some(55));
    assert_eq!(seed.split_toning.shadow_hue, Some(210));
}

#[test]
fn as_shot_record_keeps_as_shot_balance() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    write(
        &dir,
        "shot.xmp",
        crs_packet(r#"crs:WhiteBalance="As Shot" crs:Temperature="9000""#).as_bytes(),
    );
    let pending = pending(&path);
    assert_eq!(
        pending.seed.basic.temperature,
        Some(pending.source.as_shot.temperature.round())
    );
}
