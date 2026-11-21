use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::prefs::{self, profile, Store};

use super::ffi::PreferencesReply;

pub(super) use crate::prefs::profile::ProfileLease;

fn store(root: &str, systems: &str) -> Store {
    let systems: Vec<PathBuf> = serde_json::from_str(systems).unwrap_or_default();
    Store::new(PathBuf::from(root), systems)
}

pub(super) fn prefs_defaults() -> String {
    Value::Object(prefs::default_values()).to_string()
}

pub(super) fn prefs_load(
    root: &str,
    systems: &str,
    legacy: &str,
    defer_legacy_write: bool,
) -> PreferencesReply {
    let legacy = serde_json::from_str(legacy).unwrap_or_default();
    let snapshot = store(root, systems).load(&legacy, defer_legacy_write);
    PreferencesReply {
        values: Value::Object(snapshot.values).to_string(),
        notice: snapshot.notice.unwrap_or_default(),
        can_clean_legacy: snapshot.can_clean_legacy,
        agent_guard: snapshot.agent_guard,
    }
}

pub(super) fn prefs_update(
    root: &str,
    systems: &str,
    patch: &str,
    grant: bool,
    require_existing: bool,
) -> Result<(), String> {
    let patch: Map<String, Value> = serde_json::from_str(patch).map_err(|e| e.to_string())?;
    store(root, systems).update_checked(&patch, grant, require_existing)
}

pub(super) fn prefs_legacy_keys() -> Vec<String> {
    prefs::legacy_keys()
        .iter()
        .map(|key| (*key).to_owned())
        .collect()
}

pub(super) fn prefs_migration_marker() -> String {
    profile::MIGRATED.to_owned()
}

pub(super) fn prefs_export(root: &str) -> Result<String, String> {
    store(root, "[]").export()
}

pub(super) fn prefs_validate_portable(text: &str) -> Result<(), String> {
    Store::parse_portable(text).map(|_| ())
}

pub(super) fn prefs_apply_import(
    root: &str,
    legacy: &str,
    toml: &str,
    replace: bool,
    id: &str,
    require_existing: bool,
) -> Result<(), String> {
    let legacy = serde_json::from_str(legacy).map_err(|e| e.to_string())?;
    store(root, "[]").apply_import_checked(&legacy, replace.then_some(toml), id, require_existing)
}

pub(super) fn prefs_retire_receipt(root: &str, id: &str) -> Result<(), String> {
    store(root, "[]").retire_receipt(id)
}

pub(super) fn profile_open(config: &str, state: &str) -> std::io::Result<Box<ProfileLease>> {
    profile::open(Path::new(config), Path::new(state)).map(Box::new)
}

pub(super) fn profile_reset(config: &str, state: &str) -> Result<(), String> {
    profile::reset(Path::new(config), Path::new(state))
}

pub(super) fn profile_write(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    profile::replace_profile_file(Path::new(path), bytes)
}

pub(super) fn profile_retire(path: &str) -> std::io::Result<()> {
    profile::retire_profile_file(Path::new(path))
}
