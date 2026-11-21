//! Where a raw file's settings record lives (#401, `document/raw-settings-storage`):
//! the per-user Camera Raw database, one `<content hash>.xmp` per file, or the
//! `<stem>.xmp` sidecar beside it. The raw file itself is never written.

use crate::cxxqt_object::helpers::safe_io;
use pictura_codec::{pictura_raw_to_xmp, WhiteBalance};
use pictura_core::PicturaRawSettings;
use std::path::{Path, PathBuf};

/// A record larger than this is not read.
const RECORD_LIMIT: u64 = 4 * 1024 * 1024;

/// The Save Camera Raw Settings In preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum StoreKind {
    Database,
    Sidecar,
}

impl StoreKind {
    /// The preference as the shell passes it: 1 is Sidecar, anything else
    /// the database (the CS6 default).
    pub(super) fn from_index(index: i32) -> Self {
        if index == 1 {
            Self::Sidecar
        } else {
            Self::Database
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Database => Self::Sidecar,
            Self::Sidecar => Self::Database,
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::cxxqt_object) struct RawStores {
    pub(super) preferred: StoreKind,
    /// The database folder; `None` when the user has no data folder.
    pub(super) database: Option<PathBuf>,
}

impl RawStores {
    /// The user's stores: the database under `$XDG_DATA_HOME`, else
    /// `~/.local/share`.
    pub(super) fn for_user(preferred: StoreKind) -> Self {
        Self {
            preferred,
            database: database_dir(&|key| std::env::var(key).ok()),
        }
    }
}

fn database_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let data = env("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env("HOME")
                .filter(|v| !v.is_empty())
                .map(|home| Path::new(&home).join(".local/share"))
        })?;
    Some(data.join("kooka-pictura").join("Camera Raw"))
}

/// The sidecar beside `raw`: an existing `<stem>.XMP` when there is no
/// `<stem>.xmp`, else `<stem>.xmp`.
fn sidecar_path(raw: &Path) -> PathBuf {
    let lower = raw.with_extension("xmp");
    let upper = raw.with_extension("XMP");
    if !lower.exists() && upper.exists() {
        upper
    } else {
        lower
    }
}

fn record_path(kind: StoreKind, raw: &Path, key: &str, stores: &RawStores) -> Option<PathBuf> {
    match kind {
        StoreKind::Sidecar => Some(sidecar_path(raw)),
        StoreKind::Database => stores
            .database
            .as_ref()
            .map(|dir| dir.join(format!("{key}.xmp"))),
    }
}

/// The settings record for `raw` (content hash `key`): the preferred store's,
/// else the other store's.
pub(super) fn read_record(raw: &Path, key: &str, stores: &RawStores) -> Option<Vec<u8>> {
    [stores.preferred, stores.preferred.other()]
        .into_iter()
        .find_map(|kind| {
            let path = record_path(kind, raw, key, stores)?;
            safe_io::read_capped(path, RECORD_LIMIT)
        })
}

/// Where a save went.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum Saved {
    Sidecar,
    Database,
    /// The sidecar could not be written (the reason); the database holds it.
    Fallback(String),
}

/// Write `settings` for `raw` to the preferred store, falling back from the
/// sidecar to the database. `base` is the record the seed came from, the
/// starting packet when the store has none yet.
pub(super) fn save_record(
    raw: &Path,
    key: &str,
    base: Option<&[u8]>,
    settings: &PicturaRawSettings,
    stores: &RawStores,
) -> Result<Saved, String> {
    let write = |kind| write_record(kind, raw, key, base, settings, stores);
    match stores.preferred {
        StoreKind::Database => write(StoreKind::Database).map(|()| Saved::Database),
        StoreKind::Sidecar => match write(StoreKind::Sidecar) {
            Ok(()) => Ok(Saved::Sidecar),
            Err(sidecar) => write(StoreKind::Database)
                .map(|()| Saved::Fallback(sidecar.clone()))
                .map_err(|database| format!("{sidecar}; {database}")),
        },
    }
}

fn write_record(
    kind: StoreKind,
    raw: &Path,
    key: &str,
    base: Option<&[u8]>,
    settings: &PicturaRawSettings,
    stores: &RawStores,
) -> Result<(), String> {
    let path = record_path(kind, raw, key, stores)
        .ok_or_else(|| "there is no Camera Raw database folder".to_string())?;
    let packet = match safe_io::read_capped(&path, RECORD_LIMIT) {
        Some(existing) => pictura_raw_to_xmp(Some(&existing), settings),
        None => base
            .and_then(|base| pictura_raw_to_xmp(Some(base), settings))
            .or_else(|| pictura_raw_to_xmp(None, settings)),
    }
    .ok_or_else(|| format!("{} could not be updated", path.display()))?;
    if kind == StoreKind::Database {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        }
    }
    replace_file(&path, &packet).map_err(|error| format!("{}: {error}", path.display()))
}

/// Write `bytes` beside `path` and rename over it, so a failed write never
/// leaves a truncated record.
pub(super) fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// `settings` with the white-balance label its values call for: As Shot when
/// Temperature and Tint equal the rounded as-shot values, else Custom in place
/// of an unset or As Shot preset (the dialog keeps the preset when a slider
/// moves); any other preset is kept.
pub(super) fn with_white_balance_label(
    settings: &PicturaRawSettings,
    as_shot: WhiteBalance,
) -> PicturaRawSettings {
    let mut out = settings.clone();
    let matches = |value: Option<f64>, shot: f64| value.is_none_or(|v| v.round() == shot.round());
    let unedited = matches(settings.basic.temperature, as_shot.temperature)
        && matches(settings.basic.tint, as_shot.tint);
    let preset = settings.mode.white_balance;
    out.mode.white_balance = if unedited {
        Some(*b"AsSh")
    } else if preset.is_none() || preset == Some(*b"AsSh") {
        Some(*b"Cst ")
    } else {
        preset
    };
    out
}
