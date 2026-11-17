use super::super::qobject::PictureView;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::path::{NamedPath, VectorPath};
use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static EXPORT_DELAY_US: AtomicU64 = AtomicU64::new(0);

fn delay() {
    let micros = EXPORT_DELAY_US.load(Ordering::Relaxed);
    if micros > 0 {
        std::thread::sleep(Duration::from_micros(micros));
    }
}

pub fn export_tools_set_delay(_view: &PictureView, micros: i32) {
    EXPORT_DELAY_US.store(micros.max(0) as u64, Ordering::Relaxed);
}

fn ai_paths(doc: &Document) -> Vec<(String, VectorPath)> {
    let mut paths = Vec::new();
    if !doc.work_path.is_empty() {
        paths.push(("Work Path".to_string(), doc.work_path.clone()));
    }
    for NamedPath { name, path } in &doc.saved_paths {
        if !path.is_empty() {
            paths.push((name.clone(), path.clone()));
        }
    }
    paths
}

pub fn illustrator_paths_eps(view: &PictureView) -> Vec<u8> {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let owned = ai_paths(doc);
    let paths: Vec<pictura_codec::illustrator::AiPath<'_>> = owned
        .iter()
        .map(|(name, path)| pictura_codec::illustrator::AiPath {
            name: name.as_str(),
            path,
        })
        .collect();
    pictura_codec::illustrator::write_eps(doc.width, doc.height, &paths)
}

pub fn illustrator_paths_pdf(view: &PictureView) -> Vec<u8> {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let owned = ai_paths(doc);
    let paths: Vec<pictura_codec::illustrator::AiPath<'_>> = owned
        .iter()
        .map(|(name, path)| pictura_codec::illustrator::AiPath {
            name: name.as_str(),
            path,
        })
        .collect();
    pictura_codec::illustrator::write_pdf(doc.width, doc.height, &paths)
}

pub fn illustrator_has_paths(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| !ai_paths(doc).is_empty())
}

pub fn zoomify_plan_flat(base_width: i32, base_height: i32) -> Vec<i32> {
    pictura_codec::zoomify::plan(base_width.max(0) as u32, base_height.max(0) as u32).flat()
}

pub fn zoomify_group_for(ordinal: i32) -> i32 {
    pictura_codec::zoomify::group_for(ordinal.max(0) as u32) as i32
}

pub fn zoomify_properties(base_width: i32, base_height: i32) -> QString {
    if base_width <= 0 || base_height <= 0 {
        return QString::default();
    }
    QString::from(
        pictura_codec::zoomify::image_properties(&pictura_codec::zoomify::plan(
            base_width as u32,
            base_height as u32,
        ))
        .as_str(),
    )
}

pub fn zoomify_html(base_width: i32, base_height: i32) -> QString {
    if base_width <= 0 || base_height <= 0 {
        return QString::default();
    }
    QString::from(
        pictura_codec::zoomify::viewer_html(&pictura_codec::zoomify::plan(
            base_width as u32,
            base_height as u32,
        ))
        .as_str(),
    )
}

fn sanitize_name(raw: &str) -> String {
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
    if out.is_empty() {
        "Layer".to_string()
    } else {
        out
    }
}

fn fit_name(base: &str, suffix_len: usize) -> &str {
    // The five output suffixes are four bytes; the writers also append either
    // `.tmp-metadata` or `.<16 hex digits>.tmp`, so reserve the longer one.
    const FORMAT_SUFFIX_BYTES: usize = 4;
    const TEMP_SUFFIX_BYTES: usize = 21;
    let max = 255usize.saturating_sub(FORMAT_SUFFIX_BYTES + TEMP_SUFFIX_BYTES + suffix_len);
    if base.len() <= max {
        return base;
    }
    let mut end = max;
    while end > 0 && !base.is_char_boundary(end) {
        end -= 1;
    }
    &base[..end]
}

fn unique_name(base: &str, used: &[String]) -> String {
    let fits = fit_name(base, 0);
    if !used.iter().any(|name| name.eq_ignore_ascii_case(fits)) {
        return fits.to_string();
    }
    for n in 2u32.. {
        let suffix = format!(" {n}");
        let candidate = format!("{}{}", fit_name(base, suffix.len()), suffix);
        if !used
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    unreachable!("u32 name suffixes are exhausted only after billions of layers")
}

fn walk(
    layers: &[Layer],
    parent: &str,
    ancestors_visible: bool,
    visible_only: bool,
    prefix: &str,
    out: &mut Vec<(String, String)>,
) {
    for (index, layer) in layers.iter().enumerate() {
        let path = if parent.is_empty() {
            index.to_string()
        } else {
            format!("{parent}/{index}")
        };
        let visible = ancestors_visible && layer.visible;
        if layer.is_group {
            walk(&layer.children, &path, visible, visible_only, prefix, out);
        } else if !visible_only || visible {
            out.push((path, sanitize_name(&format!("{prefix}{}", layer.name))));
        }
    }
}

pub(crate) fn collect_targets(
    doc: &Document,
    visible_only: bool,
    prefix: &str,
) -> Vec<(String, String)> {
    let mut raw = Vec::new();
    walk(&doc.layers, "", true, visible_only, prefix, &mut raw);
    let mut used: Vec<String> = Vec::new();
    let mut targets = Vec::with_capacity(raw.len());
    for (path, base) in raw {
        let name = unique_name(&base, &used);
        used.push(name.clone());
        targets.push((path, name));
    }
    targets
}

pub fn export_layer_targets(view: &PictureView, visible_only: bool, prefix: &QString) -> QString {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QString::default();
    };
    let prefix = prefix.to_string();
    let mut out = String::new();
    for (path, name) in collect_targets(doc, visible_only, &prefix) {
        out.push_str(&path);
        out.push('\t');
        out.push_str(&name);
        out.push('\n');
    }
    QString::from(out.as_str())
}

pub(crate) fn scratch_document(source: &Document, layer: &Layer, gpu_compute: bool) -> Document {
    let mut scratch = Document::new(source.width, source.height, source.mode, BitDepth::Eight);
    scratch.global_light = source.global_light;
    scratch.layer_section_extra = source.layer_section_extra.clone();
    scratch.is_psb = source.is_psb;
    scratch.source_mode = source.source_mode;
    scratch.source_depth = None;
    scratch.source_palette = source.source_palette;
    scratch.document_icc = source.document_icc.clone();
    scratch.image_resources = source.image_resources.clone();
    scratch.color_mode_data = source.color_mode_data.clone();
    scratch.text_styles = source.text_styles.clone();
    let mut only = layer.clone();
    only.visible = true;
    only.source_channels = None;
    scratch.layers = LayerStack::from_layers(vec![only]);
    scratch.composite = pictura_render::composite(pictura_render::CompositeRequest::whole(
        &scratch,
        gpu_compute,
    ))
    .0;
    if scratch.mode == ColorMode::Grayscale && scratch.composite.channels == 4 {
        let plane = scratch.composite.pixel_count();
        let data = scratch.composite.data.to_vec();
        let mut normalized = vec![0u8; plane * 2];
        normalized[..plane].copy_from_slice(&data[..plane]);
        normalized[plane..].copy_from_slice(&data[3 * plane..4 * plane]);
        scratch.composite = pictura_core::PixelBuffer {
            width: scratch.composite.width,
            height: scratch.composite.height,
            channels: 2,
            data: normalized.into(),
        };
    }
    scratch.merged_composite_present = true;
    scratch
}

fn effective_icc(doc: &Document) -> Vec<u8> {
    let expected = doc.mode.color_channels();
    if let Some(icc) = doc.document_icc.as_ref() {
        if pictura_codec::Profile::from_icc(icc)
            .is_ok_and(|profile| profile.channels() == Some(expected))
        {
            return icc.to_vec();
        }
    }
    if doc.mode == ColorMode::Grayscale {
        pictura_codec::Profile::gray_gamma22().to_icc()
    } else {
        pictura_codec::Profile::srgb().to_icc()
    }
}

fn apply_icc(doc: &mut Document, include: bool) {
    if include {
        let icc = effective_icc(doc);
        pictura_codec::embed_document_icc(doc, &icc);
    } else {
        pictura_codec::strip_document_icc(doc);
    }
}

fn raster_file(doc: &Document, format: &str, include_icc: bool, path: &str) -> bool {
    let buffer = &doc.composite;
    let rgba = super::super::helpers_composite::buffer_to_rgba_bytes(buffer);
    let mut metadata = pictura_codec::raster_metadata(doc);
    if include_icc {
        metadata.icc = Some(effective_icc(doc));
    }
    let options = super::super::save_format::SaveRasterOptions {
        quality: if format == "JPG" { 90 } else { -1 },
        ..Default::default()
    };
    super::super::export::write_raster_with_metadata(
        &rgba,
        buffer.width,
        buffer.height,
        path,
        format,
        &metadata,
        &options,
    )
}

fn pdf_file(doc: &Document, include_icc: bool, path: &str) -> bool {
    let mut editing = doc.clone();
    apply_icc(&mut editing, include_icc);
    let mut page = editing.clone();
    page.source_mode = None;
    page.source_depth = None;
    page.source_planes = None;
    page.source_palette = None;
    page.source_icc = None;
    let mut options = pictura_codec::pdf::PdfOptions {
        preserve_editing: true,
        layers: true,
        icc_profile: include_icc,
        ..Default::default()
    };
    options.normalize();
    let jpeg = |request: &pictura_codec::pdf::JpegRequest| {
        let bytes = super::super::pdf_export::ffi::encode_jpeg(
            request.samples,
            request.width as i32,
            request.height as i32,
            i32::from(request.components),
            i32::from(request.quality),
        );
        (!bytes.is_empty()).then_some(bytes)
    };
    let env = pictura_codec::pdf::PdfEnv {
        jpeg: &jpeg,
        now: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default(),
        entropy: pictura_codec::os_random(),
        title: std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        live_text: None,
        proof: None,
        editing_doc: Some(&editing),
    };
    match pictura_codec::pdf::write_pdf(&page, &page.composite, &options, &env) {
        Ok(bytes) => super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok(),
        Err(_) => false,
    }
}

pub fn export_layer_file(
    view: &PictureView,
    layer_path: &QString,
    format: &QString,
    include_icc: bool,
    path: &QString,
) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let Some(layer) = pictura_render::resolve_path(doc, &layer_path.to_string()) else {
        return false;
    };
    let mut scratch = scratch_document(doc, layer, rust.gpu_compute);
    delay();
    write_document_file(
        &mut scratch,
        &format.to_string(),
        include_icc,
        &path.to_string(),
    )
}

/// Write a fully prepared document in the selected export format. Shared by
/// Export Layers and Layer Comps to Files so both use one save edge.
pub(crate) fn write_document_file(
    doc: &mut Document,
    format: &str,
    include_icc: bool,
    path: &str,
) -> bool {
    match format.to_ascii_uppercase().as_str() {
        "PSD" => {
            apply_icc(doc, include_icc);
            match pictura_codec::write_psd(doc) {
                Ok(bytes) => {
                    super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok()
                }
                Err(_) => false,
            }
        }
        "JPEG" | "JPG" => raster_file(doc, "JPG", include_icc, path),
        "TIFF" | "TIF" => raster_file(doc, "TIF", include_icc, path),
        "BMP" => raster_file(doc, "BMP", false, path),
        "PDF" => pdf_file(doc, include_icc, path),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{AdjustmentData, Plane, PsdRect};

    fn layer(name: &str) -> Layer {
        Layer {
            name: name.to_string(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 10,
                right: 10,
            },
            ..Default::default()
        }
    }

    fn group(children: Vec<Layer>) -> Layer {
        Layer {
            name: "Group".to_string(),
            is_group: true,
            children: LayerStack::from_layers(children),
            ..Default::default()
        }
    }

    fn doc(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(10, 10, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    #[test]
    fn scratch_document_carries_the_global_light_and_pattern_state() {
        let mut source = doc(vec![layer("a")]);
        source.global_light = pictura_core::GlobalLight {
            angle: 90,
            altitude: 45,
        };
        source.layer_section_extra = vec![1u8, 2, 3].into();
        source.is_psb = true;
        let scratch = scratch_document(&source, &layer("a"), false);
        assert_eq!(scratch.global_light, source.global_light);
        assert_eq!(
            &scratch.layer_section_extra[..],
            &source.layer_section_extra[..]
        );
        assert!(scratch.is_psb);
    }

    #[test]
    fn export_layers_group_children_are_targets() {
        let doc = doc(vec![group(vec![layer("a"), layer("b")])]);
        let targets = collect_targets(&doc, false, "");
        assert_eq!(
            targets,
            vec![
                ("0/0".to_string(), "a".to_string()),
                ("0/1".to_string(), "b".to_string())
            ]
        );
    }

    #[test]
    fn export_layers_adjustment_is_a_target() {
        let mut adjustment = layer("Adjustment");
        adjustment.adjustment = Some(AdjustmentData {
            key: *b"inv ",
            data: Plane::default(),
        });
        let doc = doc(vec![adjustment]);
        assert_eq!(collect_targets(&doc, false, "").len(), 1);
    }

    #[test]
    fn export_layers_hidden_exported_when_off() {
        let mut hidden = layer("hidden");
        hidden.visible = false;
        let doc = doc(vec![hidden]);
        assert_eq!(collect_targets(&doc, false, "").len(), 1);
    }

    #[test]
    fn export_layers_hidden_skipped_when_on() {
        let mut hidden = layer("hidden");
        hidden.visible = false;
        let doc = doc(vec![hidden]);
        assert!(collect_targets(&doc, true, "").is_empty());
    }

    #[test]
    fn export_layers_hidden_ancestor_skips_child() {
        let mut hidden = group(vec![layer("child")]);
        hidden.visible = false;
        let doc = doc(vec![hidden]);
        assert!(collect_targets(&doc, true, "").is_empty());
    }

    #[test]
    fn export_layers_name_sanitizes() {
        let doc = doc(vec![layer("a/b:c?")]);
        assert_eq!(collect_targets(&doc, false, "Export")[0].1, "Exporta_b_c_");
    }

    #[test]
    fn export_layers_name_collisions() {
        let doc = doc(vec![layer("same"), layer("same")]);
        let names: Vec<String> = collect_targets(&doc, false, "")
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert_eq!(names, vec!["same".to_string(), "same 2".to_string()]);
    }

    #[test]
    fn export_layers_name_falls_back() {
        let doc = doc(vec![layer("")]);
        assert_eq!(collect_targets(&doc, false, "")[0].1, "Layer");
    }

    #[test]
    fn export_layers_name_truncates_utf8() {
        let long = "é".repeat(200);
        let doc = doc(vec![layer(&long)]);
        let name = &collect_targets(&doc, false, "")[0].1;
        assert!(name.len() + ".tif".len() + 21 <= 255);
        assert!(std::str::from_utf8(name.as_bytes()).is_ok());
    }

    #[test]
    fn export_layers_name_reserves_temporary_suffixes() {
        let long = "a".repeat(300);
        let doc = doc(vec![layer(&long)]);
        let name = &collect_targets(&doc, false, "")[0].1;
        assert_eq!(name.len(), 230);
        assert!(name.len() + ".tif".len() + 21 <= 255);
    }

    #[test]
    fn export_layers_cmyk_projection_keeps_the_alpha() {
        let mut source = doc(vec![layer("a")]);
        source.source_mode = Some(ColorMode::Cmyk);
        let scratch = scratch_document(&source, &source.layers[0], false);
        assert_eq!(scratch.composite.channels, 4);
    }

    #[test]
    fn export_layers_grayscale_composite_keeps_gray_and_alpha() {
        let mut source = Document::new(10, 10, ColorMode::Grayscale, BitDepth::Eight);
        let only = layer("gray");
        source.layers = LayerStack::from_layers(vec![only.clone()]);
        let scratch = scratch_document(&source, &only, false);
        assert_eq!(scratch.composite.channels, 2);
        assert_eq!(scratch.composite.data.len(), 2 * 10 * 10);
    }

    #[test]
    fn export_layers_grayscale_psd_and_pdf_use_gray() {
        let mut source = Document::new(10, 10, ColorMode::Grayscale, BitDepth::Eight);
        let only = layer("gray");
        source.layers = LayerStack::from_layers(vec![only.clone()]);
        let mut scratch = scratch_document(&source, &only, false);
        let plane = scratch.composite.pixel_count();
        let mut data = scratch.composite.data.to_vec();
        for i in 0..plane {
            data[i] = 64;
            data[plane + i] = 128;
        }
        scratch.composite.data = data.into();
        apply_icc(&mut scratch, false);

        let psd = pictura_codec::write_psd(&scratch).expect("PSD writes");
        assert_eq!(u16::from_be_bytes([psd[12], psd[13]]), 2, "gray + alpha");
        assert_eq!(u16::from_be_bytes([psd[24], psd[25]]), 1, "grayscale mode");

        let path =
            std::env::temp_dir().join(format!("pictura-export-gray-{}.pdf", std::process::id()));
        let path_text = path.to_string_lossy().into_owned();
        assert!(pdf_file(&scratch, false, &path_text));
        let pdf = std::fs::read(&path).expect("PDF reads");
        let _ = std::fs::remove_file(&path);
        assert!(pdf.starts_with(b"%PDF"));
        assert!(pdf.windows(10).any(|window| window == b"DeviceGray"));
        assert!(!pdf.windows(10).any(|window| window == b"DeviceCMYK"));
    }

    #[test]
    fn export_layers_icc_option_embeds_and_strips() {
        let mut scratch = scratch_document(&doc(vec![layer("a")]), &layer("a"), false);
        apply_icc(&mut scratch, true);
        assert!(scratch.document_icc.is_some());
        assert!(pictura_codec::decode_image_resources(&scratch)
            .iter()
            .any(|resource| resource.id == pictura_codec::ICC_PROFILE));
        apply_icc(&mut scratch, false);
        assert!(scratch.document_icc.is_none());
        assert!(pictura_codec::decode_image_resources(&scratch)
            .iter()
            .all(|resource| resource.id != pictura_codec::ICC_PROFILE));
    }

    #[test]
    fn export_layers_scratch_document_keeps_the_layer() {
        let mut hidden = layer("hidden");
        hidden.rect = PsdRect {
            top: 2,
            left: 3,
            bottom: 8,
            right: 9,
        };
        hidden.visible = false;
        let doc = doc(vec![hidden.clone()]);
        let scratch = scratch_document(&doc, &hidden, false);
        assert_eq!(scratch.width, 10);
        assert_eq!(scratch.height, 10);
        assert_eq!(scratch.layers.len(), 1);
        assert_eq!(scratch.layers[0].rect, hidden.rect);
        assert!(scratch.layers[0].visible);
        assert!(scratch.merged_composite_present);
        assert_eq!(
            (scratch.composite.width, scratch.composite.height),
            (10, 10)
        );
    }
}
