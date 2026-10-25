//! The adjustment catalogue: every adjustment and fill-layer kind with its CS6
//! title and the surfaces it appears on (Image menu, New Fill/Adjustment Layer,
//! Adjustments panel). The shell builds those surfaces from this table; the
//! command catalogue owns the menu structure, this table owns the kinds.

/// Where a kind sits in the Image menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageCommand {
    /// Image > Adjustments > *title*…, opening the adjustment's dialog.
    AdjustmentsDialog,
    /// Image > Adjustments > *title*, applied at once.
    AdjustmentsDirect,
    /// Image > *title* at the menu root, applied at once (the Auto commands).
    ImageRoot,
    None,
}

/// How Layer > New Adjustment Layer (or New Fill Layer) offers a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerEntry {
    /// A leaf whose label carries an ellipsis (it opens Properties).
    Ellipsis,
    /// A leaf applied as is (Invert).
    Plain,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustmentEntry {
    pub kind: &'static str,
    pub title: &'static str,
    pub image: ImageCommand,
    pub layer: LayerEntry,
    /// The Adjustments panel tile's icon id, when the kind has a tile.
    pub panel_icon: Option<&'static str>,
    /// The Adjustments panel flyout row's text, when the kind has a row.
    pub panel_menu: Option<&'static str>,
    /// A fill layer (Solid Color, Gradient, Pattern) rather than an adjustment.
    pub fill: bool,
}

const fn fill(kind: &'static str, title: &'static str, icon: &'static str) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image: ImageCommand::None,
        layer: LayerEntry::Ellipsis,
        panel_icon: Some(icon),
        panel_menu: None,
        fill: true,
    }
}

/// A dialog adjustment that is also an adjustment layer, a panel tile and a
/// panel menu row.
const fn dialog(
    kind: &'static str,
    title: &'static str,
    icon: &'static str,
    menu: &'static str,
) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image: ImageCommand::AdjustmentsDialog,
        layer: LayerEntry::Ellipsis,
        panel_icon: Some(icon),
        panel_menu: Some(menu),
        fill: false,
    }
}

const fn direct(kind: &'static str, title: &'static str, image: ImageCommand) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image,
        layer: LayerEntry::None,
        panel_icon: None,
        panel_menu: None,
        fill: false,
    }
}

/// Every kind in CS6 order: the fills, then the adjustments in Adjustments
/// panel and New Adjustment Layer order (Color Lookup after Channel Mixer,
/// Invert between it and Posterize), then the dialog-only and direct commands.
pub const ADJUSTMENT_CATALOGUE: [AdjustmentEntry; 25] = [
    fill("solid-color", "Solid Color", "adjustments.solid-color"),
    fill("gradient", "Gradient", "adjustments.gradient"),
    fill("pattern", "Pattern", "adjustments.pattern"),
    dialog(
        "brightness-contrast",
        "Brightness/Contrast",
        "adjustments.brightness-contrast",
        "Brightness-Contrast",
    ),
    dialog("levels", "Levels", "adjustments.levels", "Levels"),
    dialog("curves", "Curves", "adjustments.curves", "Curves"),
    dialog("exposure", "Exposure", "adjustments.exposure", "Exposure"),
    dialog("vibrance", "Vibrance", "adjustments.vibrance", "Vibrance"),
    dialog(
        "hue-saturation",
        "Hue/Saturation",
        "adjustments.hue-saturation",
        "Hue-Saturation",
    ),
    dialog(
        "color-balance",
        "Color Balance",
        "adjustments.color-balance",
        "Color Balance",
    ),
    dialog(
        "black-white",
        "Black & White",
        "adjustments.black-white",
        "Black & White",
    ),
    dialog(
        "photo-filter",
        "Photo Filter",
        "adjustments.photo-filter",
        "Photo Filter",
    ),
    dialog(
        "channel-mixer",
        "Channel Mixer",
        "adjustments.channel-mixer",
        "Channel Mixer",
    ),
    dialog(
        "color-lookup",
        "Color Lookup",
        "adjustments.color-lookup",
        "Color Lookup",
    ),
    AdjustmentEntry {
        kind: "invert",
        title: "Invert",
        image: ImageCommand::AdjustmentsDirect,
        layer: LayerEntry::Plain,
        panel_icon: Some("adjustments.invert"),
        panel_menu: Some("Invert"),
        fill: false,
    },
    dialog(
        "posterize",
        "Posterize",
        "adjustments.posterize",
        "Posterize",
    ),
    dialog(
        "threshold",
        "Threshold",
        "adjustments.threshold",
        "Threshold",
    ),
    dialog(
        "gradient-map",
        "Gradient Map",
        "adjustments.gradient-map",
        "Gradient Map",
    ),
    dialog(
        "selective-color",
        "Selective Color",
        "adjustments.selective-color",
        "Selective Color",
    ),
    AdjustmentEntry {
        kind: "shadows-highlights",
        title: "Shadows/Highlights",
        image: ImageCommand::AdjustmentsDialog,
        layer: LayerEntry::None,
        panel_icon: None,
        panel_menu: None,
        fill: false,
    },
    direct("desaturate", "Desaturate", ImageCommand::AdjustmentsDirect),
    direct("equalize", "Equalize", ImageCommand::AdjustmentsDirect),
    direct("auto-tone", "Auto Tone", ImageCommand::ImageRoot),
    direct("auto-contrast", "Auto Contrast", ImageCommand::ImageRoot),
    direct("auto-color", "Auto Color", ImageCommand::ImageRoot),
];

/// The catalogue entry for `kind`.
pub fn adjustment_entry(kind: &str) -> Option<&'static AdjustmentEntry> {
    ADJUSTMENT_CATALOGUE.iter().find(|entry| entry.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adjustment_editor, default_adjustment_block};

    /// Image > Adjustments' dialog entries in CS6 menu order.
    const DIALOG_ORDER: [&str; 16] = [
        "brightness-contrast",
        "levels",
        "curves",
        "exposure",
        "vibrance",
        "hue-saturation",
        "color-balance",
        "black-white",
        "photo-filter",
        "channel-mixer",
        "color-lookup",
        "posterize",
        "threshold",
        "gradient-map",
        "selective-color",
        "shadows-highlights",
    ];

    #[test]
    fn adjustment_catalogue_covers_every_kind_once() {
        for (i, entry) in ADJUSTMENT_CATALOGUE.iter().enumerate() {
            assert!(
                ADJUSTMENT_CATALOGUE[i + 1..]
                    .iter()
                    .all(|e| e.kind != entry.kind),
                "{} appears twice",
                entry.kind
            );
            assert_eq!(adjustment_entry(entry.kind), Some(entry));
        }
        let dialogs: Vec<&str> = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|e| e.image == ImageCommand::AdjustmentsDialog)
            .map(|e| e.kind)
            .collect();
        assert_eq!(dialogs, DIALOG_ORDER);
        for kind in dialogs {
            let block = default_adjustment_block(kind, [0, 0, 0], [255, 255, 255])
                .unwrap_or_else(|| panic!("{kind} has no default"));
            let editor = adjustment_editor(&block).unwrap_or_else(|| panic!("{kind} decodes"));
            assert_eq!(editor.title, adjustment_entry(kind).unwrap().title);
        }
        let direct = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|e| {
                matches!(
                    e.image,
                    ImageCommand::AdjustmentsDirect | ImageCommand::ImageRoot
                )
            })
            .count();
        assert_eq!(direct, 6);
        assert_eq!(ADJUSTMENT_CATALOGUE.iter().filter(|e| e.fill).count(), 3);
        assert_eq!(adjustment_entry("hdr-toning"), None);
    }

    #[test]
    fn adjustment_catalogue_layer_entries() {
        let layers: Vec<&AdjustmentEntry> = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|e| !e.fill && e.layer != LayerEntry::None)
            .collect();
        let kinds: Vec<&str> = layers.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                "brightness-contrast",
                "levels",
                "curves",
                "exposure",
                "vibrance",
                "hue-saturation",
                "color-balance",
                "black-white",
                "photo-filter",
                "channel-mixer",
                "color-lookup",
                "invert",
                "posterize",
                "threshold",
                "gradient-map",
                "selective-color",
            ]
        );
        for entry in layers {
            assert_eq!(
                entry.layer == LayerEntry::Plain,
                entry.kind == "invert",
                "{}",
                entry.kind
            );
            assert!(entry.panel_icon.is_some() && entry.panel_menu.is_some());
        }
        assert_eq!(
            adjustment_entry("shadows-highlights").unwrap().layer,
            LayerEntry::None
        );
    }
}
