//! The raw settings stores (`document/raw-settings-storage`, #401): database by
//! content, the seed record as the write base, the white-balance label, and the
//! read-only fallback.

use super::imp::save_settings;
use super::store::{Saved, StoreKind};
use super::tests::{crs_packet, fixture_dng, pending_with, stores, temp_dir, write};
use std::path::Path;

/// Whether the process can still create files in `dir` (true for root).
fn writable(dir: &Path) -> bool {
    let probe = dir.join(".probe");
    let ok = std::fs::write(&probe, b"x").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

fn set_mode(dir: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn database_record_follows_content() {
    let dir = temp_dir();
    let bytes = fixture_dng(16, 12, true, None);
    let path = write(&dir, "shot.dng", &bytes);
    let db_stores = stores(&dir, StoreKind::Database);
    let pending = pending_with(&path, &db_stores);
    let mut settings = pending.seed.clone();
    settings.basic.exposure = Some(0.75);
    assert_eq!(
        save_settings(&pending, &settings, &db_stores),
        Ok(Saved::Database)
    );
    let key = pictura_codec::raw_settings_key(&bytes);
    assert!(dir.join("db").join(format!("{key}.xmp")).is_file());
    assert!(!dir.join("shot.xmp").exists());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let other = dir.join("elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    let moved = write(&other, "moved.dng", &bytes);
    assert_eq!(
        pending_with(&moved, &db_stores).seed.basic.exposure,
        Some(0.75)
    );
}

#[test]
fn database_record_keeps_sidecar_keys() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    let sidecar = crs_packet(r#"crs:Exposure2012="+1.00" crs:LensProfileName="Some Lens""#);
    write(&dir, "shot.xmp", sidecar.as_bytes());
    let db_stores = stores(&dir, StoreKind::Database);
    let pending = pending_with(&path, &db_stores);
    let mut settings = pending.seed.clone();
    settings.basic.exposure = Some(0.5);
    save_settings(&pending, &settings, &db_stores).expect("saved");
    let record = std::fs::read(dir.join("db").join(format!("{}.xmp", pending.key))).unwrap();
    let text = String::from_utf8(record).unwrap();
    assert!(text.contains(r#"crs:Exposure2012="+0.50""#), "{text}");
    assert!(
        text.contains(r#"crs:LensProfileName="Some Lens""#),
        "{text}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("shot.xmp")).unwrap(),
        sidecar
    );
}

#[test]
fn edited_white_balance_is_custom() {
    let dir = temp_dir();
    let path = write(&dir, "shot.dng", &fixture_dng(16, 12, true, None));
    let sidecar_stores = stores(&dir, StoreKind::Sidecar);
    let pending = pending_with(&path, &sidecar_stores);
    let mut settings = pending.seed.clone();
    settings.mode.white_balance = Some(*b"AsSh");
    assert_eq!(
        save_settings(&pending, &settings, &sidecar_stores),
        Ok(Saved::Sidecar)
    );
    let unedited = std::fs::read_to_string(dir.join("shot.xmp")).unwrap();
    assert!(
        unedited.contains(r#"crs:WhiteBalance="As Shot""#),
        "{unedited}"
    );

    settings.basic.temperature = Some(pending.source.as_shot.temperature.round() + 500.0);
    save_settings(&pending, &settings, &sidecar_stores).expect("saved");
    let edited = std::fs::read_to_string(dir.join("shot.xmp")).unwrap();
    assert!(edited.contains(r#"crs:WhiteBalance="Custom""#), "{edited}");
}

#[test]
fn read_only_folder_falls_back_to_database() {
    let dir = temp_dir();
    let raw_dir = dir.join("card");
    std::fs::create_dir_all(&raw_dir).unwrap();
    let path = write(&raw_dir, "shot.dng", &fixture_dng(16, 12, true, None));
    let sidecar_stores = stores(&dir, StoreKind::Sidecar);
    let pending = pending_with(&path, &sidecar_stores);
    set_mode(&raw_dir, 0o555);
    if writable(&raw_dir) {
        set_mode(&raw_dir, 0o755);
        eprintln!("skipped: the process can write to a read-only folder");
        return;
    }
    let saved = save_settings(&pending, &pending.seed, &sidecar_stores);
    set_mode(&raw_dir, 0o755);
    assert!(matches!(saved, Ok(Saved::Fallback(_))), "{saved:?}");
    assert!(!raw_dir.join("shot.xmp").exists());
    assert!(dir
        .join("db")
        .join(format!("{}.xmp", pending.key))
        .is_file());
}
