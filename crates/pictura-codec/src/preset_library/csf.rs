//! Photoshop `AsCs` version-2 Color Settings files.

use super::{error, Diagnostic, Reader};
use std::collections::BTreeSet;

pub use pictura_color::Intent;

const HEADER_LEN: usize = 128;
const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_TAGS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Rgb,
    Cmyk,
    Gray,
    Spot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Convert,
    Other([u8; 4]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorSettingsFile {
    header: [u8; HEADER_LEN],
    tags: Vec<([u8; 4], Vec<u8>)>,
}

impl Default for ColorSettingsFile {
    fn default() -> Self {
        let mut header = [0; HEADER_LEN];
        header[8..12].copy_from_slice(&[0, 2, 0, 0]);
        header[36..40].copy_from_slice(b"AsCs");
        Self {
            header,
            tags: Vec::new(),
        }
    }
}

impl ColorSettingsFile {
    /// Return the embedded profile without resolving against system profiles.
    pub fn profile(&self, space: Space) -> Option<&[u8]> {
        self.get(profile_key(space)).and_then(read_profile)
    }

    pub fn set_profile(&mut self, space: Space, profile: &[u8]) -> Result<(), Diagnostic> {
        if !valid_profile(profile) || profile.len() > MAX_FILE_BYTES - 8 {
            return Err(error(0, "invalid working profile for the CSF file"));
        }
        let mut bytes = b"prof\0\0\0\0".to_vec();
        bytes.extend_from_slice(profile);
        self.set(profile_key(space), bytes);
        Ok(())
    }

    pub fn policy(&self, space: Space) -> Option<Policy> {
        let key = policy_key(space)?;
        let value = self.get(key).and_then(read_u32)?.to_be_bytes();
        Some(if &value == b"conv" {
            Policy::Convert
        } else {
            Policy::Other(value)
        })
    }

    pub fn set_policy(&mut self, space: Space, policy: Policy) -> Result<(), Diagnostic> {
        let key =
            policy_key(space).ok_or_else(|| error(0, "the CSF variant has no spot policy"))?;
        let Policy::Convert = policy else {
            return Err(error(0, "unsupported CSF color management policy"));
        };
        self.set_u32(key, u32::from_be_bytes(*b"conv"));
        Ok(())
    }

    pub fn intent(&self) -> Option<Intent> {
        match self.get(b"cInt").and_then(read_u32)? {
            0 => Some(Intent::Perceptual),
            1 => Some(Intent::RelativeColorimetric),
            2 => Some(Intent::Saturation),
            3 => Some(Intent::AbsoluteColorimetric),
            _ => None,
        }
    }

    pub fn set_intent(&mut self, intent: Intent) {
        let value = match intent {
            Intent::Perceptual => 0,
            Intent::RelativeColorimetric => 1,
            Intent::Saturation => 2,
            Intent::AbsoluteColorimetric => 3,
        };
        self.set_u32(b"cInt", value);
    }

    pub fn black_point_compensation(&self) -> Option<bool> {
        self.boolean(b"kpc ")
    }
    pub fn dither(&self) -> Option<bool> {
        self.boolean(b"dith")
    }
    pub fn set_black_point_compensation(&mut self, enabled: bool) {
        self.set_u32(b"kpc ", enabled.into());
    }
    pub fn set_dither(&mut self, enabled: bool) {
        self.set_u32(b"dith", enabled.into());
    }

    pub fn unsupported_keys(&self) -> Vec<[u8; 4]> {
        self.tags
            .iter()
            .filter_map(|(key, bytes)| {
                let understood = match key {
                    b"wRGB" | b"wCMY" | b"wGry" | b"wSpt" => read_profile(bytes).is_some(),
                    b"pRGB" | b"pCMY" | b"pGry" => {
                        read_u32(bytes) == Some(u32::from_be_bytes(*b"conv"))
                    }
                    b"cInt" => self.intent().is_some(),
                    b"kpc " | b"dith" => self.boolean(key).is_some(),
                    _ => false,
                };
                (!understood).then_some(*key)
            })
            .collect()
    }

    fn boolean(&self, key: &[u8; 4]) -> Option<bool> {
        match self.get(key).and_then(read_u32)? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    fn get(&self, key: &[u8; 4]) -> Option<&[u8]> {
        self.tags
            .iter()
            .find_map(|(tag, bytes)| (tag == key).then_some(bytes.as_slice()))
    }

    fn set(&mut self, key: &[u8; 4], bytes: Vec<u8>) {
        if let Some((_, existing)) = self.tags.iter_mut().find(|(tag, _)| tag == key) {
            *existing = bytes;
        } else {
            self.tags.push((*key, bytes));
        }
    }

    fn set_u32(&mut self, key: &[u8; 4], value: u32) {
        let mut bytes = b"ui32\0\0\0\0".to_vec();
        bytes.extend_from_slice(&value.to_be_bytes());
        self.set(key, bytes);
    }
}

pub fn decode(bytes: &[u8]) -> Result<ColorSettingsFile, Diagnostic> {
    if bytes.len() > MAX_FILE_BYTES || bytes.len() < HEADER_LEN + 4 {
        return Err(error(0, "invalid CSF file size"));
    }
    let mut reader = Reader::new(bytes);
    let header: [u8; HEADER_LEN] = reader.take(HEADER_LEN)?.try_into().unwrap();
    if &header[36..40] != b"AsCs" || header[8..12] != [0, 2, 0, 0] {
        return Err(error(8, "unsupported CSF signature or version"));
    }
    if u32::from_be_bytes(header[..4].try_into().unwrap()) as usize != bytes.len() {
        return Err(error(0, "invalid declared CSF size"));
    }
    let count = reader.u32()? as usize;
    if count > MAX_TAGS || count > reader.remaining() / 12 {
        return Err(error(reader.pos, "invalid CSF tag count"));
    }
    let mut keys = BTreeSet::new();
    let table_end = reader.pos + count * 12;
    let mut tags = Vec::with_capacity(count);
    let mut total = table_end;
    for _ in 0..count {
        let key = reader.take(4)?.try_into().unwrap();
        let offset = reader.u32()? as usize;
        let length = reader.u32()? as usize;
        if !keys.insert(key) || offset < table_end || !offset.is_multiple_of(4) || length < 8 {
            return Err(error(reader.pos - 12, "invalid CSF tag directory entry"));
        }
        total = total
            .checked_add(length)
            .filter(|n| *n <= MAX_FILE_BYTES)
            .ok_or_else(|| error(reader.pos - 4, "CSF tag data exceeds the byte limit"))?;
        let data = bytes
            .get(offset..)
            .and_then(|tail| tail.get(..length))
            .ok_or_else(|| error(offset, "truncated CSF tag"))?;
        tags.push((key, data.to_vec()));
    }
    Ok(ColorSettingsFile { header, tags })
}

pub fn encode(settings: &ColorSettingsFile) -> Result<Vec<u8>, Diagnostic> {
    let count = settings.tags.len();
    if count > MAX_TAGS {
        return Err(error(HEADER_LEN, "too many CSF tags"));
    }
    let mut bytes = settings.header.to_vec();
    bytes.extend_from_slice(&(count as u32).to_be_bytes());
    bytes.resize(HEADER_LEN + 4 + 12 * count, 0);
    for (index, (key, data)) in settings.tags.iter().enumerate() {
        let length = bytes.len() + data.len();
        if length.next_multiple_of(4) > MAX_FILE_BYTES {
            return Err(error(bytes.len(), "CSF output exceeds the byte limit"));
        }
        let offset = (bytes.len() as u32).to_be_bytes();
        let at = HEADER_LEN + 4 + 12 * index;
        bytes[at..at + 4].copy_from_slice(key);
        bytes[at + 4..at + 8].copy_from_slice(&offset);
        bytes[at + 8..at + 12].copy_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(data);
        bytes.resize(length.next_multiple_of(4), 0);
    }
    let size = (bytes.len() as u32).to_be_bytes();
    bytes[..4].copy_from_slice(&size);
    Ok(bytes)
}

fn read_u32(bytes: &[u8]) -> Option<u32> {
    (bytes.len() == 12 && bytes.starts_with(b"ui32\0\0\0\0"))
        .then(|| u32::from_be_bytes(bytes[8..12].try_into().unwrap()))
}

fn valid_profile(bytes: &[u8]) -> bool {
    bytes.len() >= 132
        && &bytes[36..40] == b"acsp"
        && u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize == bytes.len()
}

fn read_profile(bytes: &[u8]) -> Option<&[u8]> {
    let bytes = bytes.strip_prefix(b"prof\0\0\0\0")?;
    valid_profile(bytes).then_some(bytes)
}

fn profile_key(space: Space) -> &'static [u8; 4] {
    match space {
        Space::Rgb => b"wRGB",
        Space::Cmyk => b"wCMY",
        Space::Gray => b"wGry",
        Space::Spot => b"wSpt",
    }
}

fn policy_key(space: Space) -> Option<&'static [u8; 4]> {
    Some(match space {
        Space::Rgb => b"pRGB",
        Space::Cmyk => b"pCMY",
        Space::Gray => b"pGry",
        Space::Spot => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csf_tag_limit_is_enforced_on_export() {
        let mut settings = ColorSettingsFile::default();
        settings.tags = (0..MAX_TAGS)
            .map(|i| ((i as u32).to_be_bytes(), vec![0; 8]))
            .collect();
        let mut decoded = decode(&encode(&settings).unwrap()).unwrap();
        decoded.set_intent(Intent::Perceptual);
        assert!(encode(&decoded).is_err());
    }
}
