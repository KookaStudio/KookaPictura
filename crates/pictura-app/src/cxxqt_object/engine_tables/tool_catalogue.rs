//! The tool catalogue bridge: the shell's `ToolInfo` table, hints and tool lists
//! are built from `pictura_script::tool_catalogue` rows.

use self::ffi::{ToolHintRow, ToolSpecRow};
use cxx_qt_lib::QString;
use pictura_script::tool_catalogue::{tool_catalogue, ToolCursor};

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

fn cursor_name(cursor: ToolCursor) -> &'static str {
    match cursor {
        ToolCursor::Arrow => "arrow",
        ToolCursor::Cross => "cross",
        ToolCursor::IBeam => "ibeam",
        ToolCursor::OpenHand => "openhand",
        ToolCursor::SizeAll => "sizeall",
    }
}

fn tool_catalogue_rows() -> Vec<ToolSpecRow> {
    tool_catalogue()
        .iter()
        .map(|spec| ToolSpecRow {
            name: QString::from(spec.name),
            label: QString::from(spec.label),
            group: i32::from(spec.group),
            shortcut: QString::from(spec.shortcut.map(String::from).unwrap_or_default().as_str()),
            cursor: QString::from(cursor_name(spec.cursor)),
            hotspot_x: i32::from(spec.hotspot.0),
            hotspot_y: i32::from(spec.hotspot.1),
            hint: QString::from(spec.hint),
            hints: spec
                .modifier_hints
                .iter()
                .map(|hint| ToolHintRow {
                    key: QString::from(hint.key),
                    text: QString::from(hint.text),
                    command_id: QString::from(hint.command_id),
                })
                .collect(),
            implemented: spec.implemented,
        })
        .collect()
}
