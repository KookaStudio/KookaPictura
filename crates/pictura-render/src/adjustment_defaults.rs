pub use pictura_codec::layer_content::defaults::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adjustment_editor, ImageCommand, ADJUSTMENT_CATALOGUE};

    #[test]
    fn every_dialog_opens_on_its_defaults() {
        let dialogs = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|entry| entry.image == ImageCommand::AdjustmentsDialog);
        for entry in dialogs {
            let kind = entry.kind;
            let block = default_adjustment_block(kind, [0, 0, 0], [255, 255, 255])
                .unwrap_or_else(|| panic!("{kind} has no default"));
            let editor = adjustment_editor(&block).unwrap_or_else(|| panic!("{kind} decodes"));
            assert_eq!(editor.title, entry.title);
            for param in &editor.params {
                assert_eq!(
                    param.value, param.default,
                    "{kind}.{} starts at its default",
                    param.key
                );
            }
        }
        assert!(default_adjustment_block("match-color", [0; 3], [0; 3]).is_none());
    }
}
