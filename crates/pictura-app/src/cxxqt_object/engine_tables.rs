//! Free-function bridges that serve the engine's tables and colour conversions
//! to the shell, so it keeps no copy of its own: blend modes, colour models,
//! the adjustment catalogue, the tool catalogue and the command catalogue.

pub(crate) mod adjustment_catalogue;
pub(crate) mod blend_modes;
pub(crate) mod color_models;
pub(crate) mod tool_catalogue;

use self::adjustment_catalogue::adjustment_catalogue;
use self::blend_modes::blend_mode_rows;
use self::color_models::{channel_plane, lab_to_srgb, srgb_to_lab};
use self::tool_catalogue::tool_catalogue_rows;
use super::command_catalogue::{command_catalogue, command_id_for_path};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[namespace = "pictura"]
    struct BlendModeRow {
        /// The 4-byte PSD key, e.g. `"mul "`.
        key: QString,
        name: QString,
        /// Whether the mode has a neutral colour (New Layer's "Fill with
        /// neutral color" option).
        has_neutral: bool,
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

    /// CIELAB D50: `l` in 0–100, `a`/`b` unclamped.
    #[namespace = "pictura"]
    struct LabValue {
        l: f64,
        a: f64,
        b: f64,
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
        /// The 27 layer modes in engine order; Pass Through is excluded.
        fn blend_mode_rows() -> Vec<BlendModeRow>;

        /// Every adjustment and fill kind in CS6 order.
        fn adjustment_catalogue() -> Vec<AdjustmentRow>;

        /// Every tool in `ToolId` order.
        fn tool_catalogue_rows() -> Vec<ToolSpecRow>;

        /// Every catalogue row, in menu order.
        fn command_catalogue() -> Vec<CommandSpecRow>;

        /// The stable id derived from a command path.
        fn command_id_for_path(path: &QStringList) -> QString;

        /// 8-bit sRGB → CIELAB in the D50 connection space.
        fn srgb_to_lab(r: u8, g: u8, b: u8) -> LabValue;

        /// CIELAB D50 → 8-bit sRGB packed `0xRRGGBB`; out-of-gamut clamps.
        fn lab_to_srgb(l: f64, a: f64, b: f64) -> u32;

        /// The Channels panel row `row`'s grey value for each `0xAARRGGBB`
        /// pixel of a `mode` document (`mode` as `channels_mode` names it):
        /// the composite's luminance, an RGB plane, or a CMYK / Multichannel
        /// ink plate, the same split channel-to-selection loads.
        fn channel_plane(pixels: &[u32], mode: &QString, row: i32) -> Vec<u8>;
    }
}
