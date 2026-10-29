//! Embedded-ICC normalization on read.
//!
//! The engine works in sRGB; a file whose pixels are in another ICC space must
//! be converted on load. This pass runs after the mode/depth `normalize`, finds
//! the profile resource, and converts the composite and every layer's color
//! channels with lcms2 (`pictura-color`), including the retained native-depth
//! planes. It then removes the stale profile from the preserved resources so a
//! save is not mis-tagged.
//!
//! ponytail: "is sRGB" is the profile's `Description` containing "srgb" (lcms2
//! has no profile compare); a differently-named sRGB profile is converted within
//! a rounding LSB. Rendering intent is fixed at relative-colorimetric.

use pictura_color::{
    convert, dither_plane, ByteTransform, ConversionOptions, Intent, Policy, Profile,
};
use pictura_core::{ColorMode, Document, PixelBuffer, Plane};
use rayon::prelude::*;

use std::borrow::Cow;
use std::cell::RefCell;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Test-only switch that routes [`convert_planes_with`] through its serial
/// branch, so a test can compare both on the same buffer.
#[cfg(test)]
static FORCE_SERIAL: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static PARALLEL_RUNS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn serial() -> bool {
    let forced = FORCE_SERIAL.load(Ordering::Relaxed);
    if !forced {
        PARALLEL_RUNS.fetch_add(1, Ordering::Relaxed);
    }
    forced
}

#[cfg(not(test))]
#[inline]
fn serial() -> bool {
    false
}

/// Pixels per parallel chunk in [`convert_planes_with`]. Chunks are whole
/// pixels, because lcms2 asserts a `u8` slice's length is a multiple of the
/// pixel format's bytes per pixel.
const CHUNK_PIXELS: usize = 4096;

use crate::common::{MODE_CMYK, MODE_GRAYSCALE, MODE_LAB, MODE_RGB};
use crate::icc_convert::{convert_layers, convert_native_document};
use crate::image_resources::{
    decode_image_resources_with_len, encode_image_resources, frame_image_resource, ImageResource,
    ICC_PROFILE,
};

/// The `Description` of an ICC profile, or `None` when it does not parse.
pub fn profile_description(icc: &[u8]) -> Option<String> {
    Profile::from_icc(icc).ok()?.description()
}

/// Convert a non-sRGB RGB document to the sRGB working space and drop its
/// profile. A non-RGB document, or a profile the engine cannot transform, is
/// left untouched so the pixels and the resource stay consistent.
#[cfg(test)]
pub(crate) fn apply_icc(doc: Document) -> Document {
    apply_icc_with_options(doc, LEGACY_OPTIONS)
}

fn apply_icc_with_options(doc: Document, options: ConversionOptions) -> Document {
    if doc.mode != ColorMode::Rgb || doc.image_resources.is_empty() {
        return doc;
    }
    let (resources, consumed) = decode_image_resources_with_len(&doc);
    let Some(profile) = resources.iter().find(|r| r.id == ICC_PROFILE) else {
        return doc;
    };
    let Ok(src) = Profile::from_icc(&profile.data) else {
        return doc;
    };
    if src.is_srgb() {
        return doc;
    }
    let dst = Profile::srgb();
    // The composite transform decides: if it cannot run (for example a CMYK
    // profile on an RGB document), leave the whole document untouched rather
    // than strip a profile whose pixels were not converted.
    let Some(composite) = convert_buffer(&doc.composite, &src, &dst) else {
        return doc;
    };
    let profile_bytes = profile.data.clone();
    let mut doc = doc;
    doc.composite = composite;
    convert_layers(&mut doc.layers, &src, &dst, options, true);
    convert_native_document(&mut doc, &src, &dst, options);
    doc.source_icc = Some(profile_bytes.into());
    let section = rebuild_resources(&doc, resources, consumed, None);
    doc.image_resources = section.into();
    doc
}

/// Apply the incoming-profile [`Policy`] to a freshly read document.
///
/// `Convert` is the historical normalisation ([`apply_icc`]); `Preserve` and
/// `Off` leave the pixel bytes alone and only decide the profile tagging.
#[cfg(test)]
pub(crate) fn apply_icc_policy(doc: Document, policy: Policy) -> Document {
    apply_icc_policy_with_options(doc, policy, LEGACY_OPTIONS)
}

/// [`apply_icc_policy`] with the caller's conversion options.
pub(crate) fn apply_icc_policy_with_options(
    doc: Document,
    policy: Policy,
    options: ConversionOptions,
) -> Document {
    // A normalized source mode (CMYK/Lab/Indexed/...) has already had its
    // profile pass; the RGB profile pass must not reinterpret its pixels.
    if doc.source_mode.is_some() {
        return doc;
    }
    match policy {
        Policy::Convert => apply_icc_with_options(doc, options),
        Policy::Preserve => apply_passthrough_policy(doc, true),
        Policy::Off => apply_passthrough_policy(doc, false),
    }
}

/// Preserve and Off share the profile lookup [`apply_icc`] does, but neither
/// transforms pixels, so a profile the engine cannot transform is still honoured.
fn apply_passthrough_policy(mut doc: Document, preserve: bool) -> Document {
    if doc.mode != ColorMode::Rgb || doc.image_resources.is_empty() {
        return doc;
    }
    let (resources, consumed) = decode_image_resources_with_len(&doc);
    let Some(profile) = resources.iter().find(|r| r.id == ICC_PROFILE) else {
        return doc;
    };
    let Ok(src) = Profile::from_icc(&profile.data) else {
        return doc;
    };
    if src.is_srgb() {
        return doc;
    }
    if preserve {
        // A profile the normalized RGB pixels no longer match (a CMYK resource on
        // a document normalize converted to RGB) cannot drive a display transform;
        // leave the document byte-unchanged, as Convert does.
        if convert_buffer(&doc.composite, &src, &Profile::srgb()).is_none() {
            return doc;
        }
        // The embedded bytes stay resource 1039 and drive the display
        // conversion; `source_icc` records only a conversion the read threw away.
        doc.document_icc = Some(profile.data.clone().into());
    } else {
        doc.image_resources = rebuild_resources(&doc, resources, consumed, None).into();
    }
    doc
}

/// Convert a buffer's color planes (the first 3 for an RGB/RGBA one, leaving any
/// alpha plane untouched), or `None` when there is no convertible color plane or
/// the transform cannot run; the buffer is never partially converted.
fn convert_buffer(buf: &PixelBuffer, src: &Profile, dst: &Profile) -> Option<PixelBuffer> {
    convert_buffer_with_options(buf, src, dst, LEGACY_OPTIONS)
}

fn convert_buffer_with_options(
    buf: &PixelBuffer,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) -> Option<PixelBuffer> {
    let color = match buf.channels {
        3 | 4 => 3,
        _ => return None,
    };
    let n = buf.pixel_count();
    if n == 0 {
        return None;
    }
    let planes: Vec<Vec<u8>> = (0..color)
        .map(|ch| buf.data[ch * n..(ch + 1) * n].to_vec())
        .collect();
    let converted = convert_planes_with_options(&planes, src, dst, options)?;
    let mut out = buf.clone();
    for (ch, plane) in converted.into_iter().enumerate() {
        out.data[ch * n..(ch + 1) * n].copy_from_slice(&plane);
    }
    Some(out)
}

/// Rebuild an image-resource section: every parsed block except 1039 (with an
/// optional freshly framed 1039 appended), followed by the unparsed tail the
/// decoder did not consume, byte-for-byte.
fn rebuild_resources(
    doc: &Document,
    resources: Vec<ImageResource>,
    consumed: usize,
    new_icc: Option<&[u8]>,
) -> Vec<u8> {
    let mut kept: Vec<ImageResource> = resources
        .into_iter()
        .filter(|r| r.id != ICC_PROFILE)
        .collect();
    if let Some(bytes) = new_icc {
        kept.push(frame_image_resource(ICC_PROFILE, "", bytes));
    }
    let mut section = encode_image_resources(&kept);
    section.extend_from_slice(&doc.image_resources[consumed..]);
    section
}

/// The raw bytes of image resource 1039, if present.
pub(crate) fn icc_resource_bytes(doc: &Document) -> Option<Vec<u8>> {
    if doc.image_resources.is_empty() {
        return None;
    }
    let (resources, _) = decode_image_resources_with_len(doc);
    resources
        .into_iter()
        .find(|r| r.id == ICC_PROFILE)
        .map(|r| r.data)
}

/// The embedded resource 1039 profile when it parses as a four-channel CMYK
/// profile; `None` for a missing, malformed, or non-CMYK resource.
pub(crate) fn cmyk_profile_from_resources(doc: &Document) -> Option<Profile> {
    if doc.image_resources.is_empty() {
        return None;
    }
    let (resources, _) = decode_image_resources_with_len(doc);
    let profile = resources.iter().find(|r| r.id == ICC_PROFILE)?;
    let parsed = Profile::from_icc(&profile.data).ok()?;
    (parsed.channels() == Some(4)).then_some(parsed)
}

/// Replace or remove resource 1039 while preserving every other resource and
/// the unparsed tail byte-for-byte.
pub(crate) fn set_icc_resource(doc: &mut Document, icc: Option<&[u8]>) {
    let (resources, consumed) = decode_image_resources_with_len(doc);
    doc.image_resources = rebuild_resources(doc, resources, consumed, icc).into();
}

/// Remove image resource 1039 without touching any other resource.
pub fn remove_document_profile(doc: &mut Document) {
    set_icc_resource(doc, None);
}

/// Attach a CMYK profile to a CMYK source document's resource 1039.
pub fn assign_cmyk_profile(doc: &mut Document, profile: &Profile) {
    let icc = profile.to_icc();
    set_icc_resource(doc, Some(&icc));
}

/// The image-resource section to emit for an output header mode `mode_code`.
///
/// A resource `1039` whose ICC data-space signature (header bytes 16..20) does
/// not match `mode_code`'s expected space is dropped, so an RGB save of a
/// document normalized from a CMYK/Lab source is not mis-tagged. A matching
/// profile, a profile too short to carry the signature, every other resource,
/// and the unparsed tail are byte-identical; a section with no mismatching
/// `1039` is returned unchanged.
pub(crate) fn resources_for_output(doc: &Document, mode_code: u16) -> Vec<u8> {
    let Some(expected) = output_data_space(mode_code) else {
        return doc.image_resources.to_vec();
    };
    let (resources, consumed) = decode_image_resources_with_len(doc);
    let mismatched = |r: &ImageResource| {
        r.id == ICC_PROFILE
            && r.data
                .get(16..20)
                .is_some_and(|sig| sig != expected.as_slice())
    };
    if !resources.iter().any(mismatched) {
        return doc.image_resources.to_vec();
    }
    let kept: Vec<ImageResource> = resources.into_iter().filter(|r| !mismatched(r)).collect();
    let mut section = encode_image_resources(&kept);
    section.extend_from_slice(&doc.image_resources[consumed..]);
    section
}

/// The ICC data-space signature the output header mode must carry, or `None`
/// for a mode with no mapped ICC space (the section is then left unchanged).
fn output_data_space(mode_code: u16) -> Option<[u8; 4]> {
    Some(match mode_code {
        MODE_RGB => *b"RGB ",
        MODE_GRAYSCALE => *b"GRAY",
        MODE_CMYK => *b"CMYK",
        MODE_LAB => *b"Lab ",
        _ => return None,
    })
}

/// The historical conversion settings: relative colorimetric, no black-point
/// compensation, no dither. Kept for the read/display paths so existing pixel
/// expectations do not move.
pub(crate) const LEGACY_OPTIONS: ConversionOptions = ConversionOptions {
    intent: Intent::RelativeColorimetric,
    black_point_compensation: false,
    dither: false,
};

/// Interleave 1 or 3 equal-length planes, convert, and split back. The planes
/// are treated as a `N x 1` image; the lcms2 transform is per-pixel.
#[cfg(test)]
pub(super) fn convert_planes(
    planes: &[Vec<u8>],
    src: &Profile,
    dst: &Profile,
) -> Option<Vec<Vec<u8>>> {
    convert_planes_with_options(planes, src, dst, LEGACY_OPTIONS)
}

/// [`convert_planes`] with explicit intent/BPC/dither. Dither is applied to
/// the 8-bit output planes and is deterministic.
pub(super) fn convert_planes_with_options(
    planes: &[Vec<u8>],
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) -> Option<Vec<Vec<u8>>> {
    let channels = planes.len();
    if channels != 1 && channels != 3 {
        return None;
    }
    let n = planes[0].len();
    if n == 0 || planes.iter().any(|p| p.len() != n) {
        return None;
    }
    let mut packed = vec![0u8; n * channels];
    for (ch, plane) in planes.iter().enumerate() {
        for (i, value) in plane.iter().enumerate() {
            packed[i * channels + ch] = *value;
        }
    }
    let out = convert(
        src,
        dst,
        &packed,
        n as u32,
        1,
        channels as u8,
        8,
        options.intent,
        options.black_point_compensation,
    )
    .ok()?;
    let mut converted = vec![vec![0u8; n]; channels];
    for (ch, plane) in converted.iter_mut().enumerate() {
        for (i, slot) in plane.iter_mut().enumerate() {
            *slot = out[i * channels + ch];
        }
        if options.dither {
            dither_plane(plane, n);
        }
    }
    Some(converted)
}

/// Assign Profile: retag the document with `target` without touching pixels.
///
/// Rewrites `image_resources` so it carries a framed resource 1039 with
/// `target`'s ICC bytes, or omits 1039 for the sRGB working space, preserving
/// every other resource block (and any unparsed tail) byte-for-byte. Sets
/// `document_icc` to match. Only an RGB document is retagged.
pub fn assign_document_profile(doc: &mut Document, target: &Profile) {
    if doc.mode != ColorMode::Rgb {
        return;
    }
    let icc = if target.is_srgb() {
        None
    } else {
        Some(target.to_icc())
    };
    let (resources, consumed) = decode_image_resources_with_len(doc);
    let section = rebuild_resources(doc, resources, consumed, icc.as_deref());
    doc.image_resources = section.into();
    doc.document_icc = icc.map(Into::into);
}

/// Convert to Profile: transform the composite and every layer's color channels
/// from the document's working profile (`sRGB` when `document_icc` is `None`) to
/// `dst`, then assign `dst`.
///
/// A channel the transform cannot convert (a partial layer) is left untouched,
/// not treated as an error. Returns `false` without mutating for a non-RGB
/// document or when the recorded working profile cannot be parsed.
pub fn convert_document(doc: &mut Document, dst: &Profile) -> bool {
    convert_document_with(doc, dst, LEGACY_OPTIONS)
}

/// [`convert_document`] with explicit conversion options.
pub fn convert_document_with(
    doc: &mut Document,
    dst: &Profile,
    options: ConversionOptions,
) -> bool {
    if doc.mode != ColorMode::Rgb {
        return false;
    }
    let src = match doc.document_icc.as_deref() {
        None => Profile::srgb(),
        Some(icc) => match Profile::from_icc(icc) {
            Ok(profile) => profile,
            Err(_) => return false,
        },
    };
    if let Some(composite) = convert_buffer_with_options(&doc.composite, &src, dst, options) {
        doc.composite = composite;
    }
    // A recorded source mode (CMYK/Lab) keeps its native store in that mode,
    // not RGB; only the 8-bit working channels are RGB and safe to convert.
    convert_layers(
        &mut doc.layers,
        &src,
        dst,
        options,
        doc.source_mode.is_none(),
    );
    convert_native_document(doc, &src, dst, options);
    assign_document_profile(doc, dst);
    true
}

/// Convert a document-space buffer to the sRGB working space for display.
///
/// Borrows the input unchanged when the document has no working profile (the
/// sRGB default) or records one that cannot be parsed, so an unprofiled document
/// displays exactly as before without an extra full-resolution copy. A profiled
/// document reuses the parsed profile and its transform across refreshes, keyed
/// on `document_icc`, so changing the working profile rebuilds it.
pub fn buffer_to_srgb<'a>(doc: &Document, buf: &'a PixelBuffer) -> Cow<'a, PixelBuffer> {
    let Some(icc) = doc.document_icc.as_deref() else {
        return Cow::Borrowed(buf);
    };
    if buf.channels != 3 && buf.channels != 4 {
        return Cow::Borrowed(buf);
    }
    let n = buf.pixel_count();
    if n == 0 {
        return Cow::Borrowed(buf);
    }
    DISPLAY_TRANSFORM.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().map(|c| c.icc.as_slice()) != Some(icc) {
            *slot = Some(CachedDisplayTransform {
                icc: icc.to_vec(),
                transform: build_display_transform(icc),
            });
        }
        match slot.as_ref().and_then(|c| c.transform.as_ref()) {
            Some(transform) => Cow::Owned(convert_planes_with(transform, buf, n)),
            None => Cow::Borrowed(buf),
        }
    })
}

/// A parsed working profile and its sRGB display transform, or `None` when the
/// profile is sRGB/unparseable and no conversion is needed.
struct CachedDisplayTransform {
    icc: Vec<u8>,
    transform: Option<ByteTransform>,
}

thread_local! {
    static DISPLAY_TRANSFORM: RefCell<Option<CachedDisplayTransform>> = const { RefCell::new(None) };
}

fn build_display_transform(icc: &[u8]) -> Option<ByteTransform> {
    let src = Profile::from_icc(icc).ok()?;
    if src.is_srgb() {
        return None;
    }
    ByteTransform::new(
        &src,
        &Profile::srgb(),
        3,
        Intent::RelativeColorimetric,
        false,
    )
    .ok()
}

/// Convert a buffer's three color planes (leaving any alpha plane untouched)
/// through an already-built transform. The transform is reentrant, so the
/// pixels are interleaved, converted and de-interleaved in parallel chunks, and
/// the result is built without forking the whole buffer.
fn convert_planes_with(transform: &ByteTransform, buf: &PixelBuffer, n: usize) -> PixelBuffer {
    debug_assert_eq!(
        transform.channels(),
        3,
        "the interleave and chunking below assume an RGB transform"
    );
    let (r, g, b) = (&buf.data[..n], &buf.data[n..2 * n], &buf.data[2 * n..3 * n]);

    let mut packed = vec![0u8; n * 3];
    let interleave = |chunk: &mut [u8], start: usize| {
        for (k, pixel) in chunk.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let i = start + k;
            pixel[0] = r[i];
            pixel[1] = g[i];
            pixel[2] = b[i];
        }
    };
    if serial() {
        interleave(&mut packed, 0);
    } else {
        packed
            .par_chunks_mut(CHUNK_PIXELS * 3)
            .enumerate()
            .for_each(|(chunk, bytes)| interleave(bytes, chunk * CHUNK_PIXELS));
    }

    let mut out_packed = vec![0u8; n * 3];
    if serial() {
        transform.apply(&packed, &mut out_packed);
    } else {
        packed
            .par_chunks(CHUNK_PIXELS * 3)
            .zip(out_packed.par_chunks_mut(CHUNK_PIXELS * 3))
            .for_each(|(source, out)| transform.apply(source, out));
    }

    let channels = buf.channels as usize;
    let alpha = (channels == 4).then(|| buf.data[3 * n..].to_vec());
    let data = Plane::build(n * channels, |data| {
        let (r, rest) = data.split_at_mut(n);
        let (g, rest) = rest.split_at_mut(n);
        let (b, a) = rest.split_at_mut(n);
        let deinterleave = |r: &mut [u8], g: &mut [u8], b: &mut [u8], start: usize| {
            for (k, (rv, gv)) in r.iter_mut().zip(g.iter_mut()).enumerate() {
                let i = (start + k) * 3;
                *rv = out_packed[i];
                *gv = out_packed[i + 1];
            }
            for (k, bv) in b.iter_mut().enumerate() {
                *bv = out_packed[(start + k) * 3 + 2];
            }
        };
        if serial() {
            deinterleave(r, g, b, 0);
        } else {
            r.par_chunks_mut(CHUNK_PIXELS)
                .zip(g.par_chunks_mut(CHUNK_PIXELS))
                .zip(b.par_chunks_mut(CHUNK_PIXELS))
                .enumerate()
                .for_each(|(chunk, ((r, g), b))| deinterleave(r, g, b, chunk * CHUNK_PIXELS));
        }
        if let Some(alpha) = alpha {
            a.copy_from_slice(&alpha);
        }
    });
    PixelBuffer {
        width: buf.width,
        height: buf.height,
        channels: buf.channels,
        data,
    }
}

#[cfg(test)]
mod tests;
