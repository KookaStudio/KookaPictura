use super::super::engine_tables::ffi::CommandSpecRow;
use cxx_qt_lib::{QString, QStringList};
use pictura_script::command_catalogue as catalogue;

pub(crate) fn command_catalogue() -> Vec<CommandSpecRow> {
    catalogue::command_catalogue()
        .iter()
        .map(|spec| CommandSpecRow {
            id: QString::from(spec.id),
            path: spec.path.iter().map(|part| QString::from(*part)).collect(),
            label: QString::from(spec.label),
            shortcut: QString::from(spec.shortcut),
            implemented: spec.implemented,
            checkable: spec.checkable,
        })
        .collect()
}

pub(crate) fn command_id_for_path(path: &QStringList) -> QString {
    let parts: Vec<String> = path.iter().map(ToString::to_string).collect();
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    match catalogue::id_for_path(&refs) {
        Some(id) => QString::from(id.as_str()),
        None => {
            eprintln!(
                "pictura: command path is not in the catalogue: {}",
                parts.join(" > ")
            );
            QString::default()
        }
    }
}

pub(crate) fn command_is_modal(id: &QString) -> bool {
    let id = id.to_string();
    catalogue::modal_ids()
        .iter()
        .any(|candidate| *candidate == id)
}
