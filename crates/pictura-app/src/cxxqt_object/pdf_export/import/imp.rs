use super::*;

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::pdf::import::{self as pdf_import, PdfImages, Rendered};
use pictura_codec::{eps, Ghostscript, Raster, RasterError, VectorKind};
use std::path::Path;

use ffi::ImportTarget;

pub(super) fn error_text(error: &RasterError) -> QString {
    let kind = match error {
        RasterError::NoGhostscript => "nogs",
        RasterError::PasswordRequired => "password",
        RasterError::WrongPassword => "wrongpassword",
        RasterError::Failed(_) => "failed",
    };
    QString::from(format!("error {kind}\n{error}").as_str())
}

pub(super) fn ok_text(lines: &[String]) -> QString {
    let mut text = String::from("ok");
    for line in lines {
        text.push('\n');
        text.push_str(line);
    }
    QString::from(text.as_str())
}

pub(super) fn password(text: &QString) -> Option<String> {
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

pub(super) fn probe(path: &Path, password: Option<&str>) -> Result<Vec<String>, RasterError> {
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

pub(super) fn pack(rasters: &[Raster]) -> Vec<u8> {
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
    let spec = super::super::super::open_jobs::ImportSpec::from_target(target);
    let rendered = super::super::super::open_jobs::render_pdf_page(
        &path.to_string(),
        &password_text.to_string(),
        page,
        &area.to_string(),
        &spec,
    );
    finish_rendered(view, rendered, &spec)
}

pub fn pdf_import_open_image(
    view: Pin<&mut PictureView>,
    path: &QString,
    index: i32,
    target: &ImportTarget,
) -> QString {
    let spec = super::super::super::open_jobs::ImportSpec::from_target(target);
    let rendered =
        super::super::super::open_jobs::render_pdf_image(&path.to_string(), index, &spec);
    finish_rendered(view, rendered, &spec)
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
    let spec = super::super::super::open_jobs::ImportSpec::from_target(target);
    let rendered = super::super::super::open_jobs::render_eps(&path.to_string(), &spec);
    finish_rendered(view, rendered, &spec)
}

/// Install a render into the view as an untitled document, or report why not.
pub(super) fn finish_rendered(
    mut view: Pin<&mut PictureView>,
    rendered: Result<Rendered, RasterError>,
    spec: &super::super::super::open_jobs::ImportSpec,
) -> QString {
    match rendered {
        Ok(rendered) => {
            match super::super::super::open_jobs::imported_document(
                &rendered.raster,
                spec,
                view.rust().gpu_compute,
                &|_, _| {},
            ) {
                Some(opened) => {
                    super::super::super::open_jobs::apply_opened(
                        &mut view.as_mut().rust_mut(),
                        opened,
                    );
                    view.as_mut().session_cancelled();
                    ok_text(&rendered.warnings)
                }
                None => error_text(&RasterError::Failed(
                    "the image could not be converted".into(),
                )),
            }
        }
        Err(error) => error_text(&error),
    }
}
