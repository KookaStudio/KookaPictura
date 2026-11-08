//! The single declarative command table. The C++ shell builds its menus from
//! this data and a generated header carries the frozen identifiers.

mod const_names;
mod entries;

/// One command table row: a menu command (or a path-derived stub).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub id: &'static str,
    /// Top-level menu first, leaf last.
    pub path: &'static [&'static str],
    pub label: &'static str,
    pub shortcut: &'static str,
    pub implemented: bool,
    pub checkable: bool,
}

/// Every catalogue row, in menu order.
pub fn command_catalogue() -> &'static [CommandSpec] {
    entries::ROWS
}

/// The frozen C++ identifier name and value, in `commands.h` order.
pub fn const_names() -> &'static [(&'static str, &'static str)] {
    const_names::CONST_NAMES
}

/// The row for `id`, if the catalogue has it.
pub fn find(id: &str) -> Option<&'static CommandSpec> {
    command_catalogue().iter().find(|spec| spec.id == id)
}

/// The stable id derived from a command path, matching the C++ helper:
/// letters and digits lowercased, spaces and hyphens become `.`, everything
/// else dropped.
pub fn command_id_for_path(path: &[&str]) -> String {
    let mut id = String::new();
    for segment in path {
        for c in segment.chars() {
            if c.is_alphanumeric() {
                id.extend(c.to_lowercase());
            } else if c == ' ' || c == '-' {
                id.push('.');
            }
        }
        id.push('.');
    }
    id.pop();
    id
}

/// The id for `path` when the catalogue owns it. A path that derives an
/// unknown id is rejected rather than silently registering a handler for a
/// command no surface can reach.
pub fn id_for_path(path: &[&str]) -> Option<String> {
    let id = command_id_for_path(path);
    find(&id).map(|_| id)
}

/// Commands whose handler opens a modal dialog or reaches the unsaved prompt.
/// The control server refuses these before running the handler. Parameterised
/// filter dialogs are derived from the filter table at the call site.
pub fn modal_ids() -> &'static [&'static str] {
    &[
        "file.new",
        "file.open",
        "file.openAs",
        "file.openAsSmartObject",
        "file.place",
        "file.fileInfo",
        "file.save",
        "file.saveAs",
        "file.exportAs",
        "file.quickExportPng",
        "file.saveForWeb",
        "file.print",
        "file.print.one.copy",
        "file.revert",
        "file.close",
        "file.closeAll",
        "file.exit",
        "edit.fill",
        "edit.stroke",
        "edit.auto.blend.layers",
        "edit.keyboardShortcuts",
        "edit.pdfPresets",
        "edit.assignProfile",
        "edit.convertProfile",
        "edit.colorSettings",
        "edit.preferences.general",
        "edit.preferences.interface",
        "edit.preferences.performance",
        "edit.preferences.unitsRulers",
        "edit.preferences.guidesGridSlices",
        "edit.preferences.type",
        "edit.preferences.agentControl",
        "image.apply.image",
        "image.image.size",
        "image.canvas.size",
        "image.rotate.arbitrary",
        "image.trim",
        "image.mode.bitmap",
        "image.mode.grayscale",
        "image.mode.indexed.color",
        "image.mode.duotone",
        "image.mode.multichannel",
        "image.mode.color.table",
        "image.adjustments.brightnesscontrast",
        "image.adjustments.levels",
        "image.adjustments.curves",
        "image.adjustments.exposure",
        "image.adjustments.vibrance",
        "image.adjustments.huesaturation",
        "image.adjustments.color.balance",
        "image.adjustments.black..white",
        "image.adjustments.photo.filter",
        "image.adjustments.channel.mixer",
        "image.adjustments.color.lookup",
        "image.adjustments.posterize",
        "image.adjustments.threshold",
        "image.adjustments.gradient.map",
        "image.adjustments.selective.color",
        "image.adjustments.shadowshighlights",
        "image.adjustments.hdr.toning",
        "image.adjustments.match.color",
        "image.adjustments.replace.color",
        "select.color.range",
        "select.refine.edge",
        "select.modify.border",
        "select.modify.smooth",
        "select.modify.expand",
        "select.modify.contract",
        "select.modify.feather",
        "select.save",
        "select.load",
        "type.warp.text",
        "type.replace.all.missing.fonts",
        "filter.filter.gallery",
        "filter.lens.correction",
        "filter.picturaRaw",
        "filter.digimarc.read.watermark",
        "view.newGuide",
        "view.screenMode.fullWithMenuBar",
        "view.screenMode.full",
        "layer.new.layer",
        "layer.new.group",
        "layer.new.groupFromLayers",
        "layer.new.fill.solidColor",
        "layer.new.fill.gradient",
        "layer.new.fill.layer.pattern",
        "layer.smartObject.replaceContents",
        "layer.smartObject.exportContents",
        "layer.layer.style.blending.options",
        "layer.layer.style.drop.shadow",
        "layer.layer.style.inner.shadow",
        "layer.layer.style.outer.glow",
        "layer.layer.style.inner.glow",
        "layer.layer.style.bevel..emboss",
        "layer.layer.style.satin",
        "layer.layer.style.color.overlay",
        "layer.layer.style.gradient.overlay",
        "layer.layer.style.pattern.overlay",
        "layer.layer.style.stroke",
        "layer.layer.style.global.light",
        "layer.layer.style.scale.effects",
        "layer.contentOptions",
        "panel.layers.panelOptions",
        "panel.layers.refineMask",
        "panel.layers.maskOptions",
        "panel.history.newSnapshot",
        "paragraph.justification",
        "panel.properties.savePreset",
        "panel.properties.loadPreset",
        "actions.newAction",
        "actions.newSet",
        "actions.options",
        "actions.playbackOptions",
        "actions.insertMenuItem",
        "actions.insertStop",
        "actions.insertConditional",
        "actions.insertPath",
        "actions.load",
        "actions.replace",
        "actions.save",
        "actions.clear",
        "actions.reset",
        "window.workspace.new",
        "window.workspace.delete",
        "window.workspace.keyboardShortcuts",
        "help.checkForUpdates",
        "help.system.info",
        "help.about",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_derived_id_matches_its_path() {
        let frozen: std::collections::HashSet<&str> =
            const_names().iter().map(|(_, value)| *value).collect();
        for spec in command_catalogue() {
            if spec.path.is_empty() {
                assert!(
                    spec.id.starts_with("panel.")
                        || spec.id.starts_with("actions.")
                        || spec.id.starts_with("character.")
                        || spec.id.starts_with("paragraph."),
                    "empty-path id is not a panel command: {}",
                    spec.id
                );
                continue;
            }
            if !frozen.contains(spec.id) {
                assert_eq!(
                    command_id_for_path(spec.path),
                    spec.id,
                    "path-derived id drifted for {:?}",
                    spec.path
                );
            }
        }
    }

    #[test]
    fn unknown_path_is_rejected() {
        assert!(id_for_path(&["File", "New…"]).is_some());
        assert!(id_for_path(&["File", "Not A Command"]).is_none());
    }

    #[test]
    fn modal_ids_exist_in_the_catalogue() {
        for id in modal_ids() {
            assert!(find(id).is_some(), "modal id is not in the catalogue: {id}");
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for spec in command_catalogue() {
            assert!(seen.insert(spec.id), "duplicate command id {}", spec.id);
        }
    }

    #[test]
    fn actions_panel_has_its_documented_default() {
        let spec = find("window.panels.actions").expect("actions panel row");
        assert_eq!(spec.shortcut, "F9");
    }
}
