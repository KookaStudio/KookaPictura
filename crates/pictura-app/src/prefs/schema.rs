use std::collections::BTreeMap;

use serde_json::{Map, Value as Json};
use toml::{Table, Value};

pub(super) const CURRENT_SCHEMA: i64 = 1;
pub(super) const VERSION: &str = "schema_version";
pub(super) const LOCAL: &str = "_local";
pub(super) const GUARD: &str = "agent_recovery_guard";
pub(super) const RECEIPTS: &str = "import_receipts";

macro_rules! fields {
    ($($group:literal / $key:literal : $ty:ty = $default:expr;)*) => {
        pub(super) const KEYS: &[&str] = &[$($key,)*];

        fn group(key: &str) -> Option<&'static str> {
            match key {
                $($key => Some($group),)*
                _ => None,
            }
        }

        pub(super) fn defaults() -> Map<String, Json> {
            Map::from_iter([$(
                ($key.into(), serde_json::to_value::<$ty>($default).unwrap()),
            )*])
        }

        fn decode(key: &str, value: Value) -> Result<Json, String> {
            match key {
                $($key => value.try_into::<$ty>().map(|v| serde_json::to_value(v).unwrap())
                    .map_err(|e| format!("{key}: {e}")),)*
                _ => Err(format!("Unknown preference {key}")),
            }
        }

        fn encode(key: &str, value: Json) -> Result<Value, String> {
            match key {
                $($key => serde_json::from_value::<$ty>(value)
                    .map_err(|e| format!("{key}: {e}"))
                    .and_then(|value| Value::try_from(value).map_err(|e| format!("{key}: {e}"))),)*
                _ => Err(format!("Unknown preference {key}")),
            }
        }
    };
}

fields! {
    "interface" / "brightnessLevel": i32 = 1;
    "interface" / "themeFamily": i32 = 0;
    "interface" / "uiScalePercent": i32 = 100;
    "interface" / "openDocumentsAsTabs": bool = true;
    "interface" / "enableFloatingDocumentDocking": bool = true;
    "performance" / "gpuCompute": bool = true;
    "performance" / "memoryUsagePercent": i32 = 60;
    "performance" / "scratchVolumes": Vec<String> = Vec::new();
    "color" / "colorPolicy": i32 = 0;
    "color" / "cmykPolicy": i32 = 0;
    "color" / "workingCmyk": i32 = 0;
    "color" / "renderIntent": i32 = 1;
    "color" / "blackPointCompensation": bool = true;
    "color" / "useDither": bool = true;
    "color" / "askMismatchOpen": bool = false;
    "color" / "askMissingOpen": bool = false;
    "color" / "textGammaEnabled": bool = true;
    "color" / "textGamma": f64 = 1.45;
    "general" / "pickerChoice": i32 = 0;
    "general" / "useShiftKeyForToolSwitch": bool = true;
    "general" / "zoomResizesWindows": bool = false;
    "updates" / "checkForUpdatesOnLaunch": bool = true;
    "type" / "typePlaceholderText": bool = true;
    "type" / "typeShowAsianOptions": bool = false;
    "type" / "typeMiddleEastern": bool = false;
    "type" / "typeMissingGlyphProtection": bool = true;
    "warnings" / "confirmLiveShapeToPath": bool = true;
    "warnings" / "confirmDeleteVectorMask": bool = true;
    "warnings" / "showFullScreenMessage": bool = true;
    "warnings" / "showSmartFilterEditMessage": bool = true;
    "fileHandling" / "saveInBackground": bool = true;
    "fileHandling" / "autoSaveRecovery": bool = true;
    "fileHandling" / "autoSaveMinutes": i32 = 10;
    "fileHandling" / "nativeSaveCompatibility": i32 = 0;
    "fileHandling" / "rawSettingsStore": i32 = 0;
    "agent" / "agentControlEnabled": bool = false;
    "agent" / "agentAllowEdit": bool = false;
    "agent" / "agentAllowSave": bool = false;
    "agent" / "agentApprovalMode": i32 = 1;
    "agent" / "agentRoots": Vec<String> = Vec::new();
    "unitsAndGuides" / "rulerUnit": i32 = 1;
    "unitsAndGuides" / "traditionalPoints": bool = false;
    "unitsAndGuides" / "guideColor": String = "#4affff".into();
    "unitsAndGuides" / "guideDashed": bool = false;
    "keyboard" / "shortcuts": BTreeMap<String, Vec<String>> = BTreeMap::new();
    "keyboard" / "toolShortcuts": BTreeMap<String, String> = BTreeMap::new();
}

pub(super) fn version(table: &Table) -> Result<i64, String> {
    table
        .get(VERSION)
        .and_then(Value::as_integer)
        .filter(|v| *v >= 0)
        .ok_or_else(|| "Missing or invalid schema_version".to_owned())
}

pub(super) fn new_document() -> Table {
    Table::from_iter([(VERSION.into(), Value::Integer(CURRENT_SCHEMA))])
}

pub(super) fn parse(text: &str) -> Result<(Table, bool), String> {
    let mut table = text.parse::<Table>().map_err(|e| e.to_string())?;
    let original = version(&table)?;
    let mut declared = original;
    while declared < CURRENT_SCHEMA {
        match declared {
            0 => {
                // The initial field layout also accepts a pre-release version 0 document.
                table.insert(VERSION.into(), Value::Integer(1));
            }
            _ => return Err(format!("Unsupported preferences schema {declared}")),
        }
        declared = version(&table)?;
    }
    Ok((table, original != declared))
}

pub(super) fn serialize(table: &Table) -> Result<String, String> {
    let mut body = table.clone();
    let declared = version(table)?;
    body.remove(VERSION);
    let rest = toml::to_string_pretty(&body).map_err(|e| e.to_string())?;
    Ok(format!("{VERSION} = {declared}\n{rest}"))
}

fn field<'a>(table: &'a Table, key: &str) -> Result<Option<&'a Value>, String> {
    let section = group(key).ok_or_else(|| format!("Unknown preference {key}"))?;
    match table.get(section) {
        Some(Value::Table(values)) => Ok(values.get(key)),
        None => Ok(None),
        Some(_) => Err(format!("{section} must be a table")),
    }
}

pub(super) fn overlay(
    values: &mut Map<String, Json>,
    table: &Table,
    compatible_only: bool,
) -> Result<(), String> {
    for &key in KEYS {
        let value = field(table, key).and_then(|value| {
            value
                .map(|value| decode(key, value.clone()).and_then(|v| normalize(key, v)))
                .transpose()
        });
        match value {
            Ok(Some(value)) => {
                values.insert(key.into(), value);
            }
            Ok(None) => {}
            Err(_) if compatible_only => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

pub(super) fn validate(table: &Table) -> Result<(), String> {
    overlay(&mut Map::new(), table, false)?;
    if let Some(local) = table.get(LOCAL) {
        let local = local
            .as_table()
            .ok_or_else(|| format!("{LOCAL} must be a table"))?;
        if local
            .get(GUARD)
            .is_some_and(|value| value.as_bool().is_none())
        {
            return Err(format!("{GUARD} must be a boolean"));
        }
        if local.get(RECEIPTS).is_some_and(|value| {
            value
                .as_array()
                .is_none_or(|ids| ids.iter().any(|id| id.as_str().is_none()))
        }) {
            return Err(format!("{RECEIPTS} must be a string array"));
        }
    }
    Ok(())
}

fn normalize(key: &str, value: Json) -> Result<Json, String> {
    let int = || value.as_i64().unwrap_or_default();
    let bounded = |range: std::ops::RangeInclusive<i64>, default| {
        if range.contains(&int()) {
            int()
        } else {
            default
        }
    };
    let setting = match key {
        "memoryUsagePercent" => Json::from(int().clamp(5, 100)),
        "textGamma" => {
            let gamma = value.as_f64().ok_or("textGamma must be finite")?;
            Json::from(gamma.clamp(1.0, 2.2))
        }
        "scratchVolumes" => Json::Array(
            value
                .as_array()
                .into_iter()
                .flatten()
                .filter(|volume| volume.as_str().is_some_and(|s| !s.is_empty()))
                .take(4)
                .cloned()
                .collect(),
        ),
        "colorPolicy" | "cmykPolicy" | "nativeSaveCompatibility" => Json::from(bounded(0..=2, 0)),
        "renderIntent" => Json::from(bounded(0..=3, 1)),
        "pickerChoice" | "rawSettingsStore" => Json::from(bounded(0..=1, 0)),
        "agentApprovalMode" => Json::from(int().clamp(0, 2)),
        "autoSaveMinutes" if ![5, 10, 15, 30, 60].contains(&int()) => Json::from(10),
        _ => value,
    };
    Ok(setting)
}

pub(super) fn set(table: &mut Table, key: &str, value: Json) -> Result<(), String> {
    let section = group(key).ok_or_else(|| format!("Unknown preference {key}"))?;
    let typed = encode(key, value)?;
    let normalized = normalize(key, decode(key, typed)?)?;
    let value = encode(key, normalized)?;
    let section = table
        .entry(section.to_owned())
        .or_insert_with(|| Value::Table(Table::new()));
    let values = section
        .as_table_mut()
        .ok_or_else(|| format!("{key} has an invalid table"))?;
    values.insert(key.to_owned(), value);
    Ok(())
}

pub(super) fn legacy_values(legacy: &Map<String, Json>) -> Map<String, Json> {
    legacy_values_inner(legacy, false)
}

pub(super) fn legacy_import_values(legacy: &Map<String, Json>) -> Map<String, Json> {
    legacy_values_inner(legacy, true)
}

fn legacy_values_inner(legacy: &Map<String, Json>, import: bool) -> Map<String, Json> {
    let schema = legacy
        .get("schemaVersion")
        .and_then(Json::as_i64)
        .unwrap_or(1);
    let mut values = Map::new();
    for &key in KEYS {
        if key == "useShiftKeyForToolSwitch" && schema < 4 && !import {
            continue;
        }
        let Some(value) = legacy.get(key) else {
            continue;
        };
        let value = match (key, value) {
            ("scratchVolumes" | "agentRoots", Json::Array(items)) => Json::Array(
                items
                    .iter()
                    .filter(|item| item.is_string())
                    .cloned()
                    .collect(),
            ),
            ("shortcuts", Json::Object(entries)) => Json::Object(
                entries
                    .iter()
                    .map(|(name, bindings)| {
                        let bindings = bindings
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|binding| binding.is_string())
                            .cloned()
                            .collect();
                        (name.clone(), Json::Array(bindings))
                    })
                    .collect(),
            ),
            ("toolShortcuts", Json::Object(entries)) => Json::Object(
                entries
                    .iter()
                    .filter(|(_, value)| value.is_string())
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            ),
            ("textGamma", value) if value.is_number() => Json::from(value.as_f64().unwrap_or(1.45)),
            (_, value)
                if value
                    .as_f64()
                    .is_some_and(|n| n.fract() == 0.0 && n.abs() <= f64::from(i32::MAX)) =>
            {
                Json::from(value.as_f64().unwrap_or_default() as i32)
            }
            (_, value) => value.clone(),
        };
        if let Ok(value) = encode(key, value)
            .and_then(|value| decode(key, value))
            .and_then(|value| normalize(key, value))
        {
            values.insert(key.into(), value);
        }
    }
    values
}

pub(super) fn apply(
    table: &mut Table,
    patch: &Map<String, Json>,
    values: &Map<String, Json>,
    merge_shortcuts: bool,
) -> Result<(), String> {
    for (key, value) in patch {
        if merge_shortcuts && matches!(key.as_str(), "shortcuts" | "toolShortcuts") {
            let changes = value
                .as_object()
                .ok_or_else(|| format!("{key} must be an object"))?;
            let mut entries = values
                .get(key)
                .and_then(Json::as_object)
                .cloned()
                .unwrap_or_default();
            for (id, binding) in changes {
                if binding.is_null() {
                    entries.remove(id);
                } else {
                    entries.insert(id.clone(), binding.clone());
                }
            }
            set(table, key, Json::Object(entries))?;
        } else {
            set(table, key, value.clone())?;
        }
    }
    Ok(())
}

pub(super) fn local(table: &Table) -> Option<&Table> {
    table.get(LOCAL).and_then(Value::as_table)
}

pub(super) fn edit_local(table: &mut Table) -> &mut Table {
    table
        .entry(LOCAL.to_owned())
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .expect("validated local preferences metadata")
}

pub(super) fn guarded(table: &Table) -> bool {
    local(table)
        .and_then(|local| local.get(GUARD))
        .and_then(Value::as_bool)
        == Some(true)
}

pub(super) fn restrict_agent(table: &mut Table) {
    let agent = table
        .entry("agent".to_owned())
        .or_insert_with(|| Value::Table(Table::new()));
    let agent = agent.as_table_mut().expect("validated agent settings");
    agent.extend([
        ("agentControlEnabled".into(), Value::Boolean(false)),
        ("agentAllowEdit".into(), Value::Boolean(false)),
        ("agentAllowSave".into(), Value::Boolean(false)),
        ("agentApprovalMode".into(), Value::Integer(0)),
        ("agentRoots".into(), Value::Array(Vec::new())),
    ]);
    edit_local(table).insert(GUARD.into(), Value::Boolean(true));
}

pub(super) fn restrict_agent_values(values: &mut Map<String, Json>) {
    values.insert("agentControlEnabled".into(), Json::Bool(false));
    values.insert("agentAllowEdit".into(), Json::Bool(false));
    values.insert("agentAllowSave".into(), Json::Bool(false));
    values.insert("agentApprovalMode".into(), Json::from(0));
    values.insert("agentRoots".into(), Json::Array(Vec::new()));
}

pub(super) fn is_agent_key(key: &str) -> bool {
    group(key) == Some("agent")
}
