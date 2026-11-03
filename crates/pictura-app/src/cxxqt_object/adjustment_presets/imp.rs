use cxx_qt_lib::{QString, QStringList};
use pictura_core::AdjustmentData;

fn from_block(block: &[u8]) -> Option<AdjustmentData> {
    Some(AdjustmentData {
        key: block.get(..4)?.try_into().ok()?,
        data: block.get(4..)?.to_vec().into(),
    })
}

fn to_block(data: Option<AdjustmentData>) -> Vec<u8> {
    data.map(|d| {
        d.key
            .iter()
            .copied()
            .chain(d.data.iter().copied())
            .collect()
    })
    .unwrap_or_default()
}

pub(super) fn adjustment_preset_kind(block: &[u8]) -> QString {
    from_block(block)
        .as_ref()
        .and_then(pictura_render::preset_kind_of)
        .map(QString::from)
        .unwrap_or_default()
}

pub(super) fn adjustment_preset_saved_names(kind: &QString) -> QStringList {
    let mut names = QStringList::default();
    for name in pictura_render::saved_preset_names(&kind.to_string()) {
        names.append(QString::from(name.as_str()));
    }
    names
}

pub(super) fn adjustment_preset_saved_block(kind: &QString, name: &QString) -> Vec<u8> {
    to_block(pictura_render::saved_preset_block(
        &kind.to_string(),
        &name.to_string(),
    ))
}

pub(super) fn adjustment_preset_save(kind: &QString, name: &QString, block: &[u8]) -> bool {
    from_block(block).is_some_and(|block| {
        pictura_render::save_preset(&kind.to_string(), &name.to_string(), &block)
    })
}

pub(super) fn adjustment_preset_delete(kind: &QString, name: &QString) -> bool {
    pictura_render::delete_preset(&kind.to_string(), &name.to_string())
}

pub(super) fn adjustment_preset_can_save(kind: &QString) -> bool {
    pictura_render::preset_is_saveable(&kind.to_string())
}

pub(super) fn adjustment_preset_supports(kind: &QString) -> bool {
    pictura_render::preset_supports_menu(&kind.to_string())
}

pub(super) fn adjustment_preset_matches(a: &[u8], b: &[u8]) -> bool {
    from_block(a)
        .zip(from_block(b))
        .is_some_and(|(a, b)| pictura_render::preset_blocks_match(&a, &b))
}
