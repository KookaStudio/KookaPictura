//! The catalogue's defaults, direct commands, and GPU routes against their
//! consumers.

use crate::{
    adjustment_editor, decode_adjustment, Defaults, GpuRoute, ImageCommand, LayerEntry,
    ADJUSTMENT_CATALOGUE,
};

/// The body of the shader's `fn adjust(` … `}` — the adjustment dispatch alone,
/// so a `case 2u:` in the blend switch cannot be mistaken for an adjustment
/// branch.
fn adjustment_dispatch() -> &'static str {
    let source = super::shader::SHADER;
    let start = source
        .find("fn adjust(")
        .expect("the shader carries an adjustment dispatch");
    let body = &source[start..];
    let end = body
        .find("@compute")
        .expect("the dispatch ends before cs_main");
    &body[..end]
}

fn adjustment_parity_body() -> &'static str {
    let source = include_str!("../../tests/gpu_parity/layers.rs");
    let start = source
        .find("fn adjustment_layers_match_cpu()")
        .expect("the GPU parity suite has an adjustment case");
    let body = &source[start..];
    let end = body
        .find("\n}\n")
        .expect("the adjustment parity case has a closing brace");
    &body[..end]
}

fn compact(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

fn assert_wgsl_id_once(dispatch: &str, id: u32, seen: &mut Vec<u32>) {
    assert!(!seen.contains(&id), "GPU id {id} is declared twice");
    seen.push(id);
    let marker = format!("kind == {id}u");
    assert_eq!(
        dispatch.matches(&marker).count(),
        1,
        "GPU id {id} must identify exactly one shader branch"
    );
}

#[test]
fn catalogue_rows_keep_every_fact_in_sync() {
    let dispatch = adjustment_dispatch();
    let parity = compact(adjustment_parity_body());
    assert!(parity.contains(&compact(
        "for entry in pictura_render::ADJUSTMENT_CATALOGUE"
    )));
    assert!(parity.contains(&compact(
        "let expected = pictura_render::ADJUSTMENT_CATALOGUE"
    )));
    assert!(parity.contains(&compact("entry.layer_default.expect(")));
    assert!(parity.contains(&compact("assert_eq!(checked, expected")));

    let mut seen_ids = Vec::new();
    let per_channel = pictura_adjust::AdjustmentLut::PerChannel([[0; 256]; 3]);
    let luma = pictura_adjust::AdjustmentLut::Luma([[0; 4]; 256]);
    for id in [super::lut::kind(&per_channel), super::lut::kind(&luma)] {
        assert_wgsl_id_once(dispatch, id, &mut seen_ids);
    }

    for entry in ADJUSTMENT_CATALOGUE {
        let kind = entry.kind;
        assert_eq!(
            entry.fill,
            kind.is_fill(),
            "{} fill classification",
            kind.name()
        );
        let opens_dialog = entry.image == ImageCommand::AdjustmentsDialog;
        assert_eq!(
            opens_dialog,
            entry.dialog_default.is_some(),
            "{} declares exactly its dialog default",
            kind.name()
        );
        if let Some(build) = entry.dialog_default {
            let block = build(Defaults::rgb());
            let decoded = decode_adjustment(&block)
                .unwrap_or_else(|| panic!("{} dialog default decodes", kind.name()));
            assert_eq!(decoded.kind(), Some(kind), "{} dialog kind", kind.name());
            let editor = adjustment_editor(&block)
                .unwrap_or_else(|| panic!("{} dialog default opens", kind.name()));
            assert_eq!(editor.title, entry.title, "{} dialog title", kind.name());
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

        let creates_adjustment_layer = !entry.fill && entry.layer != LayerEntry::None;
        assert_eq!(
            creates_adjustment_layer,
            entry.layer_default.is_some(),
            "{} declares exactly its layer default",
            kind.name()
        );
        if let Some(build) = entry.layer_default {
            let block = build(Defaults::rgb());
            let decoded = decode_adjustment(&block)
                .unwrap_or_else(|| panic!("{} layer default decodes", kind.name()));
            assert_eq!(decoded.kind(), Some(kind), "{} layer kind", kind.name());
            let editor = adjustment_editor(&block)
                .unwrap_or_else(|| panic!("{} layer default opens", kind.name()));
            assert_eq!(editor.title, entry.title, "{} layer title", kind.name());
            assert_eq!(
                super::dispatches_on_gpu(&decoded),
                entry.gpu != GpuRoute::None,
                "{} layer default follows its GPU route",
                kind.name()
            );
        }

        let is_direct = matches!(
            entry.image,
            ImageCommand::AdjustmentsDirect | ImageCommand::ImageRoot
        );
        assert_eq!(
            is_direct,
            entry.direct.is_some(),
            "{} declares exactly its direct command",
            kind.name()
        );
        if let Some(build) = entry.direct {
            let adjustment = build();
            assert_eq!(
                adjustment.kind(),
                Some(kind),
                "{} direct command yields its own kind",
                kind.name()
            );
        }

        match entry.gpu {
            GpuRoute::None => {}
            GpuRoute::Inline { id } => {
                assert_wgsl_id_once(dispatch, id, &mut seen_ids);
            }
            GpuRoute::Lut => {
                let build = entry
                    .layer_default
                    .expect("every LUT-routed kind has a layer default");
                let decoded = decode_adjustment(&build(Defaults::rgb()))
                    .expect("a LUT-routed layer default decodes");
                assert!(
                    pictura_adjust::adjustment_lut(&decoded).is_some(),
                    "{} builds its declared LUT route",
                    kind.name()
                );
            }
        }
    }
    assert_eq!(seen_ids.len(), 7, "five inline ids and two LUT shape ids");
}
