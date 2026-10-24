//! The command catalogue bridge: the C++ shell builds its menus from these
//! rows instead of maintaining a second hand-authored table.

use self::ffi::CommandSpecRow;
use cxx_qt_lib::{QString, QStringList};
use pictura_script::command_catalogue as catalogue;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[namespace = "pictura"]
    struct CommandSpecRow {
        id: QString,
        path: QStringList,
        label: QString,
        shortcut: QString,
        implemented: bool,
        checkable: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Every catalogue row, in menu order.
        fn command_catalogue() -> Vec<CommandSpecRow>;

        /// The stable id derived from a command path.
        fn command_id_for_path(path: &QStringList) -> QString;
    }
}

fn command_catalogue() -> Vec<CommandSpecRow> {
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

fn command_id_for_path(path: &QStringList) -> QString {
    let parts: Vec<String> = path.iter().map(ToString::to_string).collect();
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    let id = catalogue::command_id_for_path(&refs);
    QString::from(id.as_str())
}
