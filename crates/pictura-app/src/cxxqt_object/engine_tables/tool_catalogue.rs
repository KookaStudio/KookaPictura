//! The tool catalogue bridge: the shell's `ToolInfo` table, hints and tool lists
//! are built from `pictura_script::tool_catalogue` rows.

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    struct ToolHintRow {
        key: QString,
        text: QString,
        /// Empty, or the command whose shortcut labels the hint.
        command_id: QString,
    }

    #[namespace = "pictura"]
    struct ToolSpecRow {
        name: QString,
        label: QString,
        group: i32,
        /// The default letter; empty for none.
        shortcut: QString,
        /// `"arrow"`, `"cross"`, `"ibeam"`, `"openhand"`, or `"sizeall"`.
        cursor: QString,
        hotspot_x: i32,
        hotspot_y: i32,
        hint: QString,
        hints: Vec<ToolHintRow>,
        implemented: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Every tool in `ToolId` order.
        fn tool_catalogue_rows() -> Vec<ToolSpecRow>;
    }
}
