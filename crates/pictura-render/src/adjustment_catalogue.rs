//! The adjustment catalogue: every adjustment and fill-layer kind with its CS6
//! title, menu placement, default-block builders, direct-command constructor,
//! and GPU route. The shell builds its adjustment surfaces from these rows.

use pictura_codec::layer_content::defaults::{
    black_white_dialog, brightness_contrast_dialog, brightness_contrast_layer,
    channel_mixer_dialog, color_balance_dialog, color_lookup_dialog, curves_dialog,
    exposure_dialog, gradient_map_dialog, gradient_map_layer, hue_saturation_dialog,
    hue_saturation_layer, invert_layer, levels_dialog, photo_filter_dialog, photo_filter_layer,
    posterize_dialog, selective_color_dialog, shadows_highlights_dialog, threshold_dialog,
    vibrance_dialog, Defaults,
};
use pictura_core::{
    direct_auto_color, direct_auto_contrast, direct_auto_tone, direct_desaturate, direct_equalize,
    direct_invert, Adjustment, AdjustmentData, AdjustmentKind,
};

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

/// How the GPU compositor applies a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRoute {
    /// No GPU implementation; the layer composites on the CPU.
    None,
    /// An inline shader branch identified by its numeric kind id.
    Inline { id: u32 },
    /// A 256-entry lookup table built by the CPU adjustment code. Several kinds
    /// share the table path, so the route carries no id of its own.
    Lut,
}

#[derive(Debug, Clone, Copy)]
pub struct AdjustmentEntry {
    pub kind: AdjustmentKind,
    pub title: &'static str,
    pub image: ImageCommand,
    pub layer: LayerEntry,
    /// The Adjustments panel tile's icon id, when the kind has a tile.
    pub panel_icon: Option<&'static str>,
    /// The Adjustments panel flyout row's text, when the kind has a row.
    pub panel_menu: Option<&'static str>,
    /// A fill layer (Solid Color, Gradient, Pattern) rather than an adjustment.
    pub fill: bool,
    pub dialog_default: Option<fn(Defaults) -> AdjustmentData>,
    pub layer_default: Option<fn(Defaults) -> AdjustmentData>,
    pub direct: Option<fn() -> Adjustment>,
    /// How the GPU compositor applies it.
    pub gpu: GpuRoute,
}

const fn fill(kind: AdjustmentKind, title: &'static str, icon: &'static str) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image: ImageCommand::None,
        layer: LayerEntry::Ellipsis,
        panel_icon: Some(icon),
        panel_menu: None,
        fill: true,
        dialog_default: None,
        layer_default: None,
        direct: None,
        gpu: GpuRoute::None,
    }
}

/// A dialog adjustment that is also an adjustment layer, a panel tile and a
/// panel menu row.
const fn dialog(
    kind: AdjustmentKind,
    title: &'static str,
    icon: &'static str,
    menu: &'static str,
    dialog_default: fn(Defaults) -> AdjustmentData,
    layer_default: Option<fn(Defaults) -> AdjustmentData>,
    gpu: GpuRoute,
) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image: ImageCommand::AdjustmentsDialog,
        layer: LayerEntry::Ellipsis,
        panel_icon: Some(icon),
        panel_menu: Some(menu),
        fill: false,
        dialog_default: Some(dialog_default),
        layer_default,
        direct: None,
        gpu,
    }
}

/// The GPU route of a kind the shader applies inline.
pub const fn inline(id: u32) -> GpuRoute {
    GpuRoute::Inline { id }
}

const fn direct(
    kind: AdjustmentKind,
    title: &'static str,
    image: ImageCommand,
    direct: fn() -> Adjustment,
) -> AdjustmentEntry {
    AdjustmentEntry {
        kind,
        title,
        image,
        layer: LayerEntry::None,
        panel_icon: None,
        panel_menu: None,
        fill: false,
        dialog_default: None,
        layer_default: None,
        direct: Some(direct),
        gpu: GpuRoute::None,
    }
}

/// Every kind in CS6 order: the fills, then the adjustments in Adjustments
/// panel and New Adjustment Layer order (Color Lookup after Channel Mixer,
/// Invert between it and Posterize), then the dialog-only and direct commands.
pub const ADJUSTMENT_CATALOGUE: &[AdjustmentEntry] = &[
    fill(
        AdjustmentKind::SolidColor,
        "Solid Color",
        "adjustments.solid-color",
    ),
    fill(AdjustmentKind::Gradient, "Gradient", "adjustments.gradient"),
    fill(AdjustmentKind::Pattern, "Pattern", "adjustments.pattern"),
    dialog(
        AdjustmentKind::BrightnessContrast,
        "Brightness/Contrast",
        "adjustments.brightness-contrast",
        "Brightness-Contrast",
        brightness_contrast_dialog,
        Some(brightness_contrast_layer),
        inline(4),
    ),
    dialog(
        AdjustmentKind::Levels,
        "Levels",
        "adjustments.levels",
        "Levels",
        levels_dialog,
        Some(levels_dialog),
        GpuRoute::Lut,
    ),
    dialog(
        AdjustmentKind::Curves,
        "Curves",
        "adjustments.curves",
        "Curves",
        curves_dialog,
        Some(curves_dialog),
        GpuRoute::Lut,
    ),
    dialog(
        AdjustmentKind::Exposure,
        "Exposure",
        "adjustments.exposure",
        "Exposure",
        exposure_dialog,
        Some(exposure_dialog),
        GpuRoute::Lut,
    ),
    dialog(
        AdjustmentKind::Vibrance,
        "Vibrance",
        "adjustments.vibrance",
        "Vibrance",
        vibrance_dialog,
        Some(vibrance_dialog),
        GpuRoute::None,
    ),
    dialog(
        AdjustmentKind::HueSaturation,
        "Hue/Saturation",
        "adjustments.hue-saturation",
        "Hue-Saturation",
        hue_saturation_dialog,
        Some(hue_saturation_layer),
        inline(5),
    ),
    dialog(
        AdjustmentKind::ColorBalance,
        "Color Balance",
        "adjustments.color-balance",
        "Color Balance",
        color_balance_dialog,
        Some(color_balance_dialog),
        GpuRoute::None,
    ),
    dialog(
        AdjustmentKind::BlackWhite,
        "Black & White",
        "adjustments.black-white",
        "Black & White",
        black_white_dialog,
        Some(black_white_dialog),
        GpuRoute::None,
    ),
    dialog(
        AdjustmentKind::PhotoFilter,
        "Photo Filter",
        "adjustments.photo-filter",
        "Photo Filter",
        photo_filter_dialog,
        Some(photo_filter_layer),
        GpuRoute::None,
    ),
    dialog(
        AdjustmentKind::ChannelMixer,
        "Channel Mixer",
        "adjustments.channel-mixer",
        "Channel Mixer",
        channel_mixer_dialog,
        Some(channel_mixer_dialog),
        GpuRoute::None,
    ),
    dialog(
        AdjustmentKind::ColorLookup,
        "Color Lookup",
        "adjustments.color-lookup",
        "Color Lookup",
        color_lookup_dialog,
        Some(color_lookup_dialog),
        GpuRoute::None,
    ),
    AdjustmentEntry {
        kind: AdjustmentKind::Invert,
        title: "Invert",
        image: ImageCommand::AdjustmentsDirect,
        layer: LayerEntry::Plain,
        panel_icon: Some("adjustments.invert"),
        panel_menu: Some("Invert"),
        fill: false,
        dialog_default: None,
        layer_default: Some(invert_layer),
        direct: Some(direct_invert),
        gpu: inline(1),
    },
    dialog(
        AdjustmentKind::Posterize,
        "Posterize",
        "adjustments.posterize",
        "Posterize",
        posterize_dialog,
        Some(posterize_dialog),
        inline(2),
    ),
    dialog(
        AdjustmentKind::Threshold,
        "Threshold",
        "adjustments.threshold",
        "Threshold",
        threshold_dialog,
        Some(threshold_dialog),
        inline(3),
    ),
    dialog(
        AdjustmentKind::GradientMap,
        "Gradient Map",
        "adjustments.gradient-map",
        "Gradient Map",
        gradient_map_dialog,
        Some(gradient_map_layer),
        GpuRoute::Lut,
    ),
    dialog(
        AdjustmentKind::SelectiveColor,
        "Selective Color",
        "adjustments.selective-color",
        "Selective Color",
        selective_color_dialog,
        Some(selective_color_dialog),
        GpuRoute::None,
    ),
    AdjustmentEntry {
        kind: AdjustmentKind::ShadowsHighlights,
        title: "Shadows/Highlights",
        image: ImageCommand::AdjustmentsDialog,
        layer: LayerEntry::None,
        panel_icon: None,
        panel_menu: None,
        fill: false,
        dialog_default: Some(shadows_highlights_dialog),
        layer_default: None,
        direct: None,
        gpu: GpuRoute::None,
    },
    direct(
        AdjustmentKind::Desaturate,
        "Desaturate",
        ImageCommand::AdjustmentsDirect,
        direct_desaturate,
    ),
    direct(
        AdjustmentKind::Equalize,
        "Equalize",
        ImageCommand::AdjustmentsDirect,
        direct_equalize,
    ),
    direct(
        AdjustmentKind::AutoTone,
        "Auto Tone",
        ImageCommand::ImageRoot,
        direct_auto_tone,
    ),
    direct(
        AdjustmentKind::AutoContrast,
        "Auto Contrast",
        ImageCommand::ImageRoot,
        direct_auto_contrast,
    ),
    direct(
        AdjustmentKind::AutoColor,
        "Auto Color",
        ImageCommand::ImageRoot,
        direct_auto_color,
    ),
];

/// The catalogue entry for `kind`, by its id.
pub fn adjustment_entry(kind: &str) -> Option<&'static AdjustmentEntry> {
    ADJUSTMENT_CATALOGUE
        .iter()
        .find(|entry| entry.kind.name() == kind)
}

/// The catalogue entry for a typed kind.
pub fn entry_of(kind: AdjustmentKind) -> Option<&'static AdjustmentEntry> {
    ADJUSTMENT_CATALOGUE.iter().find(|entry| entry.kind == kind)
}

/// The adjustment a direct Image command applies.
pub fn direct_adjustment(kind: AdjustmentKind) -> Option<Adjustment> {
    entry_of(kind)?.direct.map(|build| build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjustment_editor;

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
                entry.kind.name()
            );
            assert_eq!(
                adjustment_entry(entry.kind.name()).map(|row| row.kind),
                Some(entry.kind)
            );
            assert_eq!(entry_of(entry.kind).map(|row| row.kind), Some(entry.kind));
        }
        let mut rows: Vec<AdjustmentKind> = ADJUSTMENT_CATALOGUE
            .iter()
            .map(|entry| entry.kind)
            .collect();
        let mut kinds = AdjustmentKind::ALL.to_vec();
        rows.sort_by_key(|kind| kind.name());
        kinds.sort_by_key(|kind| kind.name());
        assert_eq!(rows, kinds, "one row per kind, both directions");

        let dialogs: Vec<&str> = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|e| e.image == ImageCommand::AdjustmentsDialog)
            .map(|e| e.kind.name())
            .collect();
        assert_eq!(dialogs, DIALOG_ORDER);
        for entry in ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|entry| entry.image == ImageCommand::AdjustmentsDialog)
        {
            let block = (entry.dialog_default.expect("dialog row has a default"))(Defaults::rgb());
            let editor = adjustment_editor(&block)
                .unwrap_or_else(|| panic!("{} decodes", entry.kind.name()));
            assert_eq!(editor.title, entry.title);
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
        assert!(adjustment_entry("hdr-toning").is_none());
    }

    #[test]
    fn adjustment_catalogue_layer_entries() {
        let layers: Vec<&AdjustmentEntry> = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|e| !e.fill && e.layer != LayerEntry::None)
            .collect();
        let kinds: Vec<&str> = layers.iter().map(|e| e.kind.name()).collect();
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
                entry.kind == AdjustmentKind::Invert,
                "{}",
                entry.kind.name()
            );
            assert!(entry.panel_icon.is_some() && entry.panel_menu.is_some());
        }
        assert_eq!(
            adjustment_entry("shadows-highlights").unwrap().layer,
            LayerEntry::None
        );
    }
}
