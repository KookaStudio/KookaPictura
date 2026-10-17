//! The generic PDF / EPS import bridge (`interop/pdf-eps-import`): sniffing,
//! the page and image facts the Import PDF dialog shows, thumbnails, and
//! opening a page, an embedded image, or an EPS into a view through
//! Ghostscript (`pictura_codec::pdf::import`, `pictura_codec::eps`).
//!
//! Results travel as text: a first line `ok` or `error <kind>` (`nogs`,
//! `password`, `wrongpassword`, `failed`), then one fact, warning, or the
//! error message per line.

use super::super::helpers_composite::{current_buffer, store_composite};
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::pdf::import::{self as pdf_import, PdfImages, PdfRect, Rendered};
use pictura_codec::{eps, Ghostscript, Raster, RasterError, Resolution, VectorKind};
use pictura_core::{BitDepth, ColorMode, Document};
use std::path::Path;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    /// What one import produces: pixel size, edge smoothing, and the new document's mode (`grayscale`, `rgb`, `cmyk`, `lab`), depth (8 or 16), resolution, and display name.
    #[namespace = "pictura"]
    struct ImportTarget {
        width: i32,
        height: i32,
        anti_alias: bool,
        mode: QString,
        depth: i32,
        ppi: f64,
        per_cm: bool,
        name: QString,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `pdf` or `eps` from the file's leading bytes; empty for anything else.
        fn vector_sniff(path: &QString) -> QString;

        /// The pages of a PDF: `ok`, then per page `page <view height> <x0 y0 x1 y1>×6` (Bounding, Media, Crop, Bleed, Trim, Art boxes in view-space points), then `images <n>` (`-1` when unavailable) and `image <width> <height> <page>` per image.
        fn pdf_import_probe(path: &QString, password: &QString) -> QString;

        /// Page thumbnails whose longest page side is `longest` px: per page a little-endian `u32` width and height, then RGBA8888. Empty on failure.
        fn pdf_import_thumbnails(path: &QString, password: &QString, longest: i32) -> Vec<u8>;

        /// Images-mode thumbnails at most `longest` px, in the same layout.
        fn pdf_import_image_thumbnails(path: &QString, longest: i32) -> Vec<u8>;

        /// Open page `page` (1-based), cropped to `area` (`x0 y0 x1 y1 view_height`), into the view as a new document.
        fn pdf_import_open_page(
            view: Pin<&mut PictureView>,
            path: &QString,
            password: &QString,
            page: i32,
            area: &QString,
            target: &ImportTarget,
        ) -> QString;

        /// Open embedded image `index` at its own size into the view; `target`'s size is ignored.
        fn pdf_import_open_image(
            view: Pin<&mut PictureView>,
            path: &QString,
            index: i32,
            target: &ImportTarget,
        ) -> QString;

        /// The EPS bounding box: `ok`, then `x0 y0 x1 y1` in points.
        fn eps_import_probe(path: &QString) -> QString;

        /// Rasterize an EPS's bounding box into the view as a new document.
        fn eps_import_open(
            view: Pin<&mut PictureView>,
            path: &QString,
            target: &ImportTarget,
        ) -> QString;
    }
}

use ffi::ImportTarget;

fn error_text(error: &RasterError) -> QString {
    let kind = match error {
        RasterError::NoGhostscript => "nogs",
        RasterError::PasswordRequired => "password",
        RasterError::WrongPassword => "wrongpassword",
        RasterError::Failed(_) => "failed",
    };
    QString::from(format!("error {kind}\n{error}").as_str())
}

fn ok_text(lines: &[String]) -> QString {
    let mut text = String::from("ok");
    for line in lines {
        text.push('\n');
        text.push_str(line);
    }
    QString::from(text.as_str())
}

fn password(text: &QString) -> Option<String> {
    let text = text.to_string();
    (!text.is_empty()).then_some(text)
}

pub fn vector_sniff(path: &QString) -> QString {
    let mut head = [0u8; 1024];
    let read = std::fs::File::open(path.to_string())
        .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
        .unwrap_or(0);
    QString::from(match pictura_codec::sniff_vector(&head[..read]) {
        Some(VectorKind::Pdf) => "pdf",
        Some(VectorKind::Eps) => "eps",
        None => "",
    })
}

pub fn pdf_import_probe(path: &QString, password_text: &QString) -> QString {
    let path = path.to_string();
    let password = password(password_text);
    match probe(Path::new(&path), password.as_deref()) {
        Ok(lines) => ok_text(&lines),
        Err(error) => error_text(&error),
    }
}

fn probe(path: &Path, password: Option<&str>) -> Result<Vec<String>, RasterError> {
    let gs = Ghostscript::locate()?;
    let mut pages = pdf_import::page_info(&gs, path, password)?;
    pdf_import::query_bounding_boxes(&gs, path, password, &mut pages)?;
    let mut lines: Vec<String> = pages
        .iter()
        .map(|page| {
            let mut line = format!("page {}", page.view_size().1);
            for kind in 0..6 {
                let r = page.view_box(pdf_import::CropTo::from_index(kind));
                line.push_str(&format!(" {} {} {} {}", r.x0, r.y0, r.x1, r.y1));
            }
            line
        })
        .collect();
    match std::fs::read(path)
        .ok()
        .as_deref()
        .and_then(PdfImages::read)
    {
        Some(found) => {
            lines.push(format!("images {}", found.images.len()));
            lines.extend(
                found
                    .images
                    .iter()
                    .map(|i| format!("image {} {} {}", i.width, i.height, i.page)),
            );
        }
        None => lines.push("images -1".into()),
    }
    Ok(lines)
}

fn pack(rasters: &[Raster]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in rasters {
        out.extend_from_slice(&r.width.to_le_bytes());
        out.extend_from_slice(&r.height.to_le_bytes());
        out.extend_from_slice(&r.rgba);
    }
    out
}

pub fn pdf_import_thumbnails(path: &QString, password_text: &QString, longest: i32) -> Vec<u8> {
    let path = path.to_string();
    let password = password(password_text);
    let rasters = Ghostscript::locate().and_then(|gs| {
        let pages = pdf_import::page_info(&gs, Path::new(&path), password.as_deref())?;
        pdf_import::render_thumbnails(
            &gs,
            Path::new(&path),
            password.as_deref(),
            &pages,
            longest.max(1) as u32,
        )
    });
    rasters.map(|r| pack(&r)).unwrap_or_default()
}

pub fn pdf_import_image_thumbnails(path: &QString, longest: i32) -> Vec<u8> {
    let Ok(gs) = Ghostscript::locate() else {
        return Vec::new();
    };
    let Some(found) = std::fs::read(path.to_string())
        .ok()
        .as_deref()
        .and_then(PdfImages::read)
    else {
        return Vec::new();
    };
    let rasters: Option<Vec<Raster>> = (0..found.images.len())
        .map(|i| {
            found
                .render(&gs, i, Some(longest.max(1) as u32))
                .ok()
                .map(|r| r.raster)
        })
        .collect();
    rasters.map(|r| pack(&r)).unwrap_or_default()
}

pub fn pdf_import_open_page(
    view: Pin<&mut PictureView>,
    path: &QString,
    password_text: &QString,
    page: i32,
    area: &QString,
    target: &ImportTarget,
) -> QString {
    let path = path.to_string();
    let password = password(password_text);
    let numbers: Vec<f64> = area
        .to_string()
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    let [x0, y0, x1, y1, view_height] = numbers[..] else {
        return error_text(&RasterError::Failed("bad page area".into()));
    };
    let rendered = Ghostscript::locate().and_then(|gs| {
        pdf_import::render_area(
            &gs,
            Path::new(&path),
            password.as_deref(),
            page.max(1) as u32,
            PdfRect::new(x0, y0, x1, y1),
            view_height,
            (target.width.max(1) as u32, target.height.max(1) as u32),
            target.anti_alias,
        )
    });
    finish(view, rendered, target)
}

pub fn pdf_import_open_image(
    view: Pin<&mut PictureView>,
    path: &QString,
    index: i32,
    target: &ImportTarget,
) -> QString {
    let rendered = Ghostscript::locate().and_then(|gs| {
        let found = std::fs::read(path.to_string())
            .ok()
            .as_deref()
            .and_then(PdfImages::read)
            .ok_or_else(|| RasterError::Failed("no readable images".into()))?;
        found.render(&gs, index.max(0) as usize, None)
    });
    finish(view, rendered, target)
}

pub fn eps_import_probe(path: &QString) -> QString {
    match std::fs::read(path.to_string())
        .ok()
        .as_deref()
        .and_then(eps::bounding_box)
    {
        Some(r) => ok_text(&[format!("{} {} {} {}", r.x0, r.y0, r.x1, r.y1)]),
        None => error_text(&RasterError::Failed("no bounding box".into())),
    }
}

pub fn eps_import_open(
    view: Pin<&mut PictureView>,
    path: &QString,
    target: &ImportTarget,
) -> QString {
    let path = path.to_string();
    let rendered = Ghostscript::locate().and_then(|gs| {
        let bounds = std::fs::read(&path)
            .ok()
            .as_deref()
            .and_then(eps::bounding_box)
            .ok_or_else(|| RasterError::Failed("no bounding box".into()))?;
        eps::render_eps(
            &gs,
            Path::new(&path),
            bounds,
            (target.width.max(1) as u32, target.height.max(1) as u32),
            target.anti_alias,
        )
    });
    finish(view, rendered, target)
}

/// Install a render into the view as an untitled document, or report why not.
fn finish(
    view: Pin<&mut PictureView>,
    rendered: Result<Rendered, RasterError>,
    target: &ImportTarget,
) -> QString {
    match rendered {
        Ok(rendered) => match install(view, &rendered.raster, target) {
            true => ok_text(&rendered.warnings),
            false => error_text(&RasterError::Failed(
                "the image could not be converted".into(),
            )),
        },
        Err(error) => error_text(&error),
    }
}

/// The imported document: one transparent `Layer 1` in `target`'s mode,
/// depth, and resolution (design D5, D6).
fn import_document(raster: &Raster, target: &ImportTarget) -> Option<Document> {
    let name = target.name.to_string();
    let mut doc = Document::from_rgba(&name, raster.width, raster.height, &raster.rgba);
    if let Some(layer) = doc.layers.last_mut() {
        layer.name = "Layer 1".to_string();
    }
    pictura_codec::set_document_resolution(
        &mut doc,
        Resolution {
            ppi: if target.ppi > 0.0 { target.ppi } else { 72.0 },
            per_cm: target.per_cm,
        },
    );
    let mode = match target.mode.to_string().as_str() {
        "grayscale" => ColorMode::Grayscale,
        "cmyk" => ColorMode::Cmyk,
        "lab" => ColorMode::Lab,
        _ => ColorMode::Rgb,
    };
    if mode != ColorMode::Rgb {
        pictura_render::convert_mode(&mut doc, mode).ok()?;
    }
    if target.depth == 16 {
        pictura_render::convert_bit_depth(&mut doc, BitDepth::Sixteen).ok()?;
    }
    Some(doc)
}

fn install(mut view: Pin<&mut PictureView>, raster: &Raster, target: &ImportTarget) -> bool {
    let Some(mut doc) = import_document(raster, target) else {
        return false;
    };
    let gpu_compute = view.rust().gpu_compute;
    let rendered = current_buffer(&doc, gpu_compute);
    store_composite(&mut doc, &rendered);
    let mut rust = view.as_mut().rust_mut();
    rust.doc = Some(doc);
    rust.reset_edit_state();
    rust.active_layer = Some("0".to_string());
    let rust = &mut *rust;
    if let Some(doc) = rust.doc.as_mut() {
        rust.history.capture_live(doc, &None, "Open");
    }
    rust.path = None;
    rust.dirty = false;
    rust.source_format = "psd".into();
    true
}
