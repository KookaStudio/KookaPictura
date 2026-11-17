use super::*;

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::layer_comp::{CompRecord, LayerComp};

use crate::cxxqt_object::edit::Edit;

fn with_comp<R>(
    view: &PictureView,
    index: i32,
    f: impl FnOnce(&pictura_core::Document, &LayerComp) -> R,
) -> Option<R> {
    let doc = view.rust().session.doc.as_ref()?;
    let comp = doc.layer_comps.comps().get(usize::try_from(index).ok()?)?;
    Some(f(doc, comp))
}

pub(super) fn layer_comp_count(view: &PictureView) -> i32 {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(0, |doc| doc.layer_comps.len() as i32)
}

pub(super) fn layer_comp_id(view: &PictureView, index: i32) -> i32 {
    with_comp(view, index, |_, comp| comp.id as i32).unwrap_or(-1)
}

pub(super) fn layer_comp_name(view: &PictureView, index: i32) -> QString {
    with_comp(view, index, |_, comp| QString::from(comp.name.as_str())).unwrap_or_default()
}

pub(super) fn layer_comp_comment(view: &PictureView, index: i32) -> QString {
    with_comp(view, index, |_, comp| QString::from(comp.comment.as_str())).unwrap_or_default()
}

pub(super) fn layer_comp_record(view: &PictureView, index: i32) -> i32 {
    with_comp(view, index, |_, comp| comp.record.bits() as i32).unwrap_or(0)
}

pub(super) fn layer_comp_warning(view: &PictureView, index: i32) -> bool {
    with_comp(view, index, |doc, comp| {
        pictura_render::layer_comp_warning(doc, comp)
    })
    .unwrap_or(false)
}

pub(super) fn layer_comp_last_available(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.layer_comps.last_state().is_some())
}

pub(super) fn layer_comp_create(
    mut view: Pin<&mut PictureView>,
    name: &QString,
    comment: &QString,
    record_bits: i32,
) -> i32 {
    if view.rust().modal_transform_blocks_edit() {
        return -1;
    }
    let count = view
        .rust()
        .session
        .doc
        .as_ref()
        .map_or(0, |doc| doc.layer_comps.len());
    let name = name.to_string();
    let name = if name.trim().is_empty() {
        format!("Layer Comp {}", count + 1)
    } else {
        name
    };
    let comment = comment.to_string();
    let record = CompRecord::from_bits(record_bits.max(0) as u32);
    let mut id = -1;
    view.as_mut().edit("New Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let comp = pictura_render::capture_layer_comp(doc, 0, name, comment, record);
        id = doc.layer_comps.push_comp(comp) as i32;
        doc.layer_comps.mark_dirty();
        Edit::Meta
    });
    id
}

pub(super) fn layer_comp_apply(mut view: Pin<&mut PictureView>, id: i32) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let id = id as u32;
    let mut applied = false;
    view.as_mut().edit("Apply Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(comp) = doc.layer_comps.comp(id).cloned() else {
            return Edit::None;
        };
        let missing = pictura_render::layer_comp_missing_ids(doc, &comp);
        if comp
            .entries
            .iter()
            .all(|entry| missing.contains(&entry.layer_id))
            || !pictura_render::layer_comp_changes(doc, &comp)
        {
            return Edit::None;
        }
        let last = pictura_render::capture_last_document_state(doc);
        let outcome = pictura_render::apply_layer_comp(doc, &comp);
        if outcome.is_noop() {
            return Edit::None;
        }
        doc.layer_comps.set_last_state(last);
        doc.layer_comps.mark_base_dirty();
        applied = true;
        Edit::Full
    });
    applied
}

pub(super) fn layer_comp_update(mut view: Pin<&mut PictureView>, id: i32) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let id = id as u32;
    let mut updated = false;
    view.as_mut().edit("Update Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(old) = doc.layer_comps.comp(id).cloned() else {
            return Edit::None;
        };
        let fresh = pictura_render::capture_layer_comp(doc, id, old.name, old.comment, old.record);
        if let Some(comp) = doc.layer_comps.comp_mut(id) {
            comp.entries = fresh.entries;
            comp.warning_cleared.clear();
        }
        doc.layer_comps.mark_dirty();
        updated = true;
        Edit::Meta
    });
    updated
}

pub(super) fn layer_comp_delete(mut view: Pin<&mut PictureView>, id: i32) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let id = id as u32;
    let mut deleted = false;
    view.as_mut().edit("Delete Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        deleted = doc.layer_comps.remove_comp(id);
        if deleted {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    deleted
}

pub(super) fn layer_comp_rename(mut view: Pin<&mut PictureView>, id: i32, name: &QString) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let name = name.to_string();
    if name.trim().is_empty() {
        return false;
    }
    let id = id as u32;
    let mut renamed = false;
    view.as_mut().edit("Rename Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        renamed = doc.layer_comps.rename_comp(id, name);
        if renamed {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    renamed
}

pub(super) fn layer_comp_set_options(
    mut view: Pin<&mut PictureView>,
    id: i32,
    name: &QString,
    comment: &QString,
    record_bits: i32,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let name = name.to_string();
    if name.trim().is_empty() {
        return false;
    }
    let comment = comment.to_string();
    let record = CompRecord::from_bits(record_bits.max(0) as u32);
    let id = id as u32;
    let mut changed = false;
    view.as_mut().edit("Layer Comp Options", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(comp) = doc.layer_comps.comp(id).cloned() else {
            return Edit::None;
        };
        let mut did = false;
        if comp.name != name {
            did |= doc.layer_comps.rename_comp(id, name);
        }
        if comp.comment != comment {
            did |= doc.layer_comps.set_comment(id, comment);
        }
        if comp.record != record {
            did |= doc.layer_comps.set_record(id, record);
        }
        changed = did;
        if did {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    changed
}

pub(super) fn layer_comp_duplicate(mut view: Pin<&mut PictureView>, id: i32) -> i32 {
    if view.rust().modal_transform_blocks_edit() {
        return -1;
    }
    let id = id as u32;
    let mut created = -1;
    view.as_mut().edit("Duplicate Layer Comp", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        created = doc
            .layer_comps
            .duplicate_comp(id)
            .map_or(-1, |id| id as i32);
        if created >= 0 {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    created
}

pub(super) fn layer_comp_clear_warning(mut view: Pin<&mut PictureView>, id: i32) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let id = id as u32;
    let mut cleared = false;
    view.as_mut().edit("Clear Layer Comp Warning", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        cleared = pictura_render::clear_layer_comp_warning(doc, id);
        if cleared {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    cleared
}

pub(super) fn layer_comp_clear_all_warnings(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let mut cleared = false;
    view.as_mut().edit("Clear All Layer Comp Warnings", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        cleared = pictura_render::clear_all_layer_comp_warnings(doc);
        if cleared {
            Edit::Meta
        } else {
            Edit::None
        }
    });
    cleared
}

pub(super) fn layer_comp_restore_last(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let mut restored = false;
    view.as_mut().edit("Restore Last Document State", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(state) = doc.layer_comps.last_state().cloned() else {
            return Edit::None;
        };
        let outcome = pictura_render::restore_last_document_state(doc, &state);
        if outcome.is_noop() {
            return Edit::None;
        }
        doc.layer_comps.mark_base_dirty();
        restored = true;
        Edit::Full
    });
    restored
}

fn effective_icc(doc: &pictura_core::Document) -> Vec<u8> {
    if let Some(icc) = doc.document_icc.as_ref() {
        if pictura_codec::Profile::from_icc(icc).is_ok() {
            return icc.to_vec();
        }
    }
    if doc.mode == pictura_core::ColorMode::Grayscale {
        pictura_codec::Profile::gray_gamma22().to_icc()
    } else {
        pictura_codec::Profile::srgb().to_icc()
    }
}

fn sanitize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    out
}

fn fit_stem(stem: &str) -> String {
    const MAX: usize = 230;
    if stem.len() <= MAX {
        return stem.to_string();
    }
    let mut end = MAX;
    while end > 0 && !stem.is_char_boundary(end) {
        end -= 1;
    }
    stem[..end].to_string()
}

fn fit_stem_with_suffix(base: &str, suffix: &str) -> String {
    const MAX: usize = 230;
    let keep = MAX.saturating_sub(suffix.len());
    let mut end = keep.min(base.len());
    while end > 0 && !base.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &base[..end], suffix)
}

pub(super) fn layer_comp_export_targets(view: &PictureView, prefix: &QString) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    let prefix = sanitize(&prefix.to_string());
    let mut used: Vec<String> = Vec::new();
    let mut out = String::new();
    for comp in doc.layer_comps.comps() {
        let name = sanitize(&comp.name);
        let name = if name.is_empty() {
            "Layer Comp".to_string()
        } else {
            name
        };
        let mut stem = fit_stem(&format!("{prefix}{name}"));
        if used.iter().any(|used| used.eq_ignore_ascii_case(&stem)) {
            for n in 2u32..10000 {
                let suffix = format!(" {n}");
                let candidate = fit_stem_with_suffix(&format!("{prefix}{name}"), &suffix);
                if !used
                    .iter()
                    .any(|used| used.eq_ignore_ascii_case(&candidate))
                {
                    stem = candidate;
                    break;
                }
            }
        }
        used.push(stem.clone());
        out.push_str(&comp.id.to_string());
        out.push('\t');
        out.push_str(&stem);
        out.push('\n');
    }
    QString::from(out.as_str())
}

pub(super) fn layer_comp_export_file(
    view: &PictureView,
    id: i32,
    format: &QString,
    include_icc: bool,
    path: &QString,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let Some(comp) = doc.layer_comps.comp(id as u32).cloned() else {
        return false;
    };
    let mut scratch = doc.clone();
    pictura_render::apply_layer_comp(&mut scratch, &comp);
    scratch.composite = pictura_render::composite(pictura_render::CompositeRequest::whole(
        &scratch,
        rust.gpu_compute,
    ))
    .0;
    scratch.merged_composite_present = true;
    crate::cxxqt_object::export_tools::write_document_file(
        &mut scratch,
        &format.to_string(),
        include_icc,
        &path.to_string(),
    )
}

pub(super) fn layer_comp_export_pdf(view: &PictureView, path: &QString) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    if doc.layer_comps.is_empty() {
        return false;
    }
    let mut rendered: Vec<pictura_core::Document> = Vec::new();
    for comp in doc.layer_comps.comps() {
        let mut scratch = doc.clone();
        pictura_render::apply_layer_comp(&mut scratch, comp);
        scratch.composite = pictura_render::composite(pictura_render::CompositeRequest::whole(
            &scratch,
            rust.gpu_compute,
        ))
        .0;
        scratch.merged_composite_present = true;
        let icc = effective_icc(&scratch);
        pictura_codec::embed_document_icc(&mut scratch, &icc);
        rendered.push(scratch);
    }
    let pages: Vec<(&pictura_core::Document, &pictura_core::PixelBuffer)> =
        rendered.iter().map(|doc| (doc, &doc.composite)).collect();
    let mut options = pictura_codec::pdf::PdfOptions {
        preserve_editing: false,
        layers: false,
        icc_profile: true,
        ..Default::default()
    };
    options.normalize();
    let jpeg = |request: &pictura_codec::pdf::JpegRequest| {
        let bytes = crate::cxxqt_object::pdf_export::ffi::encode_jpeg(
            request.samples,
            request.width as i32,
            request.height as i32,
            i32::from(request.components),
            i32::from(request.quality),
        );
        (!bytes.is_empty()).then_some(bytes)
    };
    let path = path.to_string();
    let env = pictura_codec::pdf::PdfEnv {
        jpeg: &jpeg,
        now: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default(),
        entropy: pictura_codec::os_random(),
        title: std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        live_text: None,
        proof: None,
        editing_doc: None,
    };
    let Ok(bytes) = pictura_codec::pdf::write_pdf_pages(&pages, &options, &env) else {
        return false;
    };
    crate::cxxqt_object::helpers::safe_io::write_atomically(&path, &bytes, false).is_ok()
}
