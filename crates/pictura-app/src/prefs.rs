//! The application preference document and profile transactions.
#[cfg(test)]
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value as Json};
use toml::{Table, Value};

mod files;
pub(crate) mod profile;
mod schema;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct Store {
    root: PathBuf,
    system_paths: Vec<PathBuf>,
}

pub(crate) struct Load {
    pub values: Map<String, Json>,
    pub notice: Option<String>,
    pub can_clean_legacy: bool,
    pub agent_guard: bool,
}

struct Document {
    table: Table,
    bytes: Vec<u8>,
    migrated: bool,
}

enum Local {
    Current(Document),
    Future(Table),
    Missing,
    Invalid(String),
    Inaccessible(io::Error),
}

impl Store {
    pub(crate) fn new(root: PathBuf, system_paths: Vec<PathBuf>) -> Self {
        Self { root, system_paths }
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.root.join("prefs.toml")
    }

    fn backup_path(&self) -> PathBuf {
        self.root.join("prefs.toml.bak")
    }

    fn lock(&self) -> io::Result<File> {
        files::create_dir(&self.root)?;
        let lock = files::lock_file(&self.root.join("prefs.lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    fn fallback_values(&self) -> Map<String, Json> {
        let mut values = schema::defaults();
        for path in self.system_paths.iter().rev() {
            if let Local::Current(doc) = read_document(path) {
                let _ = schema::overlay(&mut values, &doc.table, false);
            }
        }
        values
    }

    fn effective(&self, table: &Table, agent_guard: bool) -> Map<String, Json> {
        let mut values = self.fallback_values();
        let _ = schema::overlay(&mut values, table, true);
        if agent_guard {
            schema::restrict_agent_values(&mut values);
        }
        values
    }

    fn loaded(&self, table: &Table, can_clean_legacy: bool, notice: Option<String>) -> Load {
        let agent_guard = schema::guarded(table);
        Load {
            values: self.effective(table, agent_guard),
            notice,
            can_clean_legacy,
            agent_guard,
        }
    }

    fn backup_for_recovery(&self) -> Table {
        let mut table = match read_document(&self.backup_path()) {
            Local::Current(doc) => doc.table,
            _ => schema::new_document(),
        };
        schema::restrict_agent(&mut table);
        table
    }

    pub(crate) fn load(&self, legacy: &Map<String, Json>, defer_legacy_write: bool) -> Load {
        let path = self.path();
        if let Local::Current(doc) = read_document(&path) {
            if !doc.migrated {
                return self.loaded(&doc.table, true, None);
            }
        }

        let lock = self.lock();
        match read_document(&path) {
            Local::Current(doc) => {
                if !doc.migrated {
                    return self.loaded(&doc.table, true, None);
                }
                let saved = lock
                    .map_err(|e| e.to_string())
                    .and_then(|_lock| self.write(&doc.table, Some(&doc)));
                let ready = saved.is_ok();
                self.loaded(&doc.table, ready, saved.err())
            }
            Local::Future(table) => self.loaded(
                &table,
                false,
                Some(
                    "This preferences file was written by a newer version and is read-only.".into(),
                ),
            ),
            Local::Missing => {
                let mut table = schema::new_document();
                let values = schema::legacy_values(legacy);
                let prepared = schema::apply(&mut table, &values, &Map::new(), false);
                if defer_legacy_write {
                    return self.loaded(
                        &table,
                        false,
                        Some("The legacy profile is busy. Using its preferences in memory.".into()),
                    );
                }
                let saved = prepared
                    .and_then(|()| lock.map_err(|e| e.to_string()))
                    .and_then(|_lock| self.write(&table, None));
                let ready = saved.is_ok();
                self.loaded(&table, ready, saved.err())
            }
            Local::Invalid(reason) => {
                let table = self.backup_for_recovery();
                let recovery = lock.map_err(|e| e.to_string()).and_then(|_lock| {
                    let quarantine = files::quarantine(&path).map_err(|e| e.to_string())?;
                    self.write(&table, None)?;
                    Ok(quarantine)
                });
                match recovery {
                    Ok(quarantine) => self.loaded(
                        &table,
                        true,
                        Some(format!(
                            "Preferences were recovered. The damaged file is at {}.",
                            quarantine.display()
                        )),
                    ),
                    Err(error) => self.loaded(
                        &table,
                        false,
                        Some(format!(
                            "Preferences could not be repaired on disk: {reason}. {error}"
                        )),
                    ),
                }
            }
            Local::Inaccessible(error) => {
                let mut values = self.fallback_values();
                values.extend(schema::legacy_values(legacy));
                schema::restrict_agent_values(&mut values);
                Load {
                    values,
                    notice: Some(format!("Preferences could not be read: {error}")),
                    can_clean_legacy: false,
                    agent_guard: true,
                }
            }
        }
    }

    fn current(&self) -> Result<Option<Document>, String> {
        match read_document(&self.path()) {
            Local::Current(doc) => Ok(Some(doc)),
            Local::Missing => Ok(None),
            Local::Future(_) => Err("The newer preferences schema is read-only.".into()),
            Local::Invalid(reason) => Err(reason),
            Local::Inaccessible(err) => Err(err.to_string()),
        }
    }

    fn write(&self, table: &Table, previous: Option<&Document>) -> Result<(), String> {
        let text = schema::serialize(table)?;
        if text.len() as u64 > files::MAX_BYTES {
            return Err("Preferences exceed the size limit.".into());
        }
        if let Some(previous) = previous {
            files::replace(&self.backup_path(), &previous.bytes).map_err(|e| e.to_string())?;
        }
        files::replace(&self.path(), text.as_bytes()).map_err(|e| e.to_string())
    }

    #[cfg(test)]
    pub(crate) fn update(
        &self,
        patch: &Map<String, Json>,
        agent_grant: bool,
    ) -> Result<(), String> {
        self.update_checked(patch, agent_grant, false)
    }

    pub(crate) fn update_checked(
        &self,
        patch: &Map<String, Json>,
        agent_grant: bool,
        require_existing: bool,
    ) -> Result<(), String> {
        let _lock = self.lock().map_err(|e| e.to_string())?;
        let previous = self.current()?;
        if require_existing && previous.is_none() {
            return Err("Legacy preferences must be migrated before saving changes.".into());
        }
        let mut next = previous
            .as_ref()
            .map_or_else(schema::new_document, |doc| doc.table.clone());
        let guarded = schema::guarded(&next);
        if guarded && !agent_grant && patch.keys().any(|key| schema::is_agent_key(key)) {
            return Err(
                "Agent control needs a fresh explicit grant after preference recovery.".into(),
            );
        }
        let values = self.effective(&next, guarded);
        schema::apply(&mut next, patch, &values, true)?;
        if guarded && agent_grant {
            schema::edit_local(&mut next).remove(schema::GUARD);
        }
        if previous
            .as_ref()
            .is_some_and(|doc| doc.table == next && !doc.migrated)
        {
            return Ok(());
        }
        self.write(&next, previous.as_ref())
    }

    pub(crate) fn parse_portable(text: &str) -> Result<Table, String> {
        if text.len() as u64 > files::MAX_BYTES {
            return Err("Preferences exceed the size limit.".into());
        }
        let (mut table, _) = schema::parse(text)?;
        if schema::version(&table)? > schema::CURRENT_SCHEMA {
            return Err("The preferences section was written by a newer version.".into());
        }
        table.remove(schema::LOCAL);
        schema::validate(&table)?;
        Ok(table)
    }

    pub(crate) fn export(&self) -> Result<String, String> {
        let mut table = self
            .current()?
            .map_or_else(schema::new_document, |doc| doc.table);
        table.remove(schema::LOCAL);
        schema::serialize(&table)
    }

    pub(crate) fn apply_import_checked(
        &self,
        legacy: &Map<String, Json>,
        replacement: Option<&str>,
        id: &str,
        require_existing: bool,
    ) -> Result<(), String> {
        let mut patch = schema::legacy_import_values(legacy);
        if patch.is_empty() && replacement.is_none() {
            return Ok(());
        }
        let _lock = self.lock().map_err(|e| e.to_string())?;
        let previous = self.current()?;
        if replacement.is_none() && require_existing && previous.is_none() {
            return Err("Legacy preferences must be migrated before importing changes.".into());
        }
        let current = previous
            .as_ref()
            .map_or_else(schema::new_document, |doc| doc.table.clone());
        if receipts(&current).any(|receipt| receipt == id) {
            return Ok(());
        }
        let mut next = if let Some(text) = replacement {
            let mut replacement = Self::parse_portable(text)?;
            if let Some(local) = current.get(schema::LOCAL) {
                replacement.insert(schema::LOCAL.into(), local.clone());
            }
            replacement
        } else {
            if schema::guarded(&current) {
                patch.retain(|key, _| !schema::is_agent_key(key));
            }
            let mut patched = current.clone();
            let values = self.effective(&current, schema::guarded(&current));
            schema::apply(&mut patched, &patch, &values, false)?;
            patched
        };
        if schema::guarded(&current) {
            schema::restrict_agent(&mut next);
        }
        let list = receipts(&current)
            .chain([id])
            .map(|id| Value::String(id.to_owned()))
            .collect();
        schema::edit_local(&mut next).insert(schema::RECEIPTS.into(), Value::Array(list));
        self.write(&next, previous.as_ref())
    }

    pub(crate) fn retire_receipt(&self, id: &str) -> Result<(), String> {
        let _lock = self.lock().map_err(|e| e.to_string())?;
        let Some(previous) = self.current()? else {
            return Ok(());
        };
        if !receipts(&previous.table).any(|receipt| receipt == id) {
            return Ok(());
        }
        let mut next = previous.table.clone();
        let remaining: Vec<_> = receipts(&previous.table)
            .filter(|receipt| *receipt != id)
            .map(|receipt| Value::String(receipt.to_owned()))
            .collect();
        let local = schema::edit_local(&mut next);
        if remaining.is_empty() {
            local.remove(schema::RECEIPTS);
        } else {
            local.insert(schema::RECEIPTS.into(), Value::Array(remaining));
        }
        if local.is_empty() {
            next.remove(schema::LOCAL);
        }
        // Receipt retirement is metadata-only. Keep the last-good preferences.
        self.write(&next, None)
    }

    pub(super) fn reset_document(&self) -> Result<String, String> {
        let mut next = schema::new_document();
        match read_document(&self.path()) {
            Local::Current(doc) => {
                if let Some(local) = doc.table.get(schema::LOCAL) {
                    next.insert(schema::LOCAL.into(), local.clone());
                }
                if schema::guarded(&doc.table) {
                    schema::restrict_agent(&mut next);
                }
            }
            Local::Invalid(_) | Local::Future(_) => schema::restrict_agent(&mut next),
            Local::Inaccessible(error) => return Err(error.to_string()),
            Local::Missing => {}
        }
        schema::serialize(&next)
    }
}

fn receipts(table: &Table) -> impl Iterator<Item = &str> {
    schema::local(table)
        .and_then(|local| local.get(schema::RECEIPTS))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

fn read_document(path: &Path) -> Local {
    let bytes = match files::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return match error.kind() {
                io::ErrorKind::NotFound => Local::Missing,
                io::ErrorKind::InvalidData => Local::Invalid(error.to_string()),
                _ => Local::Inaccessible(error),
            }
        }
    };
    let parsed = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())
        .and_then(schema::parse);
    match parsed {
        Ok((table, _)) if schema::version(&table).is_ok_and(|v| v > schema::CURRENT_SCHEMA) => {
            Local::Future(table)
        }
        Ok((table, migrated)) => match schema::validate(&table) {
            Ok(()) => Local::Current(Document {
                table,
                bytes,
                migrated,
            }),
            Err(err) => Local::Invalid(err),
        },
        Err(err) => Local::Invalid(err),
    }
}

pub(crate) fn legacy_keys() -> &'static [&'static str] {
    schema::KEYS
}

pub(crate) fn default_values() -> Map<String, Json> {
    schema::defaults()
}
