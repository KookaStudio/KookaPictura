//! PSD layer-and-mask section writing, including the session encode cache.
//!
//! Split from `write.rs` so the container/composite writer and the layer writer
//! stay under the file-size cap. The on-disk order is unchanged: records are
//! written in a flat group order, then every channel stream in the same order.
//! Independent layer channels are encoded in parallel; the cache key is the
//! source plane's identity plus the encoding parameters.

use pictura_core::*;
use rayon::prelude::*;
use std::borrow::Cow;
use std::sync::Arc;

use crate::common::*;
use crate::depth::output_depth;
use crate::error::PsdError;
use crate::write::{channel_stream, layer_retained, native_plane, write_tag};
use crate::write_cache::{ChannelKey, EncodeCache};
use crate::write_indexed::index_layer_plane;

/// A flat record in on-disk order, borrowing the model layer it came from.
struct OutRecord<'a> {
    layer: Option<&'a Layer>,
    section: u32,
    name: &'a str,
}

/// A layer channel to emit. Both variants are a complete on-disk stream
/// (compression word included): `Encoded` is engine-authored channel data,
/// `Verbatim` is a preserved `Layer.raw_channels` stream re-emitted unchanged.
enum OutChannel {
    Encoded(Arc<[u8]>),
    Verbatim(Vec<u8>),
}

impl OutChannel {
    fn bytes(&self) -> &[u8] {
        match self {
            OutChannel::Encoded(data) => data.as_ref(),
            OutChannel::Verbatim(data) => data.as_slice(),
        }
    }

    fn declared_len(&self) -> u64 {
        self.bytes().len() as u64
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.bytes());
    }
}

enum ChannelSource<'a> {
    Encode(Cow<'a, [u8]>),
    Verbatim(Vec<u8>),
}

struct ChannelPlan<'a> {
    id: i16,
    width: usize,
    height: usize,
    source: ChannelSource<'a>,
    key: Option<ChannelKey>,
}

impl<'a> ChannelPlan<'a> {
    fn encode(
        id: i16,
        width: usize,
        height: usize,
        data: Cow<'a, [u8]>,
        key: Option<ChannelKey>,
    ) -> Self {
        Self {
            id,
            width,
            height,
            source: ChannelSource::Encode(data),
            key,
        }
    }

    fn verbatim(id: i16, data: Vec<u8>) -> Self {
        Self {
            id,
            width: 0,
            height: 0,
            source: ChannelSource::Verbatim(data),
            key: None,
        }
    }
}

struct LayerPlan<'a> {
    record: OutRecord<'a>,
    channels: Vec<ChannelPlan<'a>>,
}

struct EncodedChannel {
    channel: OutChannel,
    encoded: bool,
    key: Option<ChannelKey>,
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

/// The Lab planes to emit for a layer's three color channels, aligned to
/// `layer.channels` (a `None` entry for every non-color channel). Prefers the
/// retained Lab plane when its forward conversion still matches the working
/// planes; empty when the layer does not carry exactly three id-0/1/2 color
/// channels of equal length, letting the caller keep the original planes.
///
/// ponytail: one edited channel re-encodes all three Lab planes.
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
    let known = layer
        .source_channels
        .as_ref()
        .is_some_and(|store| store.working_idents != [0; 3]);
    let ids = [0i16, 1, 2].map(|id| {
        layer
            .channels
            .iter()
            .find(|channel| channel.id == id)
            .map(|channel| channel.data.ident())
            .unwrap_or(0)
    });
    let lab = if known {
        if layer
            .source_channels
            .as_ref()
            .map(|store| store.working_idents)
            == Some(ids)
        {
            match retained {
                Some(planes)
                    if planes.iter().map(|plane| plane.len()).sum::<usize>() == plane * 3 =>
                {
                    planes.concat()
                }
                _ => crate::color_mode::rgb_to_lab(&current),
            }
        } else {
            crate::color_mode::rgb_to_lab(&current)
        }
    } else {
        match retained.map(|planes| planes.concat()) {
            Some(retained)
                if retained.len() == plane * 3
                    && crate::color_mode::lab_to_rgb(&retained).as_slice()
                        == current.as_slice() =>
            {
                retained
            }
            _ => crate::color_mode::rgb_to_lab(&current),
        }
    };
    let mut out = vec![None; layer.channels.len()];
    for (c, &i) in positions.iter().enumerate() {
        out[i] = Some(lab[c * plane..(c + 1) * plane].to_vec());
    }
    out
}

fn layer_plans<'a>(
    doc: &'a Document,
    psb: bool,
    lab_mode: bool,
    cmyk_mode: bool,
    indexed_mode: bool,
) -> Result<Vec<LayerPlan<'a>>, PsdError> {
    let records = flatten(&doc.layers);
    if records.len() > i16::MAX as usize {
        return Err(PsdError::Unsupported("too many layer records".into()));
    }
    let depth = output_depth(doc)?;
    let compression = doc.layer_compression.to_code();
    let cmyk_profile = crate::icc::cmyk_profile_from_resources(doc);
    let mut plans = Vec::with_capacity(records.len());
    for record in records {
        let mut channels = Vec::new();
        if let Some(layer) = record.layer {
            let layer_w = layer.rect.width().max(0) as usize;
            let layer_h = layer.rect.height().max(0) as usize;
            let cacheable = depth == 8
                && layer.source_channels.is_none()
                && !lab_mode
                && !cmyk_mode
                && !indexed_mode;
            let lab_channels = if lab_mode {
                lab_layer_color_planes(layer, depth)
            } else {
                Vec::new()
            };
            // A CMYK layer's working channels are three RGB planes; the output
            // synthesizes `(0,C),(1,M),(2,Y),(3,K)` and keeps the rest.
            let cmyk_planes = if cmyk_mode {
                crate::write_cmyk::cmyk_layer_color_planes(layer, depth, cmyk_profile.as_ref())
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
                    channels.push(ChannelPlan::encode(
                        id,
                        layer_w,
                        layer_h,
                        Cow::Owned(plane.into_owned()),
                        None,
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
                channels.push(ChannelPlan::encode(
                    0,
                    layer_w,
                    layer_h,
                    Cow::Owned(plane.into_owned()),
                    None,
                ));
            }
            for (index, channel) in layer.channels.iter().enumerate() {
                // The CMYK / Indexed color channels are already synthesized above.
                if (cmyk_active || indexed_active) && channel.id >= 0 && channel.id < 3 {
                    continue;
                }
                let key = if cacheable {
                    Some(ChannelKey {
                        ident: channel.data.ident(),
                        width: layer_w as u32,
                        height: layer_h as u32,
                        depth,
                        compression,
                        psb,
                        role: 0,
                    })
                } else {
                    None
                };
                // A Lab plane lives in the local `lab_channels` vector, so it
                // must be owned; a working plane may borrow the document.
                let data = match lab_channels.get(index).and_then(|plane| plane.as_deref()) {
                    Some(lab) => {
                        let plane = native_plane(
                            layer_retained(layer, depth, channel.id).as_ref(),
                            lab,
                            layer_w,
                            layer_h,
                            depth,
                        )?;
                        Cow::Owned(plane.into_owned())
                    }
                    None => native_plane(
                        layer_retained(layer, depth, channel.id).as_ref(),
                        channel.data.as_slice(),
                        layer_w,
                        layer_h,
                        depth,
                    )?,
                };
                channels.push(ChannelPlan::encode(channel.id, layer_w, layer_h, data, key));
            }
            for channel in &layer.raw_channels {
                // A depth-8 read keeps the original on-disk stream verbatim. A
                // 16/32-bit read re-wrapped it as a raw 8-bit stream, so replay
                // the retained native plane (or widen) under the output depth.
                if depth == 8 {
                    channels.push(ChannelPlan::verbatim(channel.id, channel.data.clone()));
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
                channels.push(ChannelPlan::encode(
                    channel.id,
                    layer_w,
                    layer_h,
                    Cow::Owned(plane.into_owned()),
                    None,
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
                        data.to_vec()
                    }
                    None => vec![mask.default_color; pixels],
                };
                let ident = mask.data.as_ref().map(|plane| plane.ident()).unwrap_or(0);
                let key = if cacheable && ident != 0 {
                    Some(ChannelKey {
                        ident,
                        width: width as u32,
                        height: height as u32,
                        depth,
                        compression,
                        psb,
                        role: 1,
                    })
                } else {
                    None
                };
                let plane = native_plane(
                    layer_retained(layer, depth, -2).as_ref(),
                    &data,
                    width,
                    height,
                    depth,
                )?;
                channels.push(ChannelPlan::encode(
                    -2,
                    width,
                    height,
                    Cow::Owned(plane.into_owned()),
                    key,
                ));
            }
        }
        if channels.len() > MAX_CHANNELS as usize {
            return Err(PsdError::Invalid(format!(
                "layer channel count {}",
                channels.len()
            )));
        }
        plans.push(LayerPlan { record, channels });
    }
    Ok(plans)
}

fn encode_channel(
    cache: Option<&EncodeCache>,
    compression: Compression,
    depth: u16,
    psb: bool,
    channel: &ChannelPlan<'_>,
) -> Result<EncodedChannel, PsdError> {
    match &channel.source {
        ChannelSource::Verbatim(data) => Ok(EncodedChannel {
            channel: OutChannel::Verbatim(data.clone()),
            encoded: false,
            key: None,
        }),
        ChannelSource::Encode(data) => {
            let kind = channel
                .key
                .and_then(|key| Compression::from_code(key.compression))
                .unwrap_or(compression);
            let psb = channel.key.map(|key| key.psb).unwrap_or(psb);
            if let (Some(cache), Some(key)) = (cache, channel.key) {
                if let Some(bytes) = cache.get_channel(&key) {
                    return Ok(EncodedChannel {
                        channel: OutChannel::Encoded(bytes),
                        encoded: false,
                        key: Some(key),
                    });
                }
                let bytes: Arc<[u8]> = channel_stream(
                    kind,
                    channel.width,
                    channel.height,
                    data.as_ref(),
                    depth,
                    psb,
                )?
                .into();
                cache.put_channel(key, Arc::clone(&bytes));
                return Ok(EncodedChannel {
                    channel: OutChannel::Encoded(bytes),
                    encoded: true,
                    key: Some(key),
                });
            }
            let bytes: Arc<[u8]> = channel_stream(
                kind,
                channel.width,
                channel.height,
                data.as_ref(),
                depth,
                psb,
            )?
            .into();
            Ok(EncodedChannel {
                channel: OutChannel::Encoded(bytes),
                encoded: true,
                key: None,
            })
        }
    }
}

/// Write the layer-and-mask section body and return the cache keys it used.
pub(crate) fn write_layer_info(
    doc: &Document,
    psb: bool,
    lab_mode: bool,
    cmyk_mode: bool,
    indexed_mode: bool,
    cache: Option<&EncodeCache>,
) -> Result<(Vec<u8>, Vec<ChannelKey>), PsdError> {
    let plans = layer_plans(doc, psb, lab_mode, cmyk_mode, indexed_mode)?;
    let tasks: Vec<(usize, usize)> = plans
        .iter()
        .enumerate()
        .flat_map(|(li, plan)| (0..plan.channels.len()).map(move |ci| (li, ci)))
        .collect();
    let compression = doc.layer_compression;
    let depth = output_depth(doc)?;
    let results: Vec<Result<EncodedChannel, PsdError>> = tasks
        .par_iter()
        .map(|&(li, ci)| encode_channel(cache, compression, depth, psb, &plans[li].channels[ci]))
        .collect();

    let mut encoded: Vec<Vec<(i16, OutChannel)>> = plans
        .iter()
        .map(|plan| Vec::with_capacity(plan.channels.len()))
        .collect();
    let mut layer_encoded = vec![false; plans.len()];
    let mut used = Vec::new();
    for ((li, ci), result) in tasks.iter().zip(results) {
        let out = result?;
        if out.encoded {
            layer_encoded[*li] = true;
        }
        if let Some(key) = out.key {
            used.push(key);
        }
        encoded[*li].push((plans[*li].channels[*ci].id, out.channel));
    }
    if let Some(cache) = cache {
        let count = layer_encoded.iter().filter(|encoded| **encoded).count();
        cache.record_layers_encoded(count as u64);
    }

    let mut info = Vec::new();
    info.extend_from_slice(&(plans.len() as i16).to_be_bytes());
    for (plan, channels) in plans.iter().zip(&encoded) {
        write_record(
            &mut info,
            &plan.record,
            channels,
            doc.width,
            doc.height,
            psb,
        );
    }
    for channels in &encoded {
        for (_, channel) in channels {
            channel.write(&mut info);
        }
    }
    while info.len() % 4 != 0 {
        info.push(0);
    }
    Ok((info, used))
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
        children: pictura_core::LayerStack::default(),
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
    crate::mask_params::write_record(out, layer);
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

fn luni_data(name: &str) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut data = Vec::with_capacity(4 + units.len() * 2);
    data.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        data.extend_from_slice(&unit.to_be_bytes());
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_lab_layer_reuses_retained_planes() {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let working = [[10u8, 20, 30, 40], [50, 60, 70, 80], [90, 100, 110, 120]];
        let mut layer = Layer {
            name: "Lab".to_string(),
            rect,
            channels: working
                .iter()
                .enumerate()
                .map(|(id, plane)| Channel {
                    id: id as i16,
                    data: plane.to_vec().into(),
                })
                .collect(),
            ..Default::default()
        };
        let idents = [0i16, 1, 2].map(|id| {
            layer
                .channels
                .iter()
                .find(|channel| channel.id == id)
                .expect("working channel")
                .data
                .ident()
        });
        let retained = [
            Samples::U8(vec![1, 2, 3, 4]),
            Samples::U8(vec![5, 6, 7, 8]),
            Samples::U8(vec![9, 10, 11, 12]),
        ];
        let mut store = SourceChannels::new(
            BitDepth::Eight,
            rect,
            retained
                .iter()
                .cloned()
                .enumerate()
                .map(|(id, samples)| (id as i16, samples))
                .collect(),
        );
        store.working_idents = idents;
        layer.source_channels = Some(store);
        let expected: Vec<Vec<u8>> = retained
            .iter()
            .map(|samples| match samples {
                Samples::U8(values) => values.clone(),
                _ => unreachable!("the store is 8-bit"),
            })
            .collect();
        let planes = lab_layer_color_planes(&layer, 8);
        let emitted: Vec<Vec<u8>> = planes.iter().flatten().cloned().collect();
        assert_eq!(emitted, expected);
    }
}
