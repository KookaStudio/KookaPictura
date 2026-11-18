use super::*;

pub(super) fn read_layer_record(r: &mut Reader, is_psb: bool) -> Result<RawLayer, PsdError> {
    let rect = PsdRect {
        top: r.i32()?,
        left: r.i32()?,
        bottom: r.i32()?,
        right: r.i32()?,
    };
    let nch = r.u16()?;
    if nch > MAX_CHANNELS {
        return Err(PsdError::Invalid(format!("layer channel count {nch}")));
    }
    let mut channel_ids = Vec::with_capacity(nch as usize);
    let mut channel_lens = Vec::with_capacity(nch as usize);
    for _ in 0..nch {
        channel_ids.push(r.i16()?);
        channel_lens.push(if is_psb {
            r.u64()? as usize
        } else {
            r.u32()? as usize
        });
    }

    let mut signature = [0u8; 4];
    signature.copy_from_slice(r.take(4)?);
    if &signature != b"8BIM" {
        return Err(PsdError::Invalid("bad layer blend signature".into()));
    }
    let mut key = [0u8; 4];
    key.copy_from_slice(r.take(4)?);
    // An unrecognized key degrades to Normal and is preserved verbatim; a
    // recognized mode already round-trips through BlendMode.
    let mut blend_key = None;
    let mut blend = match BlendMode::from_psd_key(key) {
        Some(mode) => mode,
        None => {
            blend_key = Some(key);
            BlendMode::Normal
        }
    };

    let opacity = r.u8()?;
    let clipping = r.u8()? != 0;
    let flags = r.u8()?;
    let _filler = r.u8()?;

    // M36 attribute defaults; the tagged blocks below override them.
    let mut fill = 255u8;
    let mut lock = LockFlags::default();
    let mut color = ColorLabel::None;
    let (mut knockout, mut blend_clipping, mut blend_interior) = (Knockout::None, true, false);
    let mut transparency_shapes = true;
    let mut layer_mask_hides_effects = false;
    let mut vector_mask_hides_effects = false;
    let mut restricted_channels: Vec<u32> = Vec::new();

    let extra_len = r.u32()? as usize;
    let extra = r.take(extra_len)?;
    let mut er = Reader::new(extra);

    // Layer mask / adjustment layer data block.
    let mask_len = er.u32()? as usize;
    let mut mask = None;
    let mut vector_params = None;
    if mask_len > 0 {
        let bytes = er.take(mask_len)?;
        let mut mr = Reader::new(bytes);
        let mask_rect = PsdRect {
            top: mr.i32()?,
            left: mr.i32()?,
            bottom: mr.i32()?,
            right: mr.i32()?,
        };
        let default_color = mr.u8()?;
        let mask_flags = mr.u8()?;
        let has_real = channel_ids.contains(&-3);
        let tail = mask_params::read_tail(mask_flags, bytes.get(18..).unwrap_or(&[]), has_real);
        vector_params = mask_params::vector_placeholder(tail.vector);
        // A record with no `-2` channel and no rect only carries parameters.
        let empty = mask_rect.width() <= 0 || mask_rect.height() <= 0;
        if channel_ids.contains(&-2) || !empty {
            mask = Some(LayerMask {
                rect: mask_rect,
                default_color,
                disabled: mask_flags & mask_params::FLAG_DISABLED != 0,
                flags: mask_flags,
                data: None,
                extra: tail.extra.into(),
                density: tail.user.0,
                feather_bits: tail.user.1,
            });
        }
    }

    // Layer blending ranges, kept verbatim for lossless re-save.
    let ranges_len = er.u32()? as usize;
    let blending_ranges = er.take(ranges_len)?.to_vec();
    let blend_if = crate::advanced_blending::parse_blend_if(&blending_ranges);

    // Legacy Pascal name, padded so (length byte + chars) is a multiple of 4.
    let name_len = er.u8()? as usize;
    let name_bytes = er.take(name_len)?;
    let mut name = String::from_utf8_lossy(name_bytes).into_owned();
    let pad = (4 - ((name_len + 1) % 4)) % 4;
    er.skip(pad)?;

    // Additional layer information: 'luni' (Unicode name), 'lsct' (group marker),
    // and the adjustment block for adjustment layers (stored verbatim).
    let mut section = None;
    let mut adjustment = None;
    let mut extra_blocks = Vec::new();
    while er.remaining() >= 12 {
        let mut tag_sig = [0u8; 4];
        tag_sig.copy_from_slice(er.take(4)?);
        // psd-tools accepts 8BIM/8B64 for a tagged block; normalized to 8BIM on write.
        if &tag_sig != b"8BIM" && &tag_sig != b"8B64" {
            return Err(PsdError::Invalid("bad tagged block signature".into()));
        }
        let mut tag_key = [0u8; 4];
        tag_key.copy_from_slice(er.take(4)?);
        let tag_len = if is_psb && is_psb_big_key(&tag_key) {
            er.u64()? as usize
        } else {
            er.u32()? as usize
        };
        let data = er.take(tag_len)?;
        if tag_len % 2 == 1 {
            // Tagged block data is padded to an even length.
            let _ = er.skip(1);
        }
        match &tag_key {
            b"luni" => {
                if let Some(unicode) = parse_luni(data) {
                    name = unicode;
                }
            }
            b"lspf" if data.len() >= 4 => {
                let value = u32::from_be_bytes(data[0..4].try_into().unwrap());
                lock = lock_from_bits(value as u8 & 0x0F);
            }
            b"lclr" if data.len() >= 2 => {
                let value = u16::from_be_bytes(data[0..2].try_into().unwrap());
                color = ColorLabel::from_byte(value as u8);
            }
            b"iOpa" if !data.is_empty() => {
                fill = data[0];
            }
            b"knko" => knockout = Knockout::from_byte(data.first().copied().unwrap_or(0)),
            b"clbl" => blend_clipping = crate::advanced_blending::bool_tag(data, true),
            b"infx" => blend_interior = crate::advanced_blending::bool_tag(data, false),
            b"tsly" => transparency_shapes = crate::advanced_blending::bool_tag(data, true),
            b"lmgm" => layer_mask_hides_effects = crate::advanced_blending::bool_tag(data, false),
            b"vmgm" => vector_mask_hides_effects = crate::advanced_blending::bool_tag(data, false),
            b"brst" => {
                if let Some(channels) = crate::advanced_blending::parse_channel_restrictions(data) {
                    restricted_channels = channels;
                } else {
                    extra_blocks.push(LayerBlock {
                        key: tag_key,
                        data: data.to_vec().into(),
                    });
                }
            }
            b"lsct" if data.len() >= 4 => {
                section = Some(u32::from_be_bytes(data[0..4].try_into().unwrap()));
                // The reference/psd-tools store a group's blend key inside 'lsct'
                // (after the '8BIM' signature); the folder record's own key is
                // usually 'norm'. Prefer the 'lsct' key so pass-through groups
                // load as PassThrough.
                if data.len() >= 12 && &data[4..8] == b"8BIM" {
                    let lsct_key: [u8; 4] = data[8..12].try_into().unwrap();
                    if let Some(m) = BlendMode::from_psd_key(lsct_key) {
                        blend = m;
                    } else {
                        blend_key = Some(lsct_key);
                    }
                }
            }
            b"lsct" => {}
            k if is_adjustment_key(k) => {
                if adjustment.is_none() {
                    adjustment = Some(AdjustmentData {
                        key: tag_key,
                        data: data.to_vec().into(),
                    });
                } else {
                    // A second adjustment key is not modeled twice; keep it raw.
                    extra_blocks.push(LayerBlock {
                        key: tag_key,
                        data: data.to_vec().into(),
                    });
                }
            }
            _ => extra_blocks.push(LayerBlock {
                key: tag_key,
                data: data.to_vec().into(),
            }),
        }
    }

    // The reference's ≤5-era transparency-protected bit shares the transparency lock.
    if flags & 0x01 != 0 {
        lock = lock.with(LockFlags::TRANSPARENCY, true);
    }

    // CS6 default type locks when the layer carries a preserved TySh block.
    if extra_blocks.iter().any(|b| b.key == *b"TySh") {
        lock = lock
            .with(LockFlags::TRANSPARENCY, true)
            .with(LockFlags::PIXELS, true);
    }

    Ok(RawLayer {
        layer: pictura_core::Layer::from_parts(
            pictura_core::LayerParts {
                name,
                rect,
                blend,
                opacity,
                fill,
                lock,
                color,
                clipping,
                visible: flags & 0x02 == 0,
                pixel_data_irrelevant: flags & 0x10 != 0,
                mask,
                adjustment,
                channels: Vec::new(),
                children: pictura_core::LayerStack::default(),
                is_group: false,
                background: false,
                blend_key,
                blending_ranges: blending_ranges.into(),
                knockout,
                blend_clipping,
                blend_interior,
                transparency_shapes,
                layer_mask_hides_effects,
                vector_mask_hides_effects,
                restricted_channels,
                blend_if,
                extra_blocks,
                raw_channels: Vec::new(),
                smart_object: None,
                vector_mask: vector_params,
                type_tool: None,
                applied_character_style: None,
                applied_paragraph_style: None,
                type_overrides: Default::default(),
                source_channels: None,
            },
            pictura_core::LayerContent::Pixel,
        ),
        channel_ids,
        channel_lens,
        section,
    })
}
