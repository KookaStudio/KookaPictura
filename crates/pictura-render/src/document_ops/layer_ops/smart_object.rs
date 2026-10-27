//! Convert a raster pixel layer into an embedded smart object.
//!
//! The operation authors a self-contained source document from the layer's
//! raster, serializes it with `write_psd`, and attaches the bytes as an
//! embedded [`SmartObject`]. The layer keeps its pixel channels (the raster
//! proxy), so rendering is unchanged; a layer with a proxy is drawn from the
//! proxy and the embedded source is only decoded for a channel-less layer.

use pictura_core::{
    BitDepth, BlendMode, Channel, Document, Layer, PsdRect, SmartObject, SmartObjectKind,
};

use super::paths::{format_segments, parse_path, resolve_path, resolve_path_mut};

/// Remove the document-level linked-source record for `uuid`, but only when no
/// layer still references it. [`super::create::duplicate_layer`] copies a smart
/// object's `uuid`, so a duplicated layer shares the record; dropping it on one
/// instance's conversion would orphan the sibling on the next save.
fn remove_link_if_unreferenced(doc: &mut Document, uuid: &str) {
    if uuid.is_empty() || uuid_referenced(&doc.layers, uuid) {
        return;
    }
    if let Some(cleaned) =
        pictura_codec::remove_linked_source(&doc.layer_section_extra, uuid, doc.is_psb)
    {
        doc.layer_section_extra = cleaned;
    }
}

fn uuid_referenced(layers: &[Layer], uuid: &str) -> bool {
    layers.iter().any(|layer| {
        layer
            .smart_object
            .as_ref()
            .is_some_and(|so| so.uuid == uuid)
            || uuid_referenced(&layer.children, uuid)
    })
}

/// Whether `path` resolves to a single convertible raster pixel layer: not a
/// group, no adjustment data, no existing smart object, and a `rect` with
/// positive width and height. The Background layer is eligible.
///
/// Also requires an 8-bit document: `write_psd` is 8-bit only (roadmap G4), so a
/// deeper document could not serialize its embedded source.
pub fn can_convert_to_smart_object(doc: &Document, path: &str) -> bool {
    doc.depth == BitDepth::Eight
        && resolve_path(doc, path).is_some_and(|layer| {
            !layer.is_group
                && layer.adjustment.is_none()
                && layer.smart_object.is_none()
                && layer.rect.width() > 0
                && layer.rect.height() > 0
        })
}

/// Convert the raster pixel layer at `path` into an embedded smart object.
///
/// A Background target is first unlocked — flag cleared, locks released, an
/// opaque alpha channel added, renamed to the next free `Layer N` — as part of
/// the same operation, so the result is an ordinary layer. Returns `false`
/// without mutating the document for any ineligible target or when the embedded
/// source cannot be serialized (the unlock is applied only after `write_psd`
/// succeeds).
pub fn convert_to_smart_object(doc: &mut Document, path: &str) -> bool {
    if !can_convert_to_smart_object(doc, path) {
        return false;
    }
    let (payload, name, was_background) = {
        let layer = resolve_path(doc, path).expect("can_convert_to_smart_object checked");
        let w = layer.rect.width() as u32;
        let h = layer.rect.height() as u32;
        let mut embedded = Document::new(w, h, doc.mode, doc.depth);
        let plane = w as usize * h as usize;
        for c in 0..doc.mode.color_channels() as usize {
            if let Some(channel) = layer.channels.iter().find(|ch| ch.id == c as i16) {
                let n = plane.min(channel.data.len());
                embedded.composite.data[c * plane..c * plane + n]
                    .copy_from_slice(&channel.data[..n]);
            }
        }
        let mut copy = layer.clone();
        if copy.background {
            super::create::release_background(&mut copy);
            copy.name = super::create::next_layer_name(doc, "Layer");
        }
        let name = copy.name.clone();
        copy.rect = PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        embedded.layers = pictura_core::LayerStack::from_layers(vec![copy]);
        let Ok(payload) = pictura_codec::write_psd(&embedded) else {
            return false;
        };
        (payload, name, layer.background)
    };
    let layer = resolve_path_mut(doc, path).expect("can_convert_to_smart_object checked");
    if was_background {
        super::create::release_background(layer);
        layer.name = name.clone();
    }
    layer.smart_object = Some(
        {
            let mut so = SmartObject::default();
            so.kind = SmartObjectKind::Embedded;
            so.filename = format!("{name}.psd");
            so.filetype = *b"8BPB";
            so.creator = *b"8BIM";
            so
        }
        .with_payload(payload),
    );
    true
}

/// `Filter > Convert for Smart Filters`: alias of [`convert_to_smart_object`].
///
/// CS6 makes the raster layer smart so filters can attach non-destructively;
/// the underlying conversion is the same operation, so this delegates.
pub fn convert_for_smart_filters(doc: &mut Document, path: &str) -> bool {
    convert_to_smart_object(doc, path)
}

/// Whether `path` resolves to a replaceable smart-object layer: not a group, no
/// adjustment data, and an `Embedded` object with a non-empty payload.
pub fn can_replace_smart_object_contents(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group
            && layer.adjustment.is_none()
            && layer.smart_object.as_ref().is_some_and(|so| {
                so.kind == SmartObjectKind::Embedded && so.payload().is_some_and(|p| !p.is_empty())
            })
    })
}

/// Read the embedded source bytes of the smart-object layer at `path`.
///
/// Returns `Some(payload.clone())` only when the layer is not a group, has no
/// adjustment data, and its `smart_object` has a non-empty payload; every other
/// target returns `None`. Pure read: never mutates the document.
pub fn smart_object_source_bytes(doc: &Document, path: &str) -> Option<Vec<u8>> {
    resolve_path(doc, path).and_then(|layer| {
        if layer.is_group || layer.adjustment.is_some() {
            return None;
        }
        layer
            .smart_object
            .as_ref()
            .and_then(|so| so.payload())
            .filter(|p| !p.is_empty())
            .map(<[u8]>::to_vec)
    })
}

/// Whether `path` resolves to a smart-object layer whose embedded payload
/// parses as a PSD/PSB document, i.e. it can be opened as an in-app editor.
///
/// Pure read: never mutates the document. Requires the same eligible shape as
/// [`can_replace_smart_object_contents`] (non-group, non-adjustment, `Embedded`,
/// non-empty payload); a non-`Embedded` object, or a non-empty payload that is
/// not a parseable PSD/PSB (a placed JPEG), returns `false`.
pub fn can_edit_smart_object_contents(doc: &Document, path: &str) -> bool {
    can_replace_smart_object_contents(doc, path)
        && smart_object_source_bytes(doc, path)
            .is_some_and(|bytes| pictura_codec::read_psd(&bytes).is_ok())
}

/// Replace the embedded source of the smart-object layer at `path`.
///
/// Returns `false` without mutating the document when the target is ineligible
/// or `bytes` do not parse as a PSD/PSB document. On success only the typed
/// source and the pixel channels change: the channels are cleared so the new
/// source renders through the embedded-source path scaled into the existing
/// `rect`, and the preserved config block and matching linked record are dropped
/// so a re-save authors a fresh source rather than re-emitting the old one.
pub fn replace_smart_object_contents(
    doc: &mut Document,
    path: &str,
    filename: &str,
    bytes: &[u8],
) -> bool {
    if !can_replace_smart_object_contents(doc, path) || pictura_codec::read_psd(bytes).is_err() {
        return false;
    }
    let old_uuid = resolve_path(doc, path)
        .and_then(|layer| layer.smart_object.as_ref())
        .map(|so| so.uuid.clone())
        .unwrap_or_default();
    {
        let layer = resolve_path_mut(doc, path).expect("can_replace_smart_object_contents checked");
        layer.channels.clear();
        layer
            .extra_blocks
            .retain(|block| !matches!(&block.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd"));
        let so = layer.smart_object.as_mut().expect("checked");
        so.set_payload(bytes.to_vec());
        so.filename = filename.to_string();
        so.filetype = *b"8BPB";
        so.creator = *b"8BIM";
        so.uuid.clear();
    }
    remove_link_if_unreferenced(doc, &old_uuid);
    true
}

/// Place a PSD/PSB file as a new topmost embedded smart-object layer.
///
/// Decodes `bytes` with [`pictura_codec::read_psd`]; returns `None` without
/// mutating the document when they do not parse. On success appends a
/// channel-less layer named `filename`, sized to the decoded document at
/// `(0, 0)`, carrying an embedded [`SmartObject`] whose payload is `bytes`, and
/// returns the new layer's path.
pub fn place_smart_object(doc: &mut Document, filename: &str, bytes: &[u8]) -> Option<String> {
    let decoded = pictura_codec::read_psd(bytes).ok()?;
    let index = doc.layers.len();
    let layer = Layer {
        name: filename.to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: decoded.height as i32,
            right: decoded.width as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: Vec::new(),
        smart_object: Some(
            {
                let mut so = SmartObject::default();
                so.kind = SmartObjectKind::Embedded;
                so.filename = filename.to_string();
                so.filetype = *b"8BPB";
                so.creator = *b"8BIM";
                so
            }
            .with_payload(bytes.to_vec()),
        ),
        ..Default::default()
    };
    if doc.layers.insert(&[], index, layer).is_err() {
        return None;
    }
    Some(format_segments(&[index]))
}

/// Open `bytes` as a new document holding `filename` as one embedded smart
/// object, or `None` when they do not parse as a PSD/PSB document.
///
/// The new document takes the source's size, mode, and depth, seeds its merged
/// composite from the source, then appends one topmost channel-less native-size
/// layer via [`place_smart_object`]. No source path is carried: the caller
/// decides whether the new document is untitled.
pub fn open_as_smart_object(filename: &str, bytes: &[u8]) -> Option<Document> {
    let src = pictura_codec::read_psd(bytes).ok()?;
    let mut doc = Document::new(src.width, src.height, src.mode, src.depth);
    doc.composite = src.composite;
    place_smart_object(&mut doc, filename, bytes)?;
    Some(doc)
}

/// Whether `path` resolves to a rasterizable smart-object layer: not a group,
/// no adjustment data, and a typed smart object to consume.
pub fn can_rasterize_smart_object(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group && layer.adjustment.is_none() && layer.smart_object.is_some()
    })
}

/// Rasterize the smart-object layer at `path` back into a plain pixel layer.
///
/// A layer with a color channel keeps its raster proxy byte-for-byte; a
/// channel-less layer is materialized from the decoded embedded source into
/// channels `0..mode.color_channels()` plus a `-1` alpha. On success the typed
/// object, the preserved config block, and the matching document-level linked
/// record are dropped so a re-save authors no smart object. Returns `false`
/// without mutating for an ineligible target or an undecodable payload.
pub fn rasterize_smart_object(doc: &mut Document, path: &str) -> bool {
    if !can_rasterize_smart_object(doc, path) {
        return false;
    }
    let rendered = {
        let layer = resolve_path(doc, path).expect("can_rasterize_smart_object checked");
        if layer.channels.iter().any(|c| c.id == 0) {
            None
        } else {
            let so = layer.smart_object.as_ref().expect("checked");
            let Some(src) = crate::render_smart_source(so, layer.rect, layer.rect) else {
                return false;
            };
            let plane = src.width as usize * src.height as usize;
            let mut channels: Vec<Channel> = (0..doc.mode.color_channels() as usize)
                .map(|c| Channel {
                    id: c as i16,
                    data: src.data[c * plane..(c + 1) * plane].to_vec().into(),
                })
                .collect();
            channels.push(Channel {
                id: -1,
                data: src.data[3 * plane..4 * plane].to_vec().into(),
            });
            Some(channels)
        }
    };
    let uuid = resolve_path(doc, path)
        .and_then(|layer| layer.smart_object.as_ref())
        .map(|so| so.uuid.clone())
        .unwrap_or_default();
    {
        let layer = resolve_path_mut(doc, path).expect("checked");
        if let Some(channels) = rendered {
            layer.channels = channels;
        }
        layer
            .extra_blocks
            .retain(|block| !matches!(&block.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd"));
        layer.smart_object = None;
    }
    remove_link_if_unreferenced(doc, &uuid);
    true
}

/// Decode the embedded source of the smart-object layer at `path`, or `None`
/// for any ineligible target or an unparseable payload.
fn embedded_source_doc(doc: &Document, path: &str) -> Option<Document> {
    let bytes = smart_object_source_bytes(doc, path)?;
    pictura_codec::read_psd(&bytes).ok()
}

/// Whether `path` resolves to a smart-object layer whose embedded source parses
/// as a PSD/PSB document with a positive size, so it can be reset.
pub fn can_reset_smart_object_transform(doc: &Document, path: &str) -> bool {
    embedded_source_doc(doc, path).is_some_and(|src| src.width > 0 && src.height > 0)
}

/// Restore the smart object at `path` to the embedded source's native transform.
///
/// Sets the layer's `rect` to `(0, 0, source_width, source_height)` — the
/// source's own frame, the same frame a freshly placed object uses — and clears
/// the pixel channels so the compositor re-renders the embedded source at native
/// scale and rotation. The smart object is kept, so the action is
/// non-destructive. Returns `false` without mutating for an ineligible target or
/// a payload that does not parse as a PSD/PSB document.
pub fn reset_smart_object_transform(doc: &mut Document, path: &str) -> bool {
    let Some(src) = embedded_source_doc(doc, path) else {
        return false;
    };
    if src.width == 0 || src.height == 0 {
        return false;
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    layer.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: src.height as i32,
        right: src.width as i32,
    };
    layer.channels.clear();
    // The mapped frame is native, so the retained native/raw streams no longer
    // match the layer; drop them rather than re-emitting a stale transform.
    layer.raw_channels.clear();
    layer.source_channels = None;
    true
}

/// Whether `path` resolves to a smart-object layer whose embedded source parses
/// as a PSD/PSB document with a positive size, so it can be unpacked.
pub fn can_convert_smart_object_to_layers(doc: &Document, path: &str) -> bool {
    embedded_source_doc(doc, path).is_some_and(|src| src.width > 0 && src.height > 0)
}

/// Replace the smart-object layer at `path` with the layers of its embedded
/// source, mapped into the object's `rect`.
///
/// The source's `(0, 0, w, h)` frame maps to the object's `rect` by a per-axis
/// scale plus the object's origin; pixel channel planes are resampled to the
/// mapped rect, groups recurse, and a source with no layers becomes one raster
/// layer built from its merged composite. The replacement layers take the
/// object's slot and the matching document-level linked record is dropped.
/// Returns `false` without mutating for an ineligible target, an unparseable
/// payload, a zero-size source, or a zero-size object rect.
pub fn convert_smart_object_to_layers(doc: &mut Document, path: &str) -> bool {
    let Some(src) = embedded_source_doc(doc, path) else {
        return false;
    };
    if src.width == 0 || src.height == 0 {
        return false;
    }
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let (obj_rect, uuid) = match resolve_path(doc, path) {
        Some(layer) => (
            layer.rect,
            layer
                .smart_object
                .as_ref()
                .map(|so| so.uuid.clone())
                .unwrap_or_default(),
        ),
        None => return false,
    };
    if obj_rect.width() <= 0 || obj_rect.height() <= 0 {
        return false;
    }
    let sx = obj_rect.width() as f64 / src.width as f64;
    let sy = obj_rect.height() as f64 / src.height as f64;
    let replacement: Vec<Layer> = if src.layers.is_empty() {
        vec![raster_layer_from_composite(&src, obj_rect)]
    } else {
        src.layers
            .iter()
            .map(|layer| map_layer(layer, sx, sy, obj_rect.left, obj_rect.top))
            .collect()
    };
    if doc.layers.replace(&segments, replacement).is_err() {
        return false;
    }
    remove_link_if_unreferenced(doc, &uuid);
    true
}

/// Map a source-document rect into the target frame: origin `(ox, oy)` plus the
/// per-axis scale.
fn map_rect(r: PsdRect, sx: f64, sy: f64, ox: i32, oy: i32) -> PsdRect {
    PsdRect {
        left: ox + (r.left as f64 * sx).round() as i32,
        top: oy + (r.top as f64 * sy).round() as i32,
        right: ox + (r.right as f64 * sx).round() as i32,
        bottom: oy + (r.bottom as f64 * sy).round() as i32,
    }
}

/// Bilinear resample of one plane from `sw × sh` to `dw × dh`; an identical
/// size copies the bytes. A zero destination or source yields an empty plane.
fn resample_plane(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Vec::new();
    }
    if sw == dw && sh == dh {
        return src.to_vec();
    }
    let mut out = vec![0u8; dw * dh];
    for y in 0..dh {
        let ly = (y as f64 + 0.5) * sh as f64 / dh as f64;
        for x in 0..dw {
            let lx = (x as f64 + 0.5) * sw as f64 / dw as f64;
            out[y * dw + x] = super::transform::bilinear(src, sw, sh, lx, ly);
        }
    }
    out
}

/// Deep-copy `layer` and map its rect, channels, mask, and children into the
/// target frame. Nested raw/native streams are dropped (they cannot be mapped).
fn map_layer(layer: &Layer, sx: f64, sy: f64, ox: i32, oy: i32) -> Layer {
    let mut out = layer.clone();
    let src_rect = layer.rect;
    let (sw, sh) = (src_rect.width(), src_rect.height());
    out.rect = map_rect(src_rect, sx, sy, ox, oy);
    let (dw, dh) = (
        out.rect.width().max(0) as usize,
        out.rect.height().max(0) as usize,
    );
    for channel in out.channels.iter_mut() {
        if sw > 0 && sh > 0 && channel.data.len() == (sw as usize) * (sh as usize) {
            channel.data = resample_plane(&channel.data, sw as usize, sh as usize, dw, dh).into();
        } else {
            channel.data = Vec::new().into();
        }
    }
    if let Some(mask) = out.mask.as_mut() {
        let (mw, mh) = (mask.rect.width(), mask.rect.height());
        let new_rect = map_rect(mask.rect, sx, sy, ox, oy);
        mask.rect = new_rect;
        mask.data = mask.data.as_ref().and_then(|data| {
            (mw > 0 && mh > 0 && data.len() == (mw as usize) * (mh as usize)).then(|| {
                resample_plane(
                    data,
                    mw as usize,
                    mh as usize,
                    new_rect.width().max(0) as usize,
                    new_rect.height().max(0) as usize,
                )
                .into()
            })
        });
    }
    if !out.children.is_empty() {
        out.children = pictura_core::LayerStack::from_layers(
            layer
                .children
                .iter()
                .map(|child| map_layer(child, sx, sy, ox, oy))
                .collect(),
        );
    }
    if out.background {
        super::create::release_background(&mut out);
    }
    out.raw_channels.clear();
    out.source_channels = None;
    out
}

/// A single raster layer covering `obj_rect`, resampled from `source`'s merged
/// composite, for an embedded source that carries no layers.
fn raster_layer_from_composite(source: &Document, obj_rect: PsdRect) -> Layer {
    let sw = source.width as usize;
    let sh = source.height as usize;
    let dw = obj_rect.width().max(0) as usize;
    let dh = obj_rect.height().max(0) as usize;
    let plane = sw * sh;
    let mut channels = Vec::new();
    if plane > 0 && source.composite.data.len() >= plane {
        let colors = source.mode.color_channels() as usize;
        for c in 0..colors {
            channels.push(Channel {
                id: c as i16,
                data: resample_plane(
                    &source.composite.data[c * plane..(c + 1) * plane],
                    sw,
                    sh,
                    dw,
                    dh,
                )
                .into(),
            });
        }
        if source.composite.data.len() >= 4 * plane {
            channels.push(Channel {
                id: -1,
                data: resample_plane(&source.composite.data[3 * plane..4 * plane], sw, sh, dw, dh)
                    .into(),
            });
        } else {
            channels.push(Channel {
                id: -1,
                data: vec![255; dw * dh].into(),
            });
        }
    }
    Layer {
        name: "Layer".into(),
        rect: obj_rect,
        channels,
        ..Default::default()
    }
}

/// Whether `path` resolves to a non-group, non-adjustment `Embedded` smart
/// object with a non-empty payload, so an independent copy can be made.
pub fn can_new_smart_object_via_copy(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group
            && layer.adjustment.is_none()
            && layer.smart_object.as_ref().is_some_and(|so| {
                so.kind == SmartObjectKind::Embedded && so.payload().is_some_and(|p| !p.is_empty())
            })
    })
}

/// Duplicate the smart-object layer at `path` with an independent embedded
/// source, returning the copy's path.
///
/// The copy is a deep clone (so its payload is a separate `Vec`), renamed
/// `"<name> copy"`, and its preserved `SoLd`/`SoLE`/`plLd`/`PlLd` config block
/// and `uuid` are cleared so the writer re-authors the copy's own linked record
/// rather than sharing the original's. It is inserted directly above the
/// original. Returns `None` without mutating for any ineligible target.
pub fn new_smart_object_via_copy(doc: &mut Document, path: &str) -> Option<String> {
    if !can_new_smart_object_via_copy(doc, path) {
        return None;
    }
    let segments = parse_path(path)?;
    let mut copy = resolve_path(doc, path)?.clone();
    copy.name = format!("{} copy", copy.name);
    copy.extra_blocks
        .retain(|block| !matches!(&block.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd"));
    if let Some(so) = copy.smart_object.as_mut() {
        so.config_descriptor.clear();
        so.uuid.clear();
    }
    let (&index, parent) = segments.split_last()?;
    if doc.layers.insert(parent, index + 1, copy).is_err() {
        return None;
    }
    let mut new_segments = segments;
    if let Some(last) = new_segments.last_mut() {
        *last = index + 1;
    }
    Some(format_segments(&new_segments))
}
