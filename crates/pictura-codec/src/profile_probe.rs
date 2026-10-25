//! Lightweight embedded-profile probes. They read only the container metadata
//! and never decode pixels, so the open pipeline can ask about a profile before
//! committing a document.

use std::io::Read;

use pictura_color::Profile;

/// The embedded profile found in a container, if any.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileProbe {
    pub embedded: Option<Vec<u8>>,
    pub channels: Option<u8>,
    pub is_srgb: bool,
    /// A profile block was found even when it does not parse.
    pub present: bool,
}

impl ProfileProbe {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        match Profile::from_icc(&bytes) {
            Ok(profile) => Self {
                embedded: Some(bytes),
                channels: profile.channels(),
                is_srgb: profile.is_srgb(),
                present: true,
            },
            Err(_) => Self {
                present: true,
                ..Self::default()
            },
        }
    }
}

fn be16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Probe a PSD/PSB image-resource section for resource 1039.
pub fn probe_psd_profile(bytes: &[u8]) -> ProfileProbe {
    if bytes.len() < 26 || &bytes[..4] != b"8BPS" {
        return ProfileProbe::default();
    }
    let Some(color_len) = be32(bytes, 26) else {
        return ProfileProbe::default();
    };
    let resources_at = 30usize.saturating_add(color_len as usize);
    let Some(resources_len) = be32(bytes, resources_at) else {
        return ProfileProbe::default();
    };
    let start = resources_at.saturating_add(4);
    let end = start.saturating_add(resources_len as usize);
    let Some(section) = bytes.get(start..end) else {
        return ProfileProbe::default();
    };
    let mut at = 0usize;
    while at + 12 <= section.len() {
        if &section[at..at + 4] != b"8BIM" {
            break;
        }
        let id = u16::from_be_bytes(section[at + 4..at + 6].try_into().unwrap_or([0, 0]));
        let name_len = usize::from(section[at + 6]);
        let name_bytes = 1 + name_len;
        let padded = (name_bytes + 1) & !1;
        let data_len_at = at + 6 + padded;
        let Some(data_len) = be32(section, data_len_at) else {
            break;
        };
        let data_at = data_len_at + 4;
        let data_end = data_at.saturating_add(data_len as usize);
        let Some(data) = section.get(data_at..data_end) else {
            break;
        };
        if id == 1039 {
            return ProfileProbe::from_bytes(data.to_vec());
        }
        at = data_end + (usize::from(data_len % 2 != 0));
    }
    ProfileProbe::default()
}

/// Probe JPEG APP2 `ICC_PROFILE` segments.
pub fn probe_jpeg_profile(bytes: &[u8]) -> ProfileProbe {
    if bytes.len() < 4 || bytes[0..2] != [0xFF, 0xD8] {
        return ProfileProbe::default();
    }
    let mut at = 2usize;
    let mut profile = Vec::new();
    while at + 4 <= bytes.len() {
        if bytes[at] != 0xFF {
            break;
        }
        let marker = bytes[at + 1];
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        let Some(len) = be16(bytes, at + 2) else {
            break;
        };
        if len < 2 {
            break;
        }
        let payload_at = at + 4;
        let payload_end = at + 2 + usize::from(len);
        let Some(payload) = bytes.get(payload_at..payload_end) else {
            break;
        };
        if marker == 0xE2 && payload.starts_with(b"ICC_PROFILE\0") && payload.len() >= 14 {
            profile.extend_from_slice(&payload[14..]);
        }
        at = payload_end;
    }
    if profile.is_empty() {
        ProfileProbe::default()
    } else {
        ProfileProbe::from_bytes(profile)
    }
}

/// Probe a PNG `iCCP` chunk.
pub fn probe_png_profile(bytes: &[u8]) -> ProfileProbe {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 8 || bytes[..8] != SIGNATURE {
        return ProfileProbe::default();
    }
    let mut at = 8usize;
    while at + 12 <= bytes.len() {
        let Some(len) = be32(bytes, at) else {
            break;
        };
        let kind = &bytes[at + 4..at + 8];
        let data_at = at + 8;
        let data_end = data_at.saturating_add(len as usize);
        let Some(data) = bytes.get(data_at..data_end) else {
            break;
        };
        if kind == b"iCCP" {
            let Some(nul) = data.iter().position(|b| *b == 0) else {
                return ProfileProbe::default();
            };
            let compressed = &data[nul + 2..];
            let mut decoder = flate2::read::ZlibDecoder::new(compressed);
            let mut profile = Vec::new();
            if decoder.read_to_end(&mut profile).is_err() {
                return ProfileProbe {
                    present: true,
                    ..ProfileProbe::default()
                };
            }
            return ProfileProbe::from_bytes(profile);
        }
        if kind == b"IEND" {
            break;
        }
        at = data_end.saturating_add(4);
    }
    ProfileProbe::default()
}

/// Probe a TIFF IFD for tag 34675 (`ICC Profile`).
pub fn probe_tiff_profile(bytes: &[u8]) -> ProfileProbe {
    if bytes.len() < 8 {
        return ProfileProbe::default();
    }
    let little = match &bytes[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return ProfileProbe::default(),
    };
    let u16_at = |at: usize| -> Option<u16> {
        let raw = bytes.get(at..at + 2)?;
        Some(if little {
            u16::from_le_bytes(raw.try_into().ok()?)
        } else {
            u16::from_be_bytes(raw.try_into().ok()?)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let raw = bytes.get(at..at + 4)?;
        Some(if little {
            u32::from_le_bytes(raw.try_into().ok()?)
        } else {
            u32::from_be_bytes(raw.try_into().ok()?)
        })
    };
    if u16_at(2) != Some(42) {
        return ProfileProbe::default();
    }
    let Some(ifd) = u32_at(4).map(|v| v as usize) else {
        return ProfileProbe::default();
    };
    let Some(count) = u16_at(ifd) else {
        return ProfileProbe::default();
    };
    for index in 0..usize::from(count) {
        let entry = ifd + 2 + index * 12;
        let Some(tag) = u16_at(entry) else {
            break;
        };
        if tag != 34675 {
            continue;
        }
        let Some(kind) = u16_at(entry + 2) else {
            break;
        };
        let Some(elements) = u32_at(entry + 4).map(|v| v as usize) else {
            break;
        };
        if kind != 7 {
            continue;
        }
        let value_at = entry + 8;
        let data = if elements <= 4 {
            bytes.get(value_at..value_at + elements)
        } else {
            let Some(offset) = u32_at(value_at).map(|v| v as usize) else {
                break;
            };
            bytes.get(offset..offset.saturating_add(elements))
        };
        if let Some(data) = data {
            return ProfileProbe::from_bytes(data.to_vec());
        }
    }
    ProfileProbe::default()
}

/// Probe a raster container by its signature.
pub fn probe_raster_profile(bytes: &[u8]) -> ProfileProbe {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        probe_jpeg_profile(bytes)
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        probe_png_profile(bytes)
    } else if bytes.starts_with(b"II") || bytes.starts_with(b"MM") {
        probe_tiff_profile(bytes)
    } else {
        ProfileProbe::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_signature_is_empty() {
        assert_eq!(probe_raster_profile(b"nope"), ProfileProbe::default());
        assert_eq!(probe_psd_profile(b"nope"), ProfileProbe::default());
    }

    #[test]
    fn malformed_containers_do_not_panic() {
        for bytes in [
            b"\xFF\xD8\xFF\xE2\x00".as_slice(),
            b"\x89PNG\r\n\x1a\n\x00\x00".as_slice(),
            b"II*\x00\xFF\xFF\xFF\x7F".as_slice(),
        ] {
            let _ = probe_raster_profile(bytes);
        }
    }
}
