//! The tool catalogue: every CS6 tool's identity, label, flyout group, default
//! shortcut, cursor and hints, in the toolbox's frozen order. The shell's
//! `ToolId` enum mirrors this order (a Qt Test checks it) and keeps the
//! handlers, options bars and toolbox rendering; the rows live here.

mod entries;

/// The cursor a tool shows before its SVG cursor asset loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCursor {
    Arrow,
    Cross,
    IBeam,
    OpenHand,
    SizeAll,
}

/// One keycap hint for the status bar. A non-empty `command_id` takes its
/// label from that command's registered shortcut instead of `key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolHint {
    pub key: &'static str,
    pub text: &'static str,
    pub command_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolSpec {
    /// The stable id name, also the `tool.<name>` icon and cursor stem.
    pub name: &'static str,
    pub label: &'static str,
    /// The toolbox flyout slot (1-based).
    pub group: u8,
    /// The default single-letter shortcut; `None` for tools without one.
    pub shortcut: Option<char>,
    pub cursor: ToolCursor,
    pub hotspot: (u8, u8),
    /// The status-bar text hint.
    pub hint: &'static str,
    pub modifier_hints: &'static [ToolHint],
    pub implemented: bool,
}

const fn h(key: &'static str, text: &'static str) -> ToolHint {
    ToolHint {
        key,
        text,
        command_id: "",
    }
}

pub(super) const SELECTION_HINTS: &[ToolHint] = &[
    h("Shift", "Add to selection"),
    h("Alt", "Subtract from selection"),
];

pub(super) const BRUSH_HINTS: &[ToolHint] =
    &[h("[", "Decrease brush size"), h("]", "Increase brush size")];

pub(super) const GRADIENT_HINTS: &[ToolHint] = &[h("Shift", "Constrain to 45°")];

pub(super) const PEN_HINTS: &[ToolHint] = &[
    h("Shift", "Constrain to 45°"),
    h("Alt", "Break the handle"),
    h("Enter", "End the path"),
];

pub(super) const PATH_SELECTION_HINTS: &[ToolHint] =
    &[h("Alt", "Drag a copy"), h("Delete", "Delete the component")];

pub(super) const DIRECT_SELECTION_HINTS: &[ToolHint] = &[
    h("Alt", "Break the handle"),
    h("Alt-click", "Select the component"),
];

pub(super) const SHAPE_HINTS: &[ToolHint] = &[
    h("Shift", "Constrain proportions"),
    h("Alt", "Draw from the centre"),
];

pub(super) const POLYGON_HINTS: &[ToolHint] = &[h("Shift", "Snap the angle to 15°")];

pub(super) const LINE_HINTS: &[ToolHint] = &[h("Shift", "Snap the angle to 45°")];

pub(super) const ROTATE_VIEW_HINTS: &[ToolHint] =
    &[h("Shift", "Snap to 15°"), h("Esc", "Reset View")];

pub(super) const CUSTOM_SHAPE_HINTS: &[ToolHint] = &[
    h("Shift", "Defined proportions"),
    h("Alt", "Draw from the centre"),
];

pub(super) const TYPE_HINTS: &[ToolHint] = &[h("Ctrl+Enter", "Commit"), h("Esc", "Cancel")];

pub(super) const MOVE_HINTS: &[ToolHint] = &[
    h("Arrows", "Nudge 1 px"),
    h("Shift+Arrows", "Nudge 10 px"),
    ToolHint {
        key: "",
        text: "Free transform",
        command_id: "edit.freeTransform",
    },
];

/// Every tool, in `ToolId` order.
pub fn tool_catalogue() -> &'static [ToolSpec] {
    &entries::ROWS
}

/// The catalogue row named `name`.
pub fn tool_spec(name: &str) -> Option<&'static ToolSpec> {
    tool_catalogue().iter().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests;
