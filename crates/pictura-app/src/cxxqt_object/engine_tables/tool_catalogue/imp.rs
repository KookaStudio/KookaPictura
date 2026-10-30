use super::ffi::{ToolHintRow, ToolSpecRow};
use cxx_qt_lib::QString;
use pictura_script::tool_catalogue::{tool_catalogue, ToolCursor};

pub(super) fn cursor_name(cursor: ToolCursor) -> &'static str {
    match cursor {
        ToolCursor::Arrow => "arrow",
        ToolCursor::Cross => "cross",
        ToolCursor::IBeam => "ibeam",
        ToolCursor::OpenHand => "openhand",
        ToolCursor::SizeAll => "sizeall",
    }
}

pub(super) fn tool_catalogue_rows() -> Vec<ToolSpecRow> {
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
