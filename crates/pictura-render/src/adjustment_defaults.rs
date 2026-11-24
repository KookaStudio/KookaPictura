//! Convenience wrappers that resolve a typed kind through the catalogue row.
//! Existing render and app consumers keep these signatures.

use pictura_core::{AdjustmentData, AdjustmentKind};

pub use pictura_codec::layer_content::defaults::Defaults;

pub fn dialog_default(kind: AdjustmentKind, ctx: Defaults) -> Option<AdjustmentData> {
    crate::entry_of(kind)?
        .dialog_default
        .map(|build| build(ctx))
}

pub fn layer_default(kind: AdjustmentKind, ctx: Defaults) -> Option<AdjustmentData> {
    crate::entry_of(kind)?.layer_default.map(|build| build(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adjustment_editor, ADJUSTMENT_CATALOGUE};

    fn ctx() -> Defaults {
        Defaults::rgb()
    }

    #[test]
    fn adjustment_row_dialog_defaults() {
        for entry in ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|entry| entry.image == crate::ImageCommand::AdjustmentsDialog)
        {
            let kind = entry.kind;
            let block = dialog_default(kind, ctx())
                .unwrap_or_else(|| panic!("{} has a dialog default", kind.name()));
            let editor =
                adjustment_editor(&block).unwrap_or_else(|| panic!("{} decodes", kind.name()));
            assert_eq!(editor.title, entry.title, "{}", kind.name());
            for param in &editor.params {
                assert_eq!(
                    param.value,
                    param.default,
                    "{}.{} starts at its default",
                    kind.name(),
                    param.key
                );
            }
        }
    }

    #[test]
    fn adjustment_row_layer_defaults() {
        for entry in ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|entry| !entry.fill && entry.layer != crate::LayerEntry::None)
        {
            let kind = entry.kind;
            let block = layer_default(kind, ctx())
                .unwrap_or_else(|| panic!("{} has a layer default", kind.name()));
            let decoded = crate::decode_adjustment(&block)
                .unwrap_or_else(|| panic!("{} decodes", kind.name()));
            assert_eq!(decoded.kind(), Some(kind), "{}", kind.name());
            let editor = adjustment_editor(&block)
                .unwrap_or_else(|| panic!("{} opens an editor", kind.name()));
            assert_eq!(editor.title, entry.title, "{}", kind.name());
        }
        assert!(layer_default(AdjustmentKind::ShadowsHighlights, ctx()).is_none());
        assert!(layer_default(AdjustmentKind::SolidColor, ctx()).is_none());
    }

    /// The kinds whose new-layer default deliberately differs from the dialog
    /// default, compared at the default black/white pair. Gradient Map is not in
    /// the list: at that pair its two defaults agree, and it differs only when
    /// the foreground/background are not black/white.
    #[test]
    fn layer_defaults_differ_deliberately() {
        let mut differing = Vec::new();
        for kind in AdjustmentKind::ALL.iter().copied() {
            let (Some(dialog), Some(layer)) =
                (dialog_default(kind, ctx()), layer_default(kind, ctx()))
            else {
                continue;
            };
            if dialog.data != layer.data {
                differing.push(kind);
            }
        }
        assert_eq!(
            differing,
            [
                AdjustmentKind::BrightnessContrast,
                AdjustmentKind::HueSaturation,
                AdjustmentKind::PhotoFilter,
            ]
        );

        let toolbar = Defaults::rgb().with_colors([10, 20, 30], [40, 50, 60]);
        assert_ne!(
            dialog_default(AdjustmentKind::GradientMap, toolbar)
                .unwrap()
                .data,
            layer_default(AdjustmentKind::GradientMap, toolbar)
                .unwrap()
                .data,
            "Gradient Map differs only off the default pair"
        );
        assert_eq!(
            dialog_default(AdjustmentKind::GradientMap, ctx())
                .unwrap()
                .data,
            layer_default(AdjustmentKind::GradientMap, ctx())
                .unwrap()
                .data,
            "and agrees at it"
        );
    }
}
