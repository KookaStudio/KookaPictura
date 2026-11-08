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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_derived_id_matches_its_path() {
        let frozen: std::collections::HashSet<&str> =
            const_names().iter().map(|(_, value)| *value).collect();
        for spec in command_catalogue() {
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

    #[test]
    fn default_shortcuts_are_unique() {
        let mut owners = std::collections::HashMap::new();
        for spec in command_catalogue() {
            if spec.shortcut.is_empty() {
                continue;
            }
            if let Some(previous) = owners.insert(spec.shortcut, spec.id) {
                panic!(
                    "{} is the default of both {} and {}",
                    spec.shortcut, previous, spec.id
                );
            }
        }
    }

    #[test]
    fn cs6_function_key_panels() {
        let expected = [
            ("window.panels.brushes", "F5"),
            ("window.panels.color", "F6"),
            ("window.panels.layers", "F7"),
            ("window.panels.info", "F8"),
            ("window.panels.actions", "F9"),
        ];
        for (id, shortcut) in expected {
            let spec = find(id).unwrap_or_else(|| panic!("{id} row"));
            assert_eq!(spec.shortcut, shortcut, "{id} default");
        }
    }

    #[test]
    fn proof_colors_owns_ctrl_y() {
        assert_eq!(
            find("view.proof.colors")
                .expect("proof colors row")
                .shortcut,
            "Ctrl+Y"
        );
        assert_eq!(find("edit.redo").expect("redo row").shortcut, "");
    }

    #[test]
    fn stamp_visible_owns_ctrl_alt_shift_e() {
        assert_eq!(
            find("layer.stamp.visible")
                .expect("stamp visible row")
                .shortcut,
            "Ctrl+Alt+Shift+E"
        );
        assert_eq!(find("file.exportAs").expect("export as row").shortcut, "");
    }
}
