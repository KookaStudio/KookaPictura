//! The adjustment catalogue bridge: the shell wires Image > Adjustments, New
//! Fill/Adjustment Layer, the Layers panel menu and the Adjustments panel from
//! `pictura_render::ADJUSTMENT_CATALOGUE` instead of its own kind tables.

use self::ffi::AdjustmentRow;
use cxx_qt_lib::QString;
use pictura_render::{AdjustmentEntry, ImageCommand, LayerEntry, ADJUSTMENT_CATALOGUE};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    struct AdjustmentRow {
        kind: QString,
        title: QString,
        /// `"dialog"` (Image > Adjustments, opens a dialog), `"direct"`
        /// (Image > Adjustments, applied at once), `"root"` (Image menu root,
        /// applied at once), or empty.
        image: QString,
        /// Offered by New Adjustment Layer / New Fill Layer.
        layer: bool,
        /// The layer leaf carries an ellipsis.
        layer_ellipsis: bool,
        /// The Adjustments panel tile icon id; empty without a tile.
        panel_icon: QString,
        /// The Adjustments panel flyout row text; empty without a row.
        panel_menu: QString,
        fill: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Every adjustment and fill kind in CS6 order.
        fn adjustment_catalogue() -> Vec<AdjustmentRow>;
    }
}

fn image_name(image: ImageCommand) -> &'static str {
    match image {
        ImageCommand::AdjustmentsDialog => "dialog",
        ImageCommand::AdjustmentsDirect => "direct",
        ImageCommand::ImageRoot => "root",
        ImageCommand::None => "",
    }
}

fn row(entry: &AdjustmentEntry) -> AdjustmentRow {
    AdjustmentRow {
        kind: QString::from(entry.kind),
        title: QString::from(entry.title),
        image: QString::from(image_name(entry.image)),
        layer: entry.layer != LayerEntry::None,
        layer_ellipsis: entry.layer == LayerEntry::Ellipsis,
        panel_icon: QString::from(entry.panel_icon.unwrap_or("")),
        panel_menu: QString::from(entry.panel_menu.unwrap_or("")),
        fill: entry.fill,
    }
}

fn adjustment_catalogue() -> Vec<AdjustmentRow> {
    ADJUSTMENT_CATALOGUE.iter().map(row).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_script::command_catalogue::command_catalogue;

    fn has_path(path: &[&str]) -> bool {
        command_catalogue().iter().any(|spec| spec.path == path)
    }

    /// Each catalogue row names a command the command catalogue has, so a title
    /// drift fails here instead of silently leaving a menu entry unwired.
    #[test]
    fn adjustment_catalogue_rows_resolve_commands() {
        for entry in ADJUSTMENT_CATALOGUE {
            match entry.image {
                ImageCommand::AdjustmentsDialog | ImageCommand::AdjustmentsDirect => assert!(
                    has_path(&["Image", "Adjustments", entry.title]),
                    "Image > Adjustments > {}",
                    entry.title
                ),
                ImageCommand::ImageRoot => {
                    assert!(has_path(&["Image", entry.title]), "Image > {}", entry.title)
                }
                ImageCommand::None => {}
            }
            if entry.fill {
                let leaf = format!("{}…", entry.title);
                assert!(
                    has_path(&["Layer", "New Fill Layer", &leaf]),
                    "New Fill Layer > {leaf}"
                );
            } else if entry.layer != LayerEntry::None {
                assert!(
                    has_path(&["Layer", "New Adjustment Layer", entry.title]),
                    "New Adjustment Layer > {}",
                    entry.title
                );
            }
        }
        let rows = adjustment_catalogue();
        assert_eq!(rows.len(), ADJUSTMENT_CATALOGUE.len());
        let tone = rows
            .iter()
            .find(|row| row.kind.to_string() == "auto-tone")
            .unwrap();
        assert_eq!(tone.image.to_string(), "root");
    }
}
