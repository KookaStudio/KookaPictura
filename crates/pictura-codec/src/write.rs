use pictura_core::*;
use std::borrow::Cow;

use crate::common::*;
use crate::depth::{apply_prediction, depth_bits, depth_of, output_depth, row_bytes};
use crate::error::PsdError;
use crate::write_indexed::{index_layer_plane, writes_indexed};

// ---------------------------------------------------------------------------
// Layer and mask information section — writing
// ---------------------------------------------------------------------------

/// A flat record in on-disk order, borrowing the model layer it came from.
struct OutRecord<'a> {
    layer: Option<&'a Layer>,
    section: u32,
    name: &'a str,
}

/// A layer channel to emit. Both variants are a complete on-disk stream
/// (compression word included): `Encoded` is engine-authored PackBits RLE,
/// `Verbatim` is a preserved `Layer.raw_channels` stream re-emitted unchanged.
enum OutChannel {
    Encoded(Vec<u8>),
    Verbatim(Vec<u8>),
}

impl OutChannel {
    fn bytes(&self) -> &[u8] {
        match self {
            OutChannel::Encoded(data) | OutChannel::Verbatim(data) => data,
        }
    }

    fn declared_len(&self) -> u64 {
        self.bytes().len() as u64
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.bytes());
    }
}

fn flatten(layers: &[Layer]) -> Vec<OutRecord<'_>> {
    enum Frame<'a> {
        Visit(&'a Layer),
        Group(&'a Layer),
    }
    let mut out = Vec::new();
    let mut stack: Vec<Frame> = layers.iter().rev().map(Frame::Visit).collect();
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Visit(layer) if layer.is_group => {
                out.push(OutRecord {
                    layer: None,
                    section: SECTION_DIVIDER,
                    name: DIVIDER_NAME,
                });
                stack.push(Frame::Group(layer));
                for child in layer.children.iter().rev() {
                    stack.push(Frame::Visit(child));
                }
            }
            Frame::Visit(layer) => out.push(OutRecord {
                layer: Some(layer),
                section: 0,
                name: record_name(layer),
            }),
            Frame::Group(layer) => out.push(OutRecord {
                layer: Some(layer),
                section: SECTION_OPEN_FOLDER,
                name: record_name(layer),
            }),
        }
    }
    out
}

/// A flagged layer is written under the PSD `"Background"` name convention
/// (design D5); the flag itself has no PSD bit.
fn record_name(layer: &Layer) -> &str {
    if layer.background {
        "Background"
    } else {
        &layer.name
    }
}

/// The Lab bytes to emit for a Lab output's composite color planes: the
/// retained Lab planes exactly when every plane's forward RGB conversion still
/// matches the working composite, else the working RGB re-encoded with the
/// approximate inverse. The document is never mutated. The retained plane is
/// the 8-bit Lab read store at depth 8, or the native store narrowed to 8-bit
/// at 16/32; for an RGB/Grayscale document there is none.
///
/// ponytail: the three Lab planes are re-encoded as a unit, so editing one
/// channel also re-encodes its unchanged siblings (Lab `a`/`b` depend on all of
/// R/G/B). Per-channel change tracking would be the upgrade if that matters.
fn lab_composite_planes(doc: &Document, depth: u16, plane: usize) -> Vec<u8> {
    let color_channels = doc.composite.channels as usize;
    let current = &doc.composite.data[..color_channels * plane];
    let retained: Option<Vec<Vec<u8>>> = (0..color_channels)
        .map(|c| crate::color_mode::composite_retained_8(doc, depth, c))
        .collect();
    if let Some(retained) = retained {
        let retained: Vec<u8> = retained.concat();
        if retained.len() == plane * color_channels
            && crate::color_mode::lab_to_rgb(&retained).as_slice() == current
        {
            return retained;
        }
    }
    crate::color_mode::rgb_to_lab(current)
}

/// The Lab planes to emit for a layer's three color channels, aligned to
/// `layer.channels` (a `None` entry for every non-color channel). Prefers the
/// retained Lab plane when its forward conversion still matches the working
/// planes; empty when the layer does not carry exactly three id-0/1/2 color
/// channels of equal length, letting the caller keep the original planes.
///
/// ponytail: same grouped ceiling as [`lab_composite_planes`] — one edited
/// channel re-encodes all three.
fn lab_layer_color_planes(layer: &Layer, depth: u16) -> Vec<Option<Vec<u8>>> {
    let positions: Vec<usize> = layer
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0 && c.id < 3)
        .map(|(i, _)| i)
        .collect();
    if positions.len() != 3 {
        return Vec::new();
    }
    let plane = layer.channels[positions[0]].data.len();
    if positions
        .iter()
        .any(|&i| layer.channels[i].data.len() != plane)
    {
        return Vec::new();
    }
    let mut current = Vec::with_capacity(plane * 3);
    for &i in &positions {
        current.extend_from_slice(&layer.channels[i].data);
    }
    let retained: Option<Vec<Vec<u8>>> = positions
        .iter()
        .map(|&i| crate::color_mode::layer_retained_8(layer, depth, layer.channels[i].id))
        .collect();
    let lab = match retained.map(|planes| planes.concat()) {
        Some(retained)
            if retained.len() == plane * 3
                && crate::color_mode::lab_to_rgb(&retained).as_slice() == current.as_slice() =>
        {
            retained
        }
        _ => crate::color_mode::rgb_to_lab(&current),
    };
    let mut out = vec![None; layer.channels.len()];
    for (c, &i) in positions.iter().enumerate() {
        out[i] = Some(lab[c * plane..(c + 1) * plane].to_vec());
    }
    out
}

/// The CMYK bytes to emit for a CMYK output's composite color planes: the four
/// retained source planes exactly when their forward RGB conversion still matches
/// the working composite, else the working RGB re-encoded with the exact
/// [`crate::color_mode::rgb_to_cmyk`] right-inverse. The document is never
/// mutated. The retained plane is the 8-bit CMYK read store at depth 8, or the
/// native store narrowed to 8-bit at 16/32; for an RGB/Grayscale document there
/// is none.
///
/// ponytail: the four planes are re-encoded as a unit, so an edited channel also
/// re-encodes its unchanged siblings. Per-channel change tracking would be the
/// upgrade if that matters.
fn cmyk_composite_planes(doc: &Document, depth: u16, plane: usize) -> Vec<u8> {
    let color_channels = doc.composite.channels as usize;
    let current = &doc.composite.data[..color_channels * plane];
    let retained: Option<Vec<Vec<u8>>> = (0..4)
        .map(|c| crate::color_mode::composite_retained_8(doc, depth, c))
        .collect();
    if let Some(retained) = retained {
        let retained: Vec<u8> = retained.concat();
        if retained.len() == plane * 4
            && crate::color_mode::cmyk_to_rgb(&retained).as_slice() == current
        {
            return retained;
        }
    }
    crate::color_mode::rgb_to_cmyk(current)
}

/// The four CMYK planes to emit for a layer's color channels, or empty when the
/// layer does not carry exactly three id-0/1/2 color channels of equal length.
/// The retained source planes are re-emitted when their forward conversion still
/// matches the working RGB; an edited layer is re-encoded with the exact
/// [`crate::color_mode::rgb_to_cmyk`] inverse. The returned planes are C, M, Y, K.
fn cmyk_layer_color_planes(layer: &Layer, depth: u16) -> Vec<Vec<u8>> {
    let positions: Vec<usize> = layer
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0 && c.id < 3)
        .map(|(i, _)| i)
        .collect();
    if positions.len() != 3 {
        return Vec::new();
    }
    let plane = layer.channels[positions[0]].data.len();
    if positions
        .iter()
        .any(|&i| layer.channels[i].data.len() != plane)
    {
        return Vec::new();
    }
    let mut current = Vec::with_capacity(plane * 3);
    for &i in &positions {
        current.extend_from_slice(&layer.channels[i].data);
    }
    let retained: Option<Vec<Vec<u8>>> = (0..4)
        .map(|c| crate::color_mode::layer_retained_8(layer, depth, c))
        .collect();
    let flat = match retained {
        Some(retained) => {
            let flat: Vec<u8> = retained.concat();
            if flat.len() == plane * 4
                && crate::color_mode::cmyk_to_rgb(&flat).as_slice() == current.as_slice()
            {
                flat
            } else {
                crate::color_mode::rgb_to_cmyk(&current)
            }
        }
        None => crate::color_mode::rgb_to_cmyk(&current),
    };
    (0..4)
        .map(|c| flat[c * plane..(c + 1) * plane].to_vec())
        .collect()
}

fn write_layer_info(
    doc: &Document,
    psb: bool,
    lab_mode: bool,
    cmyk_mode: bool,
    indexed_mode: bool,
) -> Result<Vec<u8>, PsdError> {
    let records = flatten(&doc.layers);
    if records.len() > i16::MAX as usize {
        return Err(PsdError::Unsupported("too many layer records".into()));
    }

    let mut info = Vec::new();
    info.extend_from_slice(&(records.len() as i16).to_be_bytes());

    let mut channel_data: Vec<Vec<(i16, OutChannel)>> = Vec::with_capacity(records.len());
    let depth = output_depth(doc)?;
    for record in &records {
        let mut channels: Vec<(i16, OutChannel)> = Vec::new();
        if let Some(layer) = record.layer {
            let layer_w = layer.rect.width().max(0) as usize;
            let layer_h = layer.rect.height().max(0) as usize;
            let lab_channels = if lab_mode {
                lab_layer_color_planes(layer, depth)
            } else {
                Vec::new()
            };
            // A CMYK layer's working channels are three RGB planes; the output
            // synthesizes `(0,C),(1,M),(2,Y),(3,K)` and keeps the rest.
            let cmyk_planes = if cmyk_mode {
                cmyk_layer_color_planes(layer, depth)
            } else {
                Vec::new()
            };
            let cmyk_active = cmyk_planes.len() == 4;
            if cmyk_active {
                for (id, data) in cmyk_planes.iter().enumerate() {
                    let id = id as i16;
                    let plane = native_plane(
                        layer_retained(layer, depth, id).as_ref(),
                        data,
                        layer_w,
                        layer_h,
                        depth,
                    )?;
                    channels.push((
                        id,
                        OutChannel::Encoded(channel_stream(
                            doc.layer_compression,
                            layer_w,
                            layer_h,
                            &plane,
                            depth,
                            psb,
                        )?),
                    ));
                }
            }
            // An Indexed layer's three working RGB channels are reconstructed
            // from one retained index plane (id 0); the rest are kept below.
            let indexed_plane = if indexed_mode {
                index_layer_plane(layer, depth)
            } else {
                None
            };
            let indexed_active = indexed_plane.is_some();
            if let Some(data) = &indexed_plane {
                let plane = native_plane(
                    layer_retained(layer, depth, 0).as_ref(),
                    data,
                    layer_w,
                    layer_h,
                    depth,
                )?;
                channels.push((
                    0,
                    OutChannel::Encoded(channel_stream(
                        doc.layer_compression,
                        layer_w,
                        layer_h,
                        &plane,
                        depth,
                        psb,
                    )?),
                ));
            }
            for (index, channel) in layer.channels.iter().enumerate() {
                // The CMYK / Indexed color channels are already synthesized above.
                if (cmyk_active || indexed_active) && channel.id >= 0 && channel.id < 3 {
                    continue;
                }
                let data = lab_channels
                    .get(index)
                    .and_then(|plane| plane.as_deref())
                    .unwrap_or(&channel.data);
                let plane = native_plane(
                    layer_retained(layer, depth, channel.id).as_ref(),
                    data,
                    layer_w,
                    layer_h,
                    depth,
                )?;
                channels.push((
                    channel.id,
                    OutChannel::Encoded(channel_stream(
                        doc.layer_compression,
                        layer_w,
                        layer_h,
                        &plane,
                        depth,
                        psb,
                    )?),
                ));
            }
            for channel in &layer.raw_channels {
                // A depth-8 read keeps the original on-disk stream verbatim. A
                // 16/32-bit read re-wrapped it as a raw 8-bit stream, so replay
                // the retained native plane (or widen) under the output depth.
                if depth == 8 {
                    channels.push((channel.id, OutChannel::Verbatim(channel.data.clone())));
                    continue;
                }
                let plane8 = channel.data.get(2..).unwrap_or(&[]);
                let plane = native_plane(
                    layer_retained(layer, depth, channel.id).as_ref(),
                    plane8,
                    layer_w,
                    layer_h,
                    depth,
                )?;
                channels.push((
                    channel.id,
                    OutChannel::Encoded(channel_stream(
                        doc.layer_compression,
                        layer_w,
                        layer_h,
                        &plane,
                        depth,
                        psb,
                    )?),
                ));
            }
            if let Some(mask) = &layer.mask {
                let width = mask.rect.width().max(0) as usize;
                let height = mask.rect.height().max(0) as usize;
                let pixels = width
                    .checked_mul(height)
                    .ok_or_else(|| PsdError::Invalid("layer mask size overflow".into()))?;
                let data = match &mask.data {
                    Some(data) => {
                        if data.len() != pixels {
                            return Err(PsdError::Invalid(
                                "layer mask data length mismatch".into(),
                            ));
                        }
                        data.clone()
                    }
                    None => vec![mask.default_color; pixels].into(),
                };
                let plane = native_plane(
                    layer_retained(layer, depth, -2).as_ref(),
                    &data,
                    width,
                    height,
                    depth,
                )?;
                channels.push((
                    -2,
                    OutChannel::Encoded(channel_stream(
                        doc.layer_compression,
                        width,
                        height,
                        &plane,
                        depth,
                        psb,
                    )?),
                ));
            }
        }
        if channels.len() > MAX_CHANNELS as usize {
            return Err(PsdError::Invalid(format!(
                "layer channel count {}",
                channels.len()
            )));
        }
        write_record(&mut info, record, &channels, doc.width, doc.height, psb);
        channel_data.push(channels);
    }

    for channels in &channel_data {
        for (_, channel) in channels {
            channel.write(&mut info);
        }
    }

    while info.len() % 4 != 0 {
        info.push(0);
    }
    Ok(info)
}

fn write_record(
    out: &mut Vec<u8>,
    record: &OutRecord,
    channels: &[(i16, OutChannel)],
    doc_width: u32,
    doc_height: u32,
    psb: bool,
) {
    match record.layer {
        Some(layer) => {
            out.extend_from_slice(&layer.rect.top.to_be_bytes());
            out.extend_from_slice(&layer.rect.left.to_be_bytes());
            out.extend_from_slice(&layer.rect.bottom.to_be_bytes());
            out.extend_from_slice(&layer.rect.right.to_be_bytes());
            out.extend_from_slice(&(channels.len() as u16).to_be_bytes());
            for (id, channel) in channels {
                out.extend_from_slice(&id.to_be_bytes());
                if psb {
                    out.extend_from_slice(&channel.declared_len().to_be_bytes());
                } else {
                    out.extend_from_slice(&(channel.declared_len() as u32).to_be_bytes());
                }
            }
            out.extend_from_slice(b"8BIM");
            // Preserve an unrecognized blend key; a recognized key rides along
            // only while it still matches the mode, so a changed mode wins.
            let blend_key = layer
                .blend_key
                .filter(|k| match BlendMode::from_psd_key(*k) {
                    // An unknown key is the only representation of its mode, so
                    // it survives while the mode is still the Normal fallback.
                    None => layer.blend == BlendMode::Normal,
                    Some(mode) => mode == layer.blend,
                })
                .unwrap_or_else(|| layer.blend.to_psd_key());
            out.extend_from_slice(&blend_key);
            out.push(layer.opacity);
            out.push(u8::from(layer.clipping));
            let mut record_flags = if layer.visible { 0 } else { 0x02 };
            if layer.lock.contains(LockFlags::TRANSPARENCY) {
                record_flags |= 0x01;
            }
            out.push(record_flags);
            out.push(0); // filler

            let mut extra = Vec::new();
            write_extra(
                &mut extra,
                layer,
                record.name,
                record.section,
                doc_width,
                doc_height,
                psb,
            );
            out.extend_from_slice(&(extra.len() as u32).to_be_bytes());
            out.extend_from_slice(&extra);
        }
        None => {
            out.extend_from_slice(&[0u8; 16]); // empty rect
            out.extend_from_slice(&0u16.to_be_bytes()); // no channels
            out.extend_from_slice(b"8BIM");
            out.extend_from_slice(&BlendMode::Normal.to_psd_key());
            out.push(255);
            out.push(0);
            out.push(0);
            out.push(0);

            let mut extra = Vec::new();
            write_extra(
                &mut extra,
                &empty_layer(record.name),
                record.name,
                record.section,
                doc_width,
                doc_height,
                psb,
            );
            out.extend_from_slice(&(extra.len() as u32).to_be_bytes());
            out.extend_from_slice(&extra);
        }
    }
}

/// Placeholder used to reuse [`write_extra`] for the (layer-less) divider record.
fn empty_layer(name: &str) -> Layer {
    Layer {
        name: name.to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn write_extra(
    out: &mut Vec<u8>,
    layer: &Layer,
    name: &str,
    section: u32,
    doc_width: u32,
    doc_height: u32,
    psb: bool,
) {
    match &layer.mask {
        Some(mask) => {
            // Mask block: rect (4×i32) + default colour + flags + preserved tail.
            out.extend_from_slice(&(18u32 + mask.extra.len() as u32).to_be_bytes());
            out.extend_from_slice(&mask.rect.top.to_be_bytes());
            out.extend_from_slice(&mask.rect.left.to_be_bytes());
            out.extend_from_slice(&mask.rect.bottom.to_be_bytes());
            out.extend_from_slice(&mask.rect.right.to_be_bytes());
            out.push(mask.default_color);
            let flags = (mask.flags & !0x02) | if mask.disabled { 0x02 } else { 0 };
            out.push(flags);
            out.extend_from_slice(&mask.extra);
        }
        None => out.extend_from_slice(&0u32.to_be_bytes()),
    }
    crate::advanced_blending::write_ranges(out, layer);
    write_pascal(out, name);
    write_tag(out, b"luni", &luni_data(name), psb);
    // M36 layer attributes, each omitted at its default so default documents
    // serialize byte-identically to before.
    if layer.lock.bits() != 0 {
        write_tag(
            out,
            b"lspf",
            &(u32::from(layer.lock.bits())).to_be_bytes(),
            psb,
        );
    }
    if layer.color != ColorLabel::None {
        let mut lclr = [0u8; 8];
        lclr[0..2].copy_from_slice(&u16::from(layer.color.to_byte()).to_be_bytes());
        write_tag(out, b"lclr", &lclr, psb);
    }
    if layer.fill != 255 {
        // psd-tools decodes `iOpa` as a 4-byte ByteElement (`B3x`); the reader
        // only uses the first byte.
        write_tag(out, b"iOpa", &[layer.fill, 0, 0, 0], psb);
    }
    crate::advanced_blending::write_advanced_tags(out, layer, psb);
    if let Some(adjustment) = &layer.adjustment {
        // Adjustment payload is opaque here; write the key and bytes back as read.
        write_tag(out, &adjustment.key, &adjustment.data, psb);
    }
    if section != 0 {
        // Section-divider setting: kind + '8BIM' + blend key. The reference and
        // psd-tools read a group's blend mode from here, so `pass` must ride
        // along with the section marker.
        let mut lsct = Vec::with_capacity(12);
        lsct.extend_from_slice(&section.to_be_bytes());
        lsct.extend_from_slice(b"8BIM");
        lsct.extend_from_slice(&layer.blend.to_psd_key());
        write_tag(out, b"lsct", &lsct, psb);
    }
    // Unmodeled tagged blocks, re-emitted in encounter order.
    for block in &layer.extra_blocks {
        write_tag(out, &block.key, &block.data, psb);
    }
    // An embedded smart object with no preserved config block is authored here.
    if let Some(so) = crate::smart_writer::should_author(layer) {
        let data = crate::smart_writer::author_sold_block(so, layer, doc_width, doc_height);
        write_tag(out, b"SoLd", &data, psb);
    }
    if out.len() % 2 == 1 {
        out.push(0);
    }
}

fn write_pascal(out: &mut Vec<u8>, name: &str) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(255);
    out.push(len as u8);
    out.extend_from_slice(&bytes[..len]);
    let pad = (4 - ((len + 1) % 4)) % 4;
    for _ in 0..pad {
        out.push(0);
    }
}

pub(crate) fn write_tag(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8], psb: bool) {
    // A per-layer block: the reference declares an even length with the pad byte
    // inside it; psd-tools reads exactly the declared length (padding=1 means no
    // external pad), so an odd declared length would mis-frame the next block.
    let declared = data.len() + (data.len() & 1);
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    if psb && is_psb_big_key(key) {
        out.extend_from_slice(&(declared as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(declared as u32).to_be_bytes());
    }
    out.extend_from_slice(data);
    if declared != data.len() {
        out.push(0);
    }
}

/// A document-level (global) additional-layer-information block. psd-tools reads
/// these with `TaggedBlocks.read(..., padding=4)`: the declared length is the
/// exact data length and the block is padded externally to a 4-byte boundary.
/// The big-key width rule is the same as for per-layer blocks.
pub(crate) fn write_tag_document(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8], psb: bool) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    if psb && is_psb_big_key(key) {
        out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(data);
    let pad = (4 - data.len() % 4) % 4;
    out.extend(std::iter::repeat_n(0u8, pad));
}

/// Re-frame preserved document-level tagged blocks for the destination
/// container: each block's data is copied verbatim and its exact length is
/// re-emitted in the destination width, then the block is re-padded externally
/// to a 4-byte boundary. When the source and destination widths agree the bytes
/// are returned unchanged. An unparseable block stops the walk and the remaining
/// bytes are copied verbatim.
fn reframe_document_extra(bytes: &[u8], src_psb: bool, dst_psb: bool) -> Vec<u8> {
    if src_psb == dst_psb {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut r = Reader::new(bytes);
    loop {
        let start = r.pos;
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" && sig != b"8B64" {
            r.pos = start;
            break;
        }
        let Ok(key) = r.take(4) else {
            r.pos = start;
            break;
        };
        let key: [u8; 4] = key.try_into().unwrap();
        let len = if src_psb && is_psb_big_key(&key) {
            match r.u64() {
                Ok(len) => usize::try_from(len).unwrap_or(usize::MAX),
                Err(_) => {
                    r.pos = start;
                    break;
                }
            }
        } else {
            match r.u32() {
                Ok(len) => len as usize,
                Err(_) => {
                    r.pos = start;
                    break;
                }
            }
        };
        let Ok(data) = r.take(len) else {
            r.pos = start;
            break;
        };
        // A global tagged block is padded externally to a 4-byte boundary.
        let pad = (4 - len % 4) % 4;
        if r.skip(pad).is_err() {
            r.pos = start;
            break;
        }
        write_tag_document(&mut out, &key, data, dst_psb);
    }
    out.extend_from_slice(&bytes[r.pos..]);
    out
}

fn luni_data(name: &str) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut data = Vec::with_capacity(4 + units.len() * 2);
    data.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        data.extend_from_slice(&unit.to_be_bytes());
    }
    data
}

// ---------------------------------------------------------------------------
// PackBits RLE encoding
// ---------------------------------------------------------------------------

/// Append one PackBits-encoded scanline to `out`. Runs of three or more equal
/// bytes become a repeat packet (control `1 - n`, then the byte); all other
/// bytes become literal packets of up to 128. The split is fixed, so repeated
/// writes are byte-identical.
fn encode_packbits_row(row: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < row.len() {
        let mut run = 1;
        while i + run < row.len() && row[i + run] == row[i] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            out.push((1 - run as i32) as i8 as u8);
            out.push(row[i]);
            i += run;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < row.len() && end - start < 128 {
            if end + 2 < row.len() && row[end] == row[end + 1] && row[end] == row[end + 2] {
                break;
            }
            end += 1;
        }
        out.push((end - start - 1) as u8);
        out.extend_from_slice(&row[start..end]);
        i = end;
    }
}

/// The native-depth plane to emit for one channel. When `depth` is 8 the
/// current plane is borrowed unchanged. At 16/32, `retained` (the decoded source
/// samples, when its length matches the plane) is used if narrowing it yields
/// `current`; otherwise `current` is widened. A length other than exactly
/// `width * height` is rejected, so widening cannot index out of bounds.
fn native_plane<'a>(
    retained: Option<&Samples>,
    current: &'a [u8],
    width: usize,
    height: usize,
    depth: u16,
) -> Result<Cow<'a, [u8]>, PsdError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel plane size overflow".into()))?;
    if current.len() != pixels {
        return Err(PsdError::Invalid("channel plane length mismatch".into()));
    }
    if depth == 8 {
        return Ok(Cow::Borrowed(current));
    }
    let native = depth_bits(depth).ok_or_else(|| PsdError::Invalid("sample depth".into()))?;
    if let Some(ret) = retained {
        if ret.len() == pixels && ret.narrow_to_u8() == current {
            return Ok(Cow::Owned(ret.to_bytes()));
        }
    }
    Ok(Cow::Owned(
        Samples::widen_from_u8(current, native).to_bytes(),
    ))
}

/// The retained native plane at `index` (composite color channels then document
/// extras) when the document's store matches the output `depth` and canvas.
pub(crate) fn composite_retained(doc: &Document, depth: u16, index: usize) -> Option<Samples> {
    let store = doc.source_planes.as_ref()?;
    if depth_of(Some(store.depth)) != depth
        || store.width != doc.width
        || store.height != doc.height
    {
        return None;
    }
    if store.depth == BitDepth::One {
        return (index == 0).then(|| store.samples.clone());
    }
    let plane = doc.width as usize * doc.height as usize;
    store.samples.slice(index * plane..(index + 1) * plane)
}

/// The retained native plane for layer channel `id`, when the layer's store
/// matches the output `depth` and the layer has not moved.
pub(crate) fn layer_retained(layer: &Layer, depth: u16, id: i16) -> Option<Samples> {
    let store = layer.source_channels.as_ref()?;
    if depth_of(Some(store.depth)) != depth || store.rect != layer.rect {
        return None;
    }
    store
        .planes
        .iter()
        .find(|(channel, _)| *channel == id)
        .map(|(_, samples)| samples.clone())
}

/// Encode `planes` (each `row_bytes * height`, plane-major) into one RLE
/// payload: all 2-byte (PSD) or 4-byte (PSB) scanline byte counts first
/// (plane-major, then row-major), then the packed rows in the same order. The
/// compression word is not included. PackBits is byte-wise, so `row_bytes` is
/// the native row stride (`width` at depth 8, `2 * width` at 16, `4 * width` at
/// 32).
pub(crate) fn encode_scanlines(
    planes: &[&[u8]],
    row_bytes: usize,
    height: usize,
    psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let plane_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("RLE plane size overflow".into()))?;
    let count_width = if psb { 4 } else { 2 };
    let mut counts = Vec::with_capacity(planes.len() * height * count_width);
    let mut rows = Vec::new();
    for plane in planes {
        if plane.len() != plane_len {
            return Err(PsdError::Invalid("RLE plane length mismatch".into()));
        }
        for row in 0..height {
            let start = row * row_bytes;
            let mut packed = Vec::new();
            encode_packbits_row(&plane[start..start + row_bytes], &mut packed);
            // The PSD maximum width (30 000) cannot reach the u16 limit, so the
            // guard only fires for a PSB row approaching u32; it errs rather
            // than truncate. ponytail: one uncompressed row, not the whole plane.
            if psb {
                if packed.len() > u32::MAX as usize {
                    return Err(PsdError::Invalid(
                        "RLE scanline exceeds u32 byte count".into(),
                    ));
                }
                counts.extend_from_slice(&(packed.len() as u32).to_be_bytes());
            } else {
                if packed.len() > u16::MAX as usize {
                    return Err(PsdError::Invalid(
                        "RLE scanline exceeds u16 byte count".into(),
                    ));
                }
                counts.extend_from_slice(&(packed.len() as u16).to_be_bytes());
            }
            rows.extend_from_slice(&packed);
        }
    }
    counts.extend_from_slice(&rows);
    Ok(counts)
}

/// Concatenate the native-depth `planes` (`width` pixels by `height` rows each,
/// plane-major) and zlib-wrap the result. When `predict`, apply the forward
/// depth-specific per-row delta first (the inverse of the reader's
/// `undo_prediction`); no scanline count table.
pub(crate) fn zip_scanlines(
    planes: &[&[u8]],
    width: usize,
    height: usize,
    predict: bool,
    depth: u16,
) -> Result<Vec<u8>, PsdError> {
    let plane_len = row_bytes(width, depth)
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("ZIP plane size overflow".into()))?;
    let mut data = Vec::with_capacity(planes.len() * plane_len);
    for plane in planes {
        if plane.len() != plane_len {
            return Err(PsdError::Invalid("ZIP plane length mismatch".into()));
        }
        data.extend_from_slice(plane);
    }
    if predict {
        apply_prediction(&mut data, width, planes.len() * height, depth);
    }
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&data)
        .map_err(|_| PsdError::Invalid("ZIP encode".into()))?;
    enc.finish()
        .map_err(|_| PsdError::Invalid("ZIP encode".into()))
}

/// The complete on-disk layer-channel stream for one engine-encoded plane: the
/// compression word followed by the payload for `kind` at `depth`. `plane` is
/// the native-depth plane (`row_bytes(width, depth) * height` bytes).
fn channel_stream(
    kind: Compression,
    width: usize,
    height: usize,
    plane: &[u8],
    depth: u16,
    psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let stride = row_bytes(width, depth);
    let mut out = Vec::with_capacity(2 + plane.len());
    out.extend_from_slice(&kind.to_code().to_be_bytes());
    match kind {
        Compression::Rle => {
            out.extend_from_slice(&encode_scanlines(&[plane], stride, height, psb)?)
        }
        Compression::Raw => {
            if plane.len() != stride * height {
                return Err(PsdError::Invalid("raw channel length mismatch".into()));
            }
            out.extend_from_slice(plane);
        }
        Compression::Zip => {
            out.extend_from_slice(&zip_scanlines(&[plane], width, height, false, depth)?)
        }
        Compression::ZipPrediction => {
            out.extend_from_slice(&zip_scanlines(&[plane], width, height, true, depth)?)
        }
    }
    Ok(out)
}

/// Serialize a [`Document`] into a valid PSD, or into a version-2 PSB when the
/// source document was a PSB or a dimension exceeds the PSD limit.
pub fn write_psd(doc: &Document) -> Result<Vec<u8>, PsdError> {
    let psb = doc.is_psb || doc.width > MAX_DIM_PSD || doc.height > MAX_DIM_PSD;
    write_container(doc, psb)
}

/// Serialize a [`Document`] into a version-2 PSB regardless of its dimensions.
pub fn write_psb(doc: &Document) -> Result<Vec<u8>, PsdError> {
    write_container(doc, true)
}

fn write_container(doc: &Document, psb: bool) -> Result<Vec<u8>, PsdError> {
    if doc.depth != BitDepth::Eight {
        return Err(PsdError::Unsupported("write supports 8-bit only".into()));
    }
    // The working model stays 8-bit; the output depth is the recorded source
    // depth (8 when the read retained no samples, including a converted mode),
    // so an open→save of a 16/32-bit file is not a silent downgrade. A flat,
    // unchanged depth-1 Bitmap source forces depth 1 and header mode Bitmap.
    let bitmap_mode = crate::write_bitmap::writes_bitmap(doc);
    let depth = if bitmap_mode { 1 } else { output_depth(doc)? };
    let plane = doc.width as usize * doc.height as usize;
    // A Lab source writes header mode Lab with three color channels: an 8-bit
    // read re-encodes the working RGB, and a 16/32-bit read re-emits the
    // retained native planes (see `lab_composite_planes`). At depth 8 a
    // construct with no recorded depth still fires; a 16/32 doc does because it
    // retained a native store, hence `depth != 8`.
    let lab_mode = doc.source_mode == Some(ColorMode::Lab)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source_depth.is_none());
    // A CMYK source writes header mode CMYK with four color channels: an 8-bit
    // read re-encodes the working RGB, and a 16/32-bit read re-emits the
    // retained native planes.
    let cmyk_mode = doc.source_mode == Some(ColorMode::Cmyk)
        && doc.composite.channels == 3
        && (depth != 8 || doc.source_depth.is_none());
    // An 8-bit Indexed source writes header mode Indexed with one index channel
    // and the retained palette, but only while the composite and every pixel
    // layer still expand to the working RGB; an edit falls back to RGB.
    let indexed_mode = writes_indexed(doc, depth, plane);
    // Flat unedited Duotone/Multichannel re-emits retained plates; edits fall back to RGB.
    let flat_source = crate::write_duotone::flat_source_mode(doc, depth, plane);
    let mode_code = if bitmap_mode {
        MODE_BITMAP
    } else if let Some((code, _, _)) = flat_source {
        code
    } else if lab_mode {
        MODE_LAB
    } else if cmyk_mode {
        MODE_CMYK
    } else if indexed_mode {
        MODE_INDEXED
    } else {
        match doc.mode {
            ColorMode::Grayscale => MODE_GRAYSCALE,
            ColorMode::Rgb => MODE_RGB,
            m => return Err(PsdError::Unsupported(format!("write color mode {m:?}"))),
        }
    };
    let color_channels = doc.composite.channels as usize;
    // Header channel count follows the output mode: 4 CMYK, 1 Indexed, 1-or-3
    // flat Duotone/Multichannel, else the working count. Composite data stays working-term.
    let out_color_channels = if bitmap_mode {
        1
    } else if cmyk_mode {
        4
    } else if indexed_mode {
        1
    } else if let Some((_, n, _)) = flat_source {
        n
    } else {
        color_channels
    };
    let channels = out_color_channels + doc.channels.len();
    if channels == 0 || channels > MAX_CHANNELS as usize {
        return Err(PsdError::Invalid(format!("channel count {channels}")));
    }
    let max_dim = if psb { MAX_DIM_PSB } else { MAX_DIM_PSD };
    if doc.width == 0 || doc.height == 0 || doc.width > max_dim || doc.height > max_dim {
        return Err(PsdError::Unsupported(format!(
            "dimension {}x{}",
            doc.width, doc.height
        )));
    }
    if doc.composite.width != doc.width || doc.composite.height != doc.height {
        return Err(PsdError::Invalid(
            "composite size does not match document".into(),
        ));
    }
    if doc.composite.data.len() != color_channels * plane {
        return Err(PsdError::Invalid("composite data length mismatch".into()));
    }
    for channel in &doc.channels {
        if channel.data.len() != plane {
            return Err(PsdError::Invalid(format!(
                "document channel {} data length mismatch",
                channel.id
            )));
        }
    }

    let mut out = Vec::with_capacity(26 + 12 + 2 + doc.composite.data.len());
    out.extend_from_slice(&SIGNATURE.to_be_bytes());
    out.extend_from_slice(&(if psb { VERSION_PSB } else { VERSION_PSD }).to_be_bytes());
    out.extend_from_slice(&[0u8; 6]); // reserved
    out.extend_from_slice(&(channels as u16).to_be_bytes());
    out.extend_from_slice(&doc.height.to_be_bytes());
    out.extend_from_slice(&doc.width.to_be_bytes());
    out.extend_from_slice(&depth.to_be_bytes()); // depth
    out.extend_from_slice(&mode_code.to_be_bytes());
    // An Indexed output re-emits the retained palette (the read consumed
    // `color_mode_data`); every other mode replays the preserved section.
    let color_mode_data: &[u8] = match (indexed_mode, doc.source_palette.as_ref()) {
        (true, Some(palette)) => palette,
        _ => &doc.color_mode_data,
    };
    out.extend_from_slice(&(color_mode_data.len() as u32).to_be_bytes());
    out.extend_from_slice(color_mode_data);
    let resources = crate::guide_resources::output_resources(doc, mode_code);
    out.extend_from_slice(&(resources.len() as u32).to_be_bytes());
    out.extend_from_slice(&resources);

    // Authored smart objects append a document-level linked-record block; the
    // preserved trailing bytes stay untouched, so an existing file is unchanged.
    // A PSD-sourced document written as a PSB re-frames preserved big-key blocks
    // to u64 lengths so psd-tools can parse them.
    let mut extra = reframe_document_extra(&doc.layer_section_extra, doc.is_psb, psb);
    let authoring = crate::smart_writer::collect_authoring(&doc.layers);
    if !authoring.is_empty() {
        extra.extend_from_slice(&crate::smart_writer::author_lnk2_bytes(&authoring, psb));
    }

    if doc.layers.is_empty() && doc.global_layer_mask.is_empty() && extra.is_empty() {
        if psb {
            out.extend_from_slice(&0u64.to_be_bytes()); // zero-length layer/mask section
        } else {
            out.extend_from_slice(&0u32.to_be_bytes()); // zero-length layer/mask section
        }
    } else {
        let info = write_layer_info(doc, psb, lab_mode, cmyk_mode, indexed_mode)?;
        let len_width = if psb { 8 } else { 4 };
        let section_len = len_width + info.len() + 4 + doc.global_layer_mask.len() + extra.len();
        if psb {
            out.extend_from_slice(&(section_len as u64).to_be_bytes());
            out.extend_from_slice(&(info.len() as u64).to_be_bytes());
        } else {
            out.extend_from_slice(&(section_len as u32).to_be_bytes());
            out.extend_from_slice(&(info.len() as u32).to_be_bytes());
        }
        out.extend_from_slice(&info);
        out.extend_from_slice(&(doc.global_layer_mask.len() as u32).to_be_bytes());
        out.extend_from_slice(&doc.global_layer_mask);
        out.extend_from_slice(&extra);
    }

    // "Maximize Compatibility" off: the source ended after the layer section, so
    // emit no image-data section; the reader then reports no merged composite.
    if !doc.merged_composite_present {
        return Ok(out);
    }

    out.extend_from_slice(&doc.composite_compression.to_code().to_be_bytes());
    let width = doc.width as usize;
    let height = doc.height as usize;
    // The Lab, CMYK, or Indexed output re-emits retained planes exactly when
    // they are unchanged, and otherwise re-encodes from the working RGB into a
    // temp buffer (Indexed never re-encodes: an edit already chose the RGB mode).
    let lab_composite = lab_mode.then(|| lab_composite_planes(doc, depth, plane));
    let cmyk_composite = cmyk_mode.then(|| cmyk_composite_planes(doc, depth, plane));
    let indexed_composite = indexed_mode.then(|| {
        composite_retained(doc, depth, 0)
            .expect("indexed_mode checked the composite plane")
            .to_bytes()
    });
    let flat_composite = flat_source.as_ref().map(|(_, _, planes)| planes);
    let mut planes: Vec<Cow<[u8]>> = Vec::with_capacity(channels);
    if bitmap_mode {
        // The retained depth-1 store holds the raw packed plane (row stride
        // `ceil(width / 8)`), not a `width * height` plane, so `native_plane`
        // does not apply; borrow it directly at the output depth 1.
        let retained = doc
            .source_planes
            .as_ref()
            .and_then(|store| store.samples.as_u8())
            .expect("writes_bitmap checked the packed plane");
        planes.push(Cow::Borrowed(retained));
    } else {
        for c in 0..out_color_channels {
            let current = if let Some(lab) = &lab_composite {
                &lab[c * plane..(c + 1) * plane]
            } else if let Some(cmyk) = &cmyk_composite {
                &cmyk[c * plane..(c + 1) * plane]
            } else if let Some(indexed) = &indexed_composite {
                indexed.as_slice()
            } else if let Some(flat) = &flat_composite {
                &flat[c * plane..(c + 1) * plane]
            } else {
                &doc.composite.data[c * plane..(c + 1) * plane]
            };
            planes.push(native_plane(
                composite_retained(doc, depth, c).as_ref(),
                current,
                width,
                height,
                depth,
            )?);
        }
    }
    for (i, channel) in doc.channels.iter().enumerate() {
        planes.push(native_plane(
            // A CMYK output stores four color planes before the extras, so the
            // retained extras offset is the output color count, not the working 3.
            composite_retained(doc, depth, out_color_channels + i).as_ref(),
            &channel.data,
            width,
            height,
            depth,
        )?);
    }
    let planes: Vec<&[u8]> = planes.iter().map(Cow::as_ref).collect();
    match doc.composite_compression {
        Compression::Rle => out.extend_from_slice(&encode_scanlines(
            &planes,
            row_bytes(width, depth),
            height,
            psb,
        )?),
        Compression::Raw => {
            for plane in &planes {
                out.extend_from_slice(plane);
            }
        }
        Compression::Zip => {
            out.extend_from_slice(&zip_scanlines(&planes, width, height, false, depth)?)
        }
        Compression::ZipPrediction => {
            out.extend_from_slice(&zip_scanlines(&planes, width, height, true, depth)?)
        }
    }
    Ok(out)
}
