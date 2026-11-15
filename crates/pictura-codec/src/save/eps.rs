//! EPS and DCS Save As writers.

use std::path::{Path, PathBuf};

use pictura_core::path::{NamedPath, VectorPath};
use pictura_core::{ColorMode, Document};

use super::{jpeg_quality_percent, write_atomic, JpegEncode, Raster, SaveOptions};
use crate::error::PsdError;

fn color_space(raster: &Raster) -> (&'static str, Vec<u8>) {
    match raster.mode {
        ColorMode::Cmyk => ("DeviceCMYK", raster.cmyk_interleaved()),
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => {
            ("DeviceGray", raster.gray.clone())
        }
        _ => ("DeviceRGB", raster.rgb_interleaved()),
    }
}

fn ascii85(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(4) {
        let mut n = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            n |= u32::from(b) << (24 - i * 8);
        }
        let mut digits = [0u8; 5];
        for d in digits.iter_mut().rev() {
            *d = (n % 85) as u8 + b'!';
            n /= 85;
        }
        out.extend_from_slice(&digits[..chunk.len() + 1]);
    }
    out.extend_from_slice(b"~>");
    out
}

fn hex(data: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0xF) as usize]);
    }
    out
}

/// PostScript RunLengthDecode framing. Binary `currentfile` image data is not
/// safe for every sample stream in Ghostscript (a leading run can be taken as
/// scanner tokens), so the Binary option uses a binary filter instead of raw
/// bytes.
fn run_length(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let mut run = 1;
        while at + run < data.len() && data[at + run] == data[at] && run < 128 {
            run += 1;
        }
        if run >= 2 {
            out.push((257 - run as u16) as u8);
            out.push(data[at]);
            at += run;
            continue;
        }
        let start = at;
        at += 1;
        while at < data.len() && at - start < 128 {
            let mut next = 1;
            while at + next < data.len() && data[at + next] == data[at] && next < 128 {
                next += 1;
            }
            if next >= 2 {
                break;
            }
            at += 1;
        }
        out.push((at - start - 1) as u8);
        out.extend_from_slice(&data[start..at]);
    }
    out.push(128);
    out
}

fn mask_rows(width: u32, height: u32, gray: &[u8]) -> Vec<u8> {
    let row = width.div_ceil(8) as usize;
    let mut out = vec![0u8; row * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            if gray.get(y * width as usize + x).copied().unwrap_or(0) < 128 {
                out[y * row + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn image_operator(
    width: u32,
    height: u32,
    space: &str,
    data: &[u8],
    encoding: u8,
    quality: u8,
    interpolation: bool,
    mask: bool,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let mut out = Vec::new();
    if !mask {
        out.extend_from_slice(format!("/{space} setcolorspace\n").as_bytes());
    }
    let components = if mask {
        1
    } else if space == "DeviceCMYK" {
        4
    } else if space == "DeviceGray" {
        1
    } else {
        3
    };
    let decode = match components {
        1 => "[0 1]",
        3 => "[0 1 0 1 0 1]",
        _ => "[0 1 0 1 0 1 0 1]",
    };
    let filter = match encoding {
        0 => "/ASCIIHexDecode filter",
        1 => "/ASCII85Decode filter",
        2 => "/RunLengthDecode filter",
        3 => "/DCTDecode filter",
        _ => "/ASCIIHexDecode filter",
    };
    out.extend_from_slice(
        format!(
            "<< /ImageType 1 /Width {width} /Height {height} /BitsPerComponent {} {} /Decode {decode} /ImageMatrix [{width} 0 0 -{height} 0 {height}] /DataSource currentfile {filter} {} >> {} \n",
            if mask { 1 } else { 8 },
            if mask { "/ImageMask true" } else { "" },
            if interpolation { "/Interpolate true" } else { "" },
            if mask { "imagemask" } else { "image" }
        )
        .as_bytes(),
    );
    match encoding {
        0 => {
            out.extend_from_slice(&hex(data));
            out.extend_from_slice(b">\n");
        }
        1 => {
            out.extend_from_slice(&ascii85(data));
            out.push(b'\n');
        }
        2 => {
            out.extend_from_slice(&run_length(data));
            out.push(b'\n');
        }
        3 => {
            let encode =
                jpeg.ok_or_else(|| PsdError::Unsupported("EPS JPEG needs the app encoder".into()))?;
            let bytes = encode(
                data,
                width,
                height,
                components,
                jpeg_quality_percent(quality),
            )
            .ok_or_else(|| PsdError::Invalid("EPS JPEG encode".into()))?;
            out.extend_from_slice(&bytes);
            out.push(b'\n');
        }
        _ => {}
    }
    Ok(out)
}

fn path_postscript(path: &VectorPath, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let y = |v: f64| (height as f64 - v).max(0.0);
    for subpath in &path.subpaths {
        if subpath.points.is_empty() {
            continue;
        }
        let first = subpath.points[0].anchor;
        out.extend_from_slice(format!("{} {} moveto\n", first.0, y(first.1)).as_bytes());
        for index in 0..subpath.segment_count() {
            let Some([_, out_h, in_h, end]) = subpath.segment(index) else {
                continue;
            };
            if out_h == subpath.points[index].anchor && in_h == end {
                out.extend_from_slice(format!("{} {} lineto\n", end.0, y(end.1)).as_bytes());
            } else {
                out.extend_from_slice(
                    format!(
                        "{} {} {} {} {} {} curveto\n",
                        out_h.0,
                        y(out_h.1),
                        in_h.0,
                        y(in_h.1),
                        end.0,
                        y(end.1)
                    )
                    .as_bytes(),
                );
            }
        }
        if subpath.closed {
            out.extend_from_slice(b"closepath\n");
        }
    }
    out
}

fn preview_tiff(raster: &Raster, preview: u8) -> Option<Vec<u8>> {
    match preview {
        1 => Some(super::tiff::preview_one_bit(
            raster.width,
            raster.height,
            &raster.gray,
        )),
        2 => {
            let doc = Document::from_rgba("preview", raster.width, raster.height, &raster.rgba);
            let preview = Raster::from_document(&doc).ok()?;
            let options = SaveOptions {
                tiff_layer_compression: 2,
                tiff_transparency: false,
                ..SaveOptions::default()
            };
            super::tiff::write(&doc, &preview, &options, None).ok()
        }
        _ => None,
    }
}

fn wrap_dos_eps(postscript: &[u8], preview: Option<&[u8]>) -> Vec<u8> {
    let Some(preview) = preview else {
        return postscript.to_vec();
    };
    let ps_offset = 30u32;
    let preview_offset = ps_offset + postscript.len() as u32;
    let mut out = Vec::with_capacity(30 + postscript.len() + preview.len());
    out.extend_from_slice(&[0xC5, 0xD0, 0xD3, 0xC6]);
    out.extend_from_slice(&ps_offset.to_le_bytes());
    out.extend_from_slice(&(postscript.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // WMF preview offset
    out.extend_from_slice(&0u32.to_le_bytes()); // WMF preview length
    out.extend_from_slice(&preview_offset.to_le_bytes());
    out.extend_from_slice(&(preview.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // checksum
    out.extend_from_slice(postscript);
    out.extend_from_slice(preview);
    out
}

fn bounding_box(width: u32, height: u32, doc: &Document) -> (f64, f64) {
    let ppi = crate::document_resolution(doc)
        .map(|resolution| resolution.ppi)
        .filter(|ppi| *ppi > 0.0)
        .unwrap_or(72.0);
    (
        (f64::from(width) * 72.0 / ppi).round(),
        (f64::from(height) * 72.0 / ppi).round(),
    )
}

pub(super) fn write(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let (space, data) = color_space(raster);
    let mut ps = Vec::new();
    ps.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");
    let (box_w, box_h) = bounding_box(raster.width, raster.height, doc);
    ps.extend_from_slice(format!("%%BoundingBox: 0 0 {box_w} {box_h}\n").as_bytes());
    ps.extend_from_slice(b"%%EndComments\n");
    if options.eps_halftone {
        ps.extend_from_slice(
            b"<< /HalftoneType 1 /Frequency 60 /Angle 45 /SpotFunction { pop } >> sethalftone\n",
        );
    }
    if options.eps_transfer {
        ps.extend_from_slice(b"{ } settransfer\n");
    }
    if options.eps_ps_color_management {
        ps.extend_from_slice(b"<< /UseCIEColor true >> setpagedevice\n");
    }
    if !doc.work_path.is_empty() {
        ps.extend_from_slice(b"gsave\nnewpath\n");
        ps.extend_from_slice(&path_postscript(&doc.work_path, raster.height));
        ps.extend_from_slice(b"clip\n");
    }
    let mask = options.eps_transparent_whites && raster.mode == ColorMode::Bitmap;
    let payload = if mask {
        mask_rows(raster.width, raster.height, &raster.gray)
    } else {
        data
    };
    ps.extend_from_slice(&image_operator(
        raster.width,
        raster.height,
        space,
        &payload,
        options.eps_encoding,
        options.eps_jpeg_quality,
        options.eps_interpolation,
        mask,
        jpeg,
    )?);
    if !doc.work_path.is_empty() {
        ps.extend_from_slice(b"grestore\n");
    }
    if options.eps_vector_data {
        let paths: Vec<&VectorPath> = std::iter::once(&doc.work_path)
            .chain(doc.saved_paths.iter().map(|p: &NamedPath| &p.path))
            .collect();
        for path in paths {
            if path.is_empty() {
                continue;
            }
            ps.extend_from_slice(b"newpath\n");
            ps.extend_from_slice(&path_postscript(path, raster.height));
            ps.extend_from_slice(b"stroke\n");
        }
    }
    ps.extend_from_slice(b"showpage\n%%EOF\n");
    let preview = preview_tiff(raster, options.eps_preview);
    Ok(wrap_dos_eps(&ps, preview.as_deref()))
}

fn set_extension(path: &Path, extension: &str) -> PathBuf {
    let mut out = path.to_path_buf();
    out.set_extension(extension);
    out
}

pub(super) fn dcs_cmyk(raster: &Raster) -> Vec<u8> {
    let n = raster.width as usize * raster.height as usize;
    if raster.cmyk.len() == n * 4 {
        raster.cmyk.clone()
    } else {
        super::rgb_to_cmyk_separation(&raster.rgb)
    }
}

fn dcs_separations(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let mut ps = Vec::new();
    ps.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");
    let (box_w, box_h) = bounding_box(raster.width, raster.height, doc);
    ps.extend_from_slice(format!("%%BoundingBox: 0 0 {box_w} {box_h}\n").as_bytes());
    ps.extend_from_slice(b"%%EndComments\n");
    let n = raster.width as usize * raster.height as usize;
    let cmyk = dcs_cmyk(raster);
    for (name, index) in [("C", 0usize), ("M", 1), ("Y", 2), ("K", 3)] {
        ps.extend_from_slice(format!("%%Plate: {name}\n").as_bytes());
        let plane = &cmyk[index * n..(index + 1) * n];
        ps.extend_from_slice(&image_operator(
            raster.width,
            raster.height,
            "DeviceGray",
            plane,
            options.eps_encoding,
            options.eps_jpeg_quality,
            options.eps_interpolation,
            false,
            jpeg,
        )?);
    }
    ps.extend_from_slice(b"showpage\n%%EOF\n");
    let preview = preview_tiff(raster, options.dcs_preview);
    Ok(wrap_dos_eps(&ps, preview.as_deref()))
}

pub(super) fn write_dcs(
    path: &Path,
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<(), PsdError> {
    if options.dcs_version != 1 {
        return write_atomic(path, &dcs_separations(raster, options, doc, jpeg)?);
    }
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut eps_options = options.clone();
    eps_options.eps_preview = options.dcs_preview;
    // DCS's master is a CMYK composite, not the source RGB composite.
    let cmyk = dcs_cmyk(raster);
    let master = Raster {
        mode: ColorMode::Cmyk,
        cmyk: cmyk.clone(),
        ..raster.clone()
    };
    files.push((path.to_path_buf(), write(&master, &eps_options, doc, jpeg)?));
    let n = raster.width as usize * raster.height as usize;
    for (index, ext) in ["C", "M", "Y", "K"].iter().enumerate() {
        let plane = cmyk[index * n..(index + 1) * n].to_vec();
        let plate = Raster {
            width: raster.width,
            height: raster.height,
            mode: ColorMode::Grayscale,
            rgba: plane.iter().flat_map(|&v| [v, v, v, 255]).collect(),
            rgb: plane.iter().flat_map(|&v| [v, v, v]).collect(),
            gray: plane.clone(),
            cmyk: plane.iter().flat_map(|&v| [v, 0, 0, 0]).collect(),
            lab: plane.iter().flat_map(|&v| [v, 128, 128]).collect(),
            alpha: false,
            palette: None,
            indices: None,
        };
        files.push((
            set_extension(path, ext),
            write(&plate, &eps_options, doc, jpeg)?,
        ));
    }
    write_set(files)
}

/// Write a DCS plate set as a unit while preserving any previous set. Every
/// existing target is renamed to a backup first; a failed write removes the new
/// members and restores the backups.
pub(super) fn write_set(files: Vec<(PathBuf, Vec<u8>)>) -> Result<(), PsdError> {
    let mut backups: Vec<(PathBuf, PathBuf)> = Vec::new();
    let restore = |backups: &[(PathBuf, PathBuf)], written: &[PathBuf]| {
        for prior in written {
            let _ = std::fs::remove_file(prior);
        }
        for (target, backup) in backups.iter().rev() {
            let _ = std::fs::rename(backup, target);
        }
    };
    for (target, _) in &files {
        if !target.exists() {
            continue;
        }
        let name = target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dcs");
        let backup = target.with_file_name(format!(
            ".{name}.{:016x}.bak",
            u64::from_le_bytes(crate::os_random::<8>())
        ));
        if std::fs::rename(target, &backup).is_err() {
            restore(&backups, &[]);
            return Err(PsdError::Invalid("DCS backup failed".into()));
        }
        backups.push((target.clone(), backup));
    }
    let mut written: Vec<PathBuf> = Vec::new();
    for (target, bytes) in &files {
        if let Err(e) = write_atomic(target, bytes) {
            restore(&backups, &written);
            return Err(e);
        }
        written.push(target.clone());
    }
    for (_, backup) in backups {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
}
