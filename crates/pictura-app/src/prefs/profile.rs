use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{files, legacy_keys, Store};

const MARKER: &str = ".profile-reset.json";
const PRESENCE_LOCK: &str = ".profile-presence.lock";
pub(crate) const MIGRATED: &str = "preferencesMigrated";
const PREF_FILES: &[&str] = &["prefs.toml", "prefs.toml.bak"];
const STATE_FILES: &[&str] = &["state.json", "workspaces", "pending-settings.json"];
const CLEARED_SESSION_KEYS: &[&str] = &[
    "layout",
    "layoutRevision",
    "toolsColumns",
    "panelRailMode",
    "railWidth",
    "autoCollapseIconic",
    "autoShowHidden",
    "panelGroups",
    "panelColumns",
];

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResetRecord {
    config_root: PathBuf,
    state_root: PathBuf,
    id: String,
    missing: Vec<String>,
}

pub(crate) struct ProfileLease {
    _locks: Vec<File>,
}

fn roots<'a>(config: &'a Path, state: &'a Path) -> Vec<&'a Path> {
    let mut roots = vec![config, state];
    roots.sort_unstable();
    roots.dedup();
    roots
}

fn leases(directories: &[&Path], exclusive: bool, busy_is_error: bool) -> io::Result<Vec<File>> {
    let mut locks = Vec::with_capacity(directories.len());
    for root in directories {
        let path = root.join(PRESENCE_LOCK);
        let lock = files::create_dir(root).and_then(|()| files::lock_file(&path));
        let lock = match lock {
            Ok(lock) => lock,
            Err(error) if !exclusive && shared_read_only_error(&error) => match File::open(&path) {
                Ok(lock) => lock,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            },
            Err(error) => return Err(error),
        };
        if !exclusive {
            lock.lock_shared()?;
        } else if busy_is_error {
            lock.try_lock().map_err(|error| match error {
                std::fs::TryLockError::WouldBlock => io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Another instance is using this profile.",
                ),
                std::fs::TryLockError::Error(error) => error,
            })?;
        } else {
            lock.lock()?;
        }
        locks.push(lock);
    }
    Ok(locks)
}

fn shared_read_only_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem
    )
}

fn canonical(root: &Path) -> PathBuf {
    if let Ok(path) = fs::canonicalize(root) {
        path
    } else if let (Some(parent), Some(name)) = (root.parent(), root.file_name()) {
        canonical(parent).join(name)
    } else {
        root.to_owned()
    }
}

fn marker(path: &Path) -> io::Result<Option<ResetRecord>> {
    let bytes = match files::read(&path.join(MARKER)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut record: ResetRecord = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if record.id.is_empty()
        || !record
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || !record.config_root.is_absolute()
        || !record.state_root.is_absolute()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid profile reset record.",
        ));
    }
    record.config_root = canonical(&record.config_root);
    record.state_root = canonical(&record.state_root);
    if path != record.config_root && path != record.state_root {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The reset record does not own this profile.",
        ));
    }
    Ok(Some(record))
}

fn backup(root: &Path, id: &str) -> PathBuf {
    root.join(format!("reset-backup-{id}"))
}

impl ResetRecord {
    fn roots(&self) -> Vec<&Path> {
        roots(&self.config_root, &self.state_root)
    }

    fn targets(&self) -> impl Iterator<Item = (&Path, &'static str)> {
        PREF_FILES
            .iter()
            .map(|&name| (self.config_root.as_path(), name))
            .chain(
                STATE_FILES
                    .iter()
                    .map(|&name| (self.state_root.as_path(), name)),
            )
    }

    fn commit_path(&self) -> PathBuf {
        backup(&self.state_root, &self.id).join(".committed")
    }

    fn publication_path(&self) -> PathBuf {
        backup(&self.state_root, &self.id).join(".published")
    }

    fn rollback_path(&self) -> PathBuf {
        backup(&self.state_root, &self.id).join(".rolled-back")
    }

    fn publish_records(&self) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        for root in self.roots() {
            files::replace(&root.join(MARKER), &bytes)?;
        }
        Ok(())
    }

    fn close_records(&self) -> io::Result<()> {
        for root in self.roots() {
            if marker(root)?.as_ref() == Some(self) {
                files::remove(&root.join(MARKER))?;
            }
        }
        Ok(())
    }

    fn recover(&self) -> io::Result<()> {
        let mut found = false;
        for root in self.roots() {
            if let Some(record) = marker(root)? {
                if record != *self {
                    return Err(io::Error::other(
                        "The profile has conflicting reset records.",
                    ));
                }
                found = true;
            }
        }
        if !found {
            return Ok(());
        }
        if self.publication_path().try_exists()?
            && !self.commit_path().try_exists()?
            && !self.rollback_path().try_exists()?
        {
            for (root, name) in self.targets() {
                let original = backup(root, &self.id).join(name);
                let active = root.join(name);
                if original.try_exists()? {
                    files::rename(&original, &active)?;
                } else if self.missing.iter().any(|missing| missing == name)
                    && matches!(name, "prefs.toml" | "state.json")
                {
                    files::remove(&active)?;
                }
            }
            files::replace(&self.rollback_path(), b"rolled back\n")?;
        }
        self.close_records()
    }
}

pub(crate) fn open(config: &Path, state: &Path) -> io::Result<ProfileLease> {
    let config = canonical(config);
    let state = canonical(state);
    let directories = roots(&config, &state);
    loop {
        let shared = leases(&directories, false, false)?;
        let mut pending = None;
        for root in &directories {
            if let Some(record) = marker(root)? {
                pending = Some(record);
                break;
            }
        }
        let Some(record) = pending else {
            return Ok(ProfileLease { _locks: shared });
        };
        drop(shared);
        let _exclusive = leases(&record.roots(), true, false)?;
        record.recover()?;
    }
}

fn read_session(path: &Path) -> io::Result<Map<String, Value>> {
    let bytes = match files::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => return Err(error),
    };
    Ok(serde_json::from_slice::<Map<String, Value>>(&bytes).unwrap_or_default())
}

pub(crate) fn reset(config: &Path, state: &Path) -> Result<(), String> {
    reset_inner(config, state).map_err(|error| error.to_string())
}

fn reset_inner(config: &Path, state: &Path) -> io::Result<()> {
    let config = canonical(config);
    let state = canonical(state);
    let directories = roots(&config, &state);
    let _exclusive = leases(&directories, true, true)?;
    for root in &directories {
        if marker(root)?.is_some() {
            return Err(io::Error::other(
                "Restart to recover the previous reset first.",
            ));
        }
    }

    let session_path = state.join("state.json");
    let mut session = read_session(&session_path)?;
    for key in legacy_keys().iter().chain(CLEARED_SESSION_KEYS) {
        session.remove(*key);
    }
    session.insert(MIGRATED.into(), Value::Bool(true));
    let session = serde_json::to_vec(&session).map_err(io::Error::other)?;
    let prefs = Store::new(config.clone(), Vec::new())
        .reset_document()
        .map_err(io::Error::other)?;

    let mut record = ResetRecord {
        config_root: config,
        state_root: state,
        id: files::unique(),
        missing: Vec::new(),
    };
    record.missing = record
        .targets()
        .map(|(root, name)| {
            root.join(name)
                .try_exists()
                .map(|exists| (!exists).then(|| name.to_owned()))
        })
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    for root in record.roots() {
        let archive = backup(root, &record.id);
        files::create_dir(&archive)?;
        files::sync_dir(root)?;
    }
    if let Err(error) = record.publish_records() {
        let _ = record.close_records();
        return Err(error);
    }

    let result = (|| {
        files::replace(&record.publication_path(), b"published\n")?;
        for (root, name) in record.targets() {
            if !record.missing.iter().any(|missing| missing == name) {
                files::rename(&root.join(name), &backup(root, &record.id).join(name))?;
            }
        }
        files::replace(&record.config_root.join("prefs.toml"), prefs.as_bytes())?;
        files::replace(&session_path, &session)?;
        files::replace(&record.commit_path(), b"committed\n")
    })();
    match result {
        Ok(()) => record.close_records(),
        Err(error) => {
            record.recover()?;
            Err(error)
        }
    }
}

pub(crate) fn replace_profile_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    files::create_dir(files::parent(path))?;
    files::replace(path, bytes)
}

pub(crate) fn retire_profile_file(path: &Path) -> io::Result<()> {
    files::remove(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_presence_errors_allow_shared_open() {
        assert!(shared_read_only_error(&io::Error::from(
            io::ErrorKind::ReadOnlyFilesystem
        )));
        assert!(shared_read_only_error(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
        assert!(!shared_read_only_error(&io::Error::from(
            io::ErrorKind::NotFound
        )));
    }
}
