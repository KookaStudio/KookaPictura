//! The adjustment catalogue bridge: the shell wires Image > Adjustments, New
//! Fill/Adjustment Layer, the Layers panel menu and the Adjustments panel from
//! `pictura_render::ADJUSTMENT_CATALOGUE` instead of its own kind tables.

mod imp;

use imp::*;

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
