use super::{
    ffi::{PreparedVectorPdf, VectorPlaceResult},
    PictureView, Pin, QString,
};
use crate::cxxqt_object::edit::{structural_after, Edit};
use crate::cxxqt_object::state::{PendingVectorPlace, PictureViewRust};
use crate::cxxqt_object::{helpers::safe_io, impl_core::Outcome, pdf_export::vector_geometry};
use cxx_qt::CxxQtType;
use pictura_codec::pdf::smart_source::{self, SmartPdf};
use pictura_core::{Document, Layer, SmartObject, SmartObjectKind, VectorPlacement};
use std::path::Path;

pub fn vector_pdf_page_count(bytes: &[u8]) -> i32 {
    if (bytes.len() as u64) > vector_pdf_import_limit() {
        return 0;
    }
    SmartPdf::open(bytes).map_or(0, |pdf| pdf.page_count() as i32)
}

pub fn vector_pdf_import_limit() -> u64 {
    crate::cxxqt_object::helpers::safe_io::import_file_limit()
}

pub fn prepare_vector_pdf(
    bytes: &[u8],
    page: u32,
    crop: u8,
    canvas_width: u32,
    canvas_height: u32,
) -> PreparedVectorPdf {
    let mut output = PreparedVectorPdf {
        error: QString::default(),
        page,
        page_count: 0,
        crop,
        width_points: 0.0,
        height_points: 0.0,
        left: 0,
        top: 0,
        width: 0,
        height: 0,
        rgba: Vec::new(),
    };
    if canvas_width == 0 || canvas_height == 0 {
        output.error = QString::from("The destination has no canvas.");
        return output;
    }
    if (bytes.len() as u64) > vector_pdf_import_limit() {
        output.error = QString::from("The PDF exceeds the import size limit.");
        return output;
    }
    let render = (|| {
        let mut pdf = SmartPdf::open(bytes)?;
        let placement = pdf.prepare(page, crop)?;
        let (width, height) = (placement.width_points, placement.height_points);
        let scale = (f64::from(canvas_width) / width)
            .min(f64::from(canvas_height) / height)
            .min(1.0);
        let size = (
            (width * scale).round().max(1.0) as u32,
            (height * scale).round().max(1.0) as u32,
        );
        let raster = pdf.render(placement, size)?;
        Ok::<_, pictura_codec::RasterError>((placement, raster, scale < 1.0))
    })();
    match render {
        Ok((placement, raster, fitted)) => {
            output.page_count = placement.page_count;
            output.width_points = placement.width_points;
            output.height_points = placement.height_points;
            output.width = raster.width;
            output.height = raster.height;
            if fitted {
                output.left = (canvas_width.saturating_sub(raster.width) / 2) as i32;
                output.top = (canvas_height.saturating_sub(raster.height) / 2) as i32;
            }
            output.rgba = raster.rgba;
        }
        Err(error) => output.error = QString::from(error.to_string().as_str()),
    }
    output
}

pub fn place_prepared_pdf(
    mut view: Pin<&mut PictureView>,
    source: &PreparedVectorPdf,
    bytes: &[u8],
    filename: &QString,
    smart: bool,
    label: &QString,
) -> QString {
    if source.width == 0 || source.height == 0 {
        return QString::default();
    }
    let filename = filename.to_string();
    let layer_name = Path::new(&filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&filename);
    let label = label.to_string();
    let mut created = String::new();
    let mut insert = |rust: &mut PictureViewRust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let path = pictura_render::add_raster_layer_from_rgba(
            doc,
            layer_name,
            source.width,
            source.height,
            &source.rgba,
        );
        let Some(layer) = pictura_render::resolve_path_mut(doc, &path) else {
            return Edit::None;
        };
        layer.rect.left += source.left;
        layer.rect.right += source.left;
        layer.rect.top += source.top;
        layer.rect.bottom += source.top;
        if smart {
            let mut object = SmartObject::default();
            object.kind = SmartObjectKind::Embedded;
            object.uuid = pictura_codec::new_vector_source_id();
            object.filetype = smart_source::filetype(&filename);
            object.filename = filename.clone();
            object.authored_vector_placement = true;
            object.vector_placement = Some(VectorPlacement {
                page: source.page,
                page_count: source.page_count,
                crop: source.crop,
                width_points: source.width_points,
                height_points: source.height_points,
            });
            object.set_payload(bytes.to_vec());
            pictura_codec::edit_layer(layer, |parts| parts.smart_object = Some(object));
        }
        let damage = pictura_render::damage::influence(doc, &path);
        rust.select_only_layer(&path);
        if label == "Paste" {
            rust.session.set_selection(None);
        }
        created = path;
        structural_after(damage.map(|rect| vec![rect]))
    };
    if label == "Place" && smart {
        view.as_mut().settle_pending_composite();
        {
            let mut rust = view.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.snapshot() else {
                return QString::default();
            };
            if rust.pending_vector_place.is_some() {
                return QString::default();
            }
            let before = PendingVectorPlace {
                doc,
                target: rust.layer_target.clone(),
                generation: rust.session.document_generation,
                history_id: rust.session.history.current_id(),
            };
            if matches!(insert(&mut rust), Edit::None) {
                return QString::default();
            }
            rust.pending_vector_place = Some(before);
            rust.vector_place_result = VectorPlaceResult::Pending;
        }
        view.as_mut().recomposite();
    } else {
        view.as_mut().edit(&label, insert);
    }
    QString::from(created.as_str())
}

pub fn vector_place_result(view: &PictureView) -> VectorPlaceResult {
    view.rust().vector_place_result
}

impl PictureView {
    pub(in crate::cxxqt_object) fn finish_vector_place(
        mut self: Pin<&mut Self>,
        result: VectorPlaceResult,
    ) -> bool {
        let Some(before) = self.as_mut().rust_mut().pending_vector_place.take() else {
            return false;
        };
        let session = &self.rust().session;
        if session.document_generation != before.generation
            || session.history.current_id() != before.history_id
        {
            self.as_mut().rust_mut().vector_place_result = VectorPlaceResult::Refused;
            return false;
        }
        self.as_mut().rust_mut().vector_place_result = result;
        if result == VectorPlaceResult::Committed {
            self.as_mut()
                .commit_transform_edit("Place", true, |_| Edit::Structural { damage: None });
            return true;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session.doc.set(Some(before.doc));
            rust.layer_target = before.target;
        }
        self.as_mut().recomposite();
        false
    }
}

pub fn vector_smart_object_id(view: &PictureView, path: &QString) -> QString {
    QString::from(vector_object(view, path).map_or("", |object| object.uuid.as_str()))
}

pub fn vector_smart_object_suffix(view: &PictureView, path: &QString) -> QString {
    QString::from(match vector_object(view, path) {
        Some(object)
            if object.filetype == *b"AI  "
                || smart_source::filetype(&object.filename) == *b"AI  " =>
        {
            "ai"
        }
        Some(_) => "pdf",
        None => "",
    })
}

fn vector_object<'a>(view: &'a PictureView, path: &QString) -> Option<&'a SmartObject> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::resolve_path(doc, &path.to_string())
        .and_then(|layer| layer.smart_object())
        .filter(|object| {
            object.kind == SmartObjectKind::Embedded && object.vector_placement.is_some()
        })
}

pub fn vector_edit_revision(view: &PictureView, id: &QString) -> u64 {
    let id = id.to_string();
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return 0;
    };
    vector_geometry::source_instances(&doc.layers, &id)
        .first()
        .and_then(|layer| layer.smart_object())
        .map_or(0, SmartObject::payload_digest)
}

pub fn start_vector_edit(
    view: Pin<&mut PictureView>,
    id: &QString,
    expected: u64,
    file: &QString,
) -> bool {
    let id = id.to_string();
    let file = file.to_string();
    view.start_source_job("Edit Contents", move |doc| {
        let bytes = safe_io::read_capped(file, safe_io::import_file_limit())?;
        apply_vector_edit(doc, &id, expected, bytes)
    })
}

fn apply_vector_edit(
    doc: &mut Document,
    id: &str,
    expected: u64,
    bytes: Vec<u8>,
) -> Option<Outcome> {
    if id.is_empty() || expected == 0 {
        return None;
    }
    let layers = vector_geometry::source_instances(&doc.layers, id);
    let first = layers.first()?.smart_object()?;
    let old = first.payload()?;
    if first.payload_digest() != expected
        || layers.iter().any(|layer| {
            layer.smart_object().is_none_or(|object| {
                object.vector_placement.is_none() || object.payload() != Some(old)
            })
        })
    {
        return None;
    }
    if old == bytes {
        return Some(Outcome::default());
    }
    let mut pdf = SmartPdf::open(&bytes).ok()?;
    let has_config = layers
        .iter()
        .any(|layer| !layer.smart_object().unwrap().config_descriptor.is_empty());
    let authored_record = if has_config {
        Some(pictura_codec::replace_embedded_payload(
            &doc.layer_section_extra,
            id,
            &bytes,
            doc.is_psb,
        )?)
    } else {
        None
    };
    let mut source = SmartObject::default();
    source.set_payload(bytes);
    let mut changed = 0;
    vector_geometry::update_source_instances(&mut doc.layers, id, &mut |layer: &mut Layer| {
        let object = layer.smart_object()?;
        let selection = object.vector_placement?;
        let selection = pdf.prepare(selection.page, selection.crop).ok()?;
        let pixels =
            vector_geometry::render(&pdf, selection, layer.rect, &object.vector_transforms)?;
        pictura_codec::edit_layer(layer, |parts| {
            let object = parts.smart_object.as_mut().expect("source checked above");
            object.vector_placement = Some(selection);
            object.payload_digest = source.payload_digest();
            *object.payload_mut() = source.payload_mut().clone();
        });
        vector_geometry::install_proxy(layer, pixels)?;
        changed += 1;
        Some(())
    })?;
    if let Some(extra) = authored_record {
        doc.layer_section_extra = extra.into();
    }
    Some(Outcome {
        count: changed,
        ..Outcome::default()
    })
}
