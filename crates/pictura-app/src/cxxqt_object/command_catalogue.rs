//! The command catalogue bridge: the C++ shell builds its menus from these
//! rows instead of maintaining a second hand-authored table.

mod imp;

use imp::*;

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
