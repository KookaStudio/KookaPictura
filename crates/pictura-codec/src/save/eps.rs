//! EPS and DCS Save As writers.

use std::path::{Path, PathBuf};

use pictura_core::path::{NamedPath, VectorPath};
use pictura_core::{ColorMode, Document};

use super::{jpeg_quality_percent, write_atomic, BeforeWrite, JpegEncode, Raster, SaveOptions};
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
    scale: f64,
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
    let image_w = f64::from(width) * scale;
    let image_h = f64::from(height) * scale;
    out.extend_from_slice(
        format!(
            "<< /ImageType 1 /Width {width} /Height {height} /BitsPerComponent {} {} /Decode {decode} /ImageMatrix [{image_w} 0 0 -{image_h} 0 {image_h}] /DataSource currentfile {filter} {} >> {} \n",
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

fn path_postscript(path: &VectorPath, height: u32, scale: f64) -> Vec<u8> {
    let mut out = Vec::new();
    let x = |v: f64| v * scale;
    let y = |v: f64| (height as f64 - v) * scale;
    for subpath in &path.subpaths {
        if subpath.points.is_empty() {
            continue;
        }
        let first = subpath.points[0].anchor;
        out.extend_from_slice(format!("{} {} moveto\n", x(first.0), y(first.1)).as_bytes());
        for index in 0..subpath.segment_count() {
            let Some([_, out_h, in_h, end]) = subpath.segment(index) else {
                continue;
            };
            if out_h == subpath.points[index].anchor && in_h == end {
                out.extend_from_slice(format!("{} {} lineto\n", x(end.0), y(end.1)).as_bytes());
            } else {
                out.extend_from_slice(
                    format!(
                        "{} {} {} {} {} {} curveto\n",
                        x(out_h.0),
                        y(out_h.1),
                        x(in_h.0),
                        y(in_h.1),
                        x(end.0),
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

fn page_scale(doc: &Document) -> f64 {
    let ppi = crate::document_resolution(doc)
        .map(|resolution| resolution.ppi)
        .filter(|ppi| *ppi > 0.0)
        .unwrap_or(72.0);
    72.0 / ppi
}

fn bounding_box(width: u32, height: u32, doc: &Document) -> (f64, f64, f64) {
    let scale = page_scale(doc);
    (
        (f64::from(width) * scale).round(),
        (f64::from(height) * scale).round(),
        scale,
    )
}

/// Emit the document's designated clipping path, if resource 2999 names a
/// saved path that still exists. Returns whether a `gsave` is waiting for a
/// matching `grestore`.
fn append_clip(ps: &mut Vec<u8>, doc: &Document, height: u32, scale: f64) -> bool {
    let Some(clip) = crate::path_resources::clipping_path(doc) else {
        return false;
    };
    if clip.path.is_empty() {
        return false;
    }
    ps.extend_from_slice(b"gsave\nnewpath\n");
    ps.extend_from_slice(&path_postscript(&clip.path, height, scale));
    ps.extend_from_slice(b"clip\n");
    true
}

fn build_eps(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
    extra_header: &[String],
) -> Result<Vec<u8>, PsdError> {
    let (space, data) = color_space(raster);
    let mut ps = Vec::new();
    ps.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");
    let (box_w, box_h, scale) = bounding_box(raster.width, raster.height, doc);
    ps.extend_from_slice(format!("%%BoundingBox: 0 0 {box_w} {box_h}\n").as_bytes());
    for line in extra_header {
        ps.extend_from_slice(line.as_bytes());
        ps.push(b'\n');
    }
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
    let clipped = append_clip(&mut ps, doc, raster.height, scale);
    let mask = options.eps_transparent_whites && raster.mode == ColorMode::Bitmap;
    let payload = if mask {
        mask_rows(raster.width, raster.height, &raster.gray)
    } else {
        data
    };
    ps.extend_from_slice(&image_operator(
        raster.width,
        raster.height,
        scale,
        space,
        &payload,
        options.eps_encoding,
        options.eps_jpeg_quality,
        options.eps_interpolation,
        mask,
        jpeg,
    )?);
    if clipped {
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
            ps.extend_from_slice(&path_postscript(path, raster.height, scale));
            ps.extend_from_slice(b"stroke\n");
        }
    }
    ps.extend_from_slice(b"showpage\n%%EOF\n");
    Ok(ps)
}

pub(super) fn write(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    write_with_header(raster, options, doc, jpeg, &[])
}

fn write_with_header(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
    extra_header: &[String],
) -> Result<Vec<u8>, PsdError> {
    let ps = build_eps(raster, options, doc, jpeg, extra_header)?;
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

fn gray_plate(raster: &Raster, plane: &[u8]) -> Raster {
    Raster {
        width: raster.width,
        height: raster.height,
        mode: ColorMode::Grayscale,
        rgba: plane.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        rgb: plane.iter().flat_map(|&v| [v, v, v]).collect(),
        gray: plane.to_vec(),
        cmyk: plane.iter().flat_map(|&v| [v, 0, 0, 0]).collect(),
        lab: plane.iter().flat_map(|&v| [v, 128, 128]).collect(),
        alpha: false,
        palette: None,
        indices: None,
    }
}

/// DCS 2.0's single-file form: the composite first, then one complete EPS
/// separation per plate, with `%%PlateFile` records pointing at each payload.
fn dcs_2_file(
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let n = raster.width as usize * raster.height as usize;
    let cmyk = dcs_cmyk(raster);
    let master = Raster {
        mode: ColorMode::Cmyk,
        cmyk: cmyk.clone(),
        ..raster.clone()
    };
    let names = [
        ("Cyan", 0usize),
        ("Magenta", 1),
        ("Yellow", 2),
        ("Black", 3),
    ];
    let mut header = vec!["%%DocumentProcessColors: Cyan Magenta Yellow Black".to_string()];
    for (name, _) in names {
        header.push(format!("%%PlateFile: ({name}) EPS #0000000000 0000000000"));
    }
    let mut main = build_eps(&master, options, doc, jpeg, &header)?;
    let mut plate_options = options.clone();
    plate_options.eps_preview = 0;
    plate_options.eps_vector_data = false;
    let mut plates = Vec::with_capacity(names.len());
    for (_, index) in names {
        let plate = gray_plate(raster, &cmyk[index * n..(index + 1) * n]);
        plates.push(build_eps(&plate, &plate_options, doc, jpeg, &[])?);
    }
    let preview = preview_tiff(raster, options.dcs_preview);
    let dir_offset = if preview.is_some() { 30usize } else { 0 };
    let mut offset = dir_offset + main.len();
    for ((name, _), plate) in names.iter().zip(&plates) {
        let old = format!("%%PlateFile: ({name}) EPS #0000000000 0000000000");
        let new = format!(
            "%%PlateFile: ({name}) EPS #{offset:010} {:010}",
            plate.len()
        );
        let at = main
            .windows(old.len())
            .position(|window| window == old.as_bytes())
            .ok_or_else(|| PsdError::Invalid("DCS plate record".into()))?;
        main[at..at + old.len()].copy_from_slice(new.as_bytes());
        offset += plate.len();
    }
    for plate in plates {
        main.extend_from_slice(&plate);
    }
    Ok(wrap_dos_eps(&main, preview.as_deref()))
}

#[cfg(test)]
pub(super) fn write_dcs(
    path: &Path,
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<(), PsdError> {
    write_dcs_guarded(path, raster, options, doc, jpeg, None)
}

pub(super) fn write_dcs_guarded(
    path: &Path,
    raster: &Raster,
    options: &SaveOptions,
    doc: &Document,
    jpeg: Option<&JpegEncode<'_>>,
    before_write: Option<&BeforeWrite<'_>>,
) -> Result<(), PsdError> {
    let check = || before_write.is_none_or(|guard| guard());
    if options.dcs_version != 1 {
        let bytes = dcs_2_file(raster, options, doc, jpeg)?;
        if !check() {
            return Err(PsdError::Invalid("save was interrupted".into()));
        }
        return write_atomic(path, &bytes);
    }
    let n = raster.width as usize * raster.height as usize;
    let cmyk = dcs_cmyk(raster);
    let mut eps_options = options.clone();
    eps_options.eps_preview = options.dcs_preview;
    // DCS's master is a CMYK composite, not the source RGB composite.
    let master = Raster {
        mode: ColorMode::Cmyk,
        cmyk: cmyk.clone(),
        ..raster.clone()
    };
    let mut header = vec!["%%DocumentProcessColors: Cyan Magenta Yellow Black".to_string()];
    for (name, ext) in [
        ("Cyan", "C"),
        ("Magenta", "M"),
        ("Yellow", "Y"),
        ("Black", "K"),
    ] {
        header.push(format!(
            "%%{name}Plate: {}",
            set_extension(path, ext).display()
        ));
    }
    let mut files: Vec<(PathBuf, Vec<u8>)> = vec![(
        path.to_path_buf(),
        write_with_header(&master, &eps_options, doc, jpeg, &header)?,
    )];
    let mut plate_options = options.clone();
    plate_options.eps_preview = 0;
    plate_options.eps_vector_data = false;
    for (index, ext) in ["C", "M", "Y", "K"].iter().enumerate() {
        let plate = gray_plate(raster, &cmyk[index * n..(index + 1) * n]);
        files.push((
            set_extension(path, ext),
            write(&plate, &plate_options, doc, jpeg)?,
        ));
    }
    if !check() {
        return Err(PsdError::Invalid("save was interrupted".into()));
    }
    write_set(files)
}

/// Write a DCS plate set as a unit while preserving any previous set. Every
/// existing target is renamed to a backup first; a failed write removes the new
/// members and restores the backups. A directory at any target is refused
/// before any member moves.
pub(super) fn write_set(files: Vec<(PathBuf, Vec<u8>)>) -> Result<(), PsdError> {
    for (target, _) in &files {
        if target.exists() && !target.is_file() {
            return Err(PsdError::Invalid("DCS target is not a file".into()));
        }
    }
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
