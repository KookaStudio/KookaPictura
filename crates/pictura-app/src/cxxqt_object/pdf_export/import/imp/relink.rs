use super::{PictureView, Pin, QString};
use crate::cxxqt_object::helpers::{decode_import, safe_io};
use crate::cxxqt_object::helpers_composite::buffer_to_rgba_bytes;
use crate::cxxqt_object::impl_core::Outcome;
use crate::cxxqt_object::vector_geometry;
use cxx_qt::CxxQtType;
use pictura_codec::pdf::smart_source::{filetype, SmartPdf};
use pictura_codec::{ExternalLinkReplacement, ImageBudget, ImageFormat, PlacedSource, Raster};
use pictura_core::{Document, Layer, SmartObject, SmartObjectKind};
use std::path::Path;

mod fixture;
pub use fixture::relink_fixture_for_test;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Family {
    Photoshop,
    Pdf,
    Illustrator,
    Png,
    Jpeg,
    Tiff,
}

fn original_family(object: &SmartObject) -> Option<Family> {
    Some(match &object.filetype {
        b"8BPS" | b"8BPB" => Family::Photoshop,
        b"PDF " => Family::Pdf,
        b"AI  " => Family::Illustrator,
        b"PNG " | b"PNGf" => Family::Png,
        b"JPEG" => Family::Jpeg,
        b"TIFF" => Family::Tiff,
        _ => return None,
    })
}

fn supported_source(doc: &Document, id: &str) -> Option<(Family, PlacedSource)> {
    if id.is_empty()
        || !pictura_codec::repairable_external_link(&doc.layer_section_extra, id, doc.is_psb)
    {
        return None;
    }
    let layers = vector_geometry::source_instances(&doc.layers, id);
    let first = layers.first()?.smart_object()?;
    let family = original_family(first)?;
    let source = pictura_codec::read_placed_source(&first.config_descriptor)?;
    let vector = matches!(family, Family::Pdf | Family::Illustrator);
    if (vector && source.crop.is_none()) || (!vector && source.page != 1) {
        return None;
    }
    for layer in layers {
        let object = layer.smart_object()?;
        if object.kind != SmartObjectKind::External
            || original_family(object)? != family
            || vector_geometry::size(layer.rect).is_none()
            || object.vector_transforms.last().is_some_and(|step| {
                step.destination.is_some_and(|destination| {
                    vector_geometry::size(destination) != vector_geometry::size(layer.rect)
                })
            })
        {
            return None;
        }
        let placement = pictura_codec::read_placed_source(&object.config_descriptor)?;
        if placement.width != source.width
            || placement.height != source.height
            || placement.page != source.page
            || placement.crop != source.crop
        {
            return None;
        }
    }
    Some((family, source))
}

pub fn relink_source_id(view: &PictureView, path: &QString) -> QString {
    let id = (|| {
        let doc = view.rust().session.doc.as_ref()?;
        let layer = pictura_render::resolve_path(doc, &path.to_string())?;
        let object = layer.smart_object()?;
        if object.kind != SmartObjectKind::External {
            return None;
        }
        supported_source(doc, &object.uuid)?;
        Some(object.uuid.as_str())
    })();
    QString::from(id.unwrap_or(""))
}

pub fn relink_source_location(view: &PictureView, id: &QString) -> QString {
    let location = view.rust().session.doc.as_ref().and_then(|doc| {
        pictura_codec::external_link_location(&doc.layer_section_extra, &id.to_string(), doc.is_psb)
    });
    QString::from(location.as_deref().unwrap_or(""))
}

pub fn start_relink_source(
    view: Pin<&mut PictureView>,
    id: &QString,
    file: &QString,
    relative: &QString,
    timestamp: &[u8],
) -> bool {
    let id = id.to_string();
    let file = file.to_string();
    let relative = relative.to_string();
    let timestamp = timestamp.try_into().ok();
    view.start_source_job("Relink to File", move |doc| {
        let bytes = safe_io::read_capped(&file, safe_io::import_file_limit())?;
        let filename = Path::new(&file).file_name()?.to_str()?;
        let full_path = format!("file://{file}");
        apply_relink(doc, &id, &full_path, &relative, filename, timestamp, &bytes)
    })
}

fn raster_replacement(bytes: &[u8], family: Family) -> Option<([u8; 4], Raster)> {
    if family == Family::Photoshop {
        let doc = pictura_codec::read_psd(bytes).ok()?;
        let image = pictura_render::smart_object_document_pixels(&doc)?;
        return Some((
            if doc.is_psb { *b"8BPB" } else { *b"8BPS" },
            Raster {
                width: image.width,
                height: image.height,
                rgba: buffer_to_rgba_bytes(&image),
            },
        ));
    }
    let probe = pictura_codec::probe_image(bytes, ImageBudget::default()).ok()?;
    let filetype = match (family, probe.format) {
        (Family::Png, ImageFormat::Png) => *b"PNG ",
        (Family::Jpeg, ImageFormat::Jpeg) => *b"JPEG",
        (Family::Tiff, ImageFormat::Tiff) => *b"TIFF",
        _ => return None,
    };
    let (rgba, width, height) = decode_import(bytes)?;
    Some((
        filetype,
        Raster {
            width,
            height,
            rgba,
        },
    ))
}

fn apply_relink(
    doc: &mut Document,
    id: &str,
    full_path: &str,
    relative_path: &str,
    filename: &str,
    timestamp: Option<[u8; 16]>,
    bytes: &[u8],
) -> Option<Outcome> {
    let (family, expected) = supported_source(doc, id)?;
    let (filetype, raster, mut pdf) = if matches!(family, Family::Pdf | Family::Illustrator) {
        let kind = filetype(filename);
        if (kind == *b"AI  ") != (family == Family::Illustrator) {
            return None;
        }
        (kind, None, Some(SmartPdf::open(bytes).ok()?))
    } else {
        let (filetype, raster) = raster_replacement(bytes, family)?;
        if (f64::from(raster.width), f64::from(raster.height)) != (expected.width, expected.height)
        {
            return None;
        }
        (filetype, Some(raster), None)
    };
    let extra = pictura_codec::repair_external_link(
        &doc.layer_section_extra,
        id,
        doc.is_psb,
        &ExternalLinkReplacement {
            full_path,
            relative_path,
            filename,
            filetype,
            bytes,
            timestamp,
        },
    )?;
    let mut count = 0;
    vector_geometry::update_source_instances(&mut doc.layers, id, &mut |layer: &mut Layer| {
        let object = layer.smart_object()?;
        let placed = pictura_codec::read_placed_source(&object.config_descriptor)?;
        let clip_placement = placed.transform.is_some();
        let transforms: Vec<_> = placed
            .transform
            .into_iter()
            .chain(object.vector_transforms.iter().cloned())
            .collect();
        let proxy = if let Some(pdf) = &mut pdf {
            let page = pdf.prepare(placed.page, placed.crop?).ok()?;
            if (page.width_points - placed.width).abs() > 0.01
                || (page.height_points - placed.height).abs() > 0.01
            {
                return None;
            }
            vector_geometry::render_projected(pdf, page, layer.rect, &transforms, clip_placement)?
        } else {
            vector_geometry::project(
                raster.as_ref()?.clone(),
                layer.rect,
                &transforms,
                clip_placement,
            )?
        };
        pictura_codec::edit_layer(layer, |parts| {
            let object = parts.smart_object.as_mut().expect("source checked above");
            object.filename = filename.to_owned();
            object.filetype = filetype;
        });
        vector_geometry::install_proxy(layer, proxy)?;
        count += 1;
        Some(())
    })?;
    doc.layer_section_extra = extra.into();
    Some(Outcome {
        count,
        ..Outcome::default()
    })
}
