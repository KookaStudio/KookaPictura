use crate::common::Reader;
use crate::common::COMPRESSION_RLE;
use crate::depth::row_bytes;
use crate::image_resources::{decode_section, frame_image_resource};
use crate::write::encode_packbits_row;
use crate::write_stream::Sink;
use crate::PsdError;

const VERSION_INFO: u16 = 1057;

pub(crate) fn has_merged_transparency_marker(extra: &[u8], psb: bool) -> bool {
    let mut reader = Reader::new(extra);
    while let Some((key, _, _, _)) = crate::filter_effects::next_block(&mut reader, psb) {
        if matches!(&key, b"Mtrn" | b"Mt16" | b"Mt32") {
            return true;
        }
    }
    false
}

pub(crate) fn has_real_merged_data(resources: &[u8]) -> Option<bool> {
    decode_section(resources)
        .0
        .into_iter()
        .find(|resource| resource.id == VERSION_INFO && resource.data.len() >= 5)
        .map(|resource| resource.data[4] != 0)
}

pub(crate) fn set_real_merged_data(resources: &[u8], present: bool) -> Vec<u8> {
    let (records, consumed) = decode_section(resources);
    let mut output = Vec::with_capacity(resources.len());
    let mut updated = false;
    for record in records {
        if record.id == VERSION_INFO && record.data.len() >= 5 {
            let mut raw = record.raw;
            let data_offset = raw.len() - record.data.len() - record.data.len() % 2;
            raw[data_offset + 4] = u8::from(present);
            output.extend_from_slice(&raw);
            updated = true;
        } else {
            output.extend_from_slice(&record.raw);
        }
    }
    if !updated && !present {
        let mut data = vec![0, 0, 0, 1, 0];
        for name in ["Kooka Pictura", "Kooka Pictura"] {
            let encoded: Vec<_> = name.encode_utf16().collect();
            data.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
            for unit in encoded {
                data.extend_from_slice(&unit.to_be_bytes());
            }
        }
        data.extend_from_slice(&1u32.to_be_bytes());
        output.extend_from_slice(&frame_image_resource(VERSION_INFO, "", &data).raw);
    }
    output.extend_from_slice(&resources[consumed..]);
    output
}

pub(crate) fn put_placeholder(
    sink: &mut Sink<'_>,
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
    psb: bool,
) -> Result<(), PsdError> {
    // A zero float is uniform at the byte level, keeping even the widest
    // 32-bit PSD scanline within the 16-bit PackBits row-length field.
    let fill = if depth == 32 { 0 } else { 255 };
    let row = vec![fill; row_bytes(width, depth)];
    let mut packed = Vec::new();
    encode_packbits_row(&row, &mut packed);
    let count = (packed.len() as u32).to_be_bytes();
    let count = if psb { &count[..] } else { &count[2..] };
    sink.put(&COMPRESSION_RLE.to_be_bytes())?;
    for _ in 0..height * channels {
        sink.put(count)?;
    }
    for _ in 0..height * channels {
        sink.put(&packed)?;
    }
    Ok(())
}
