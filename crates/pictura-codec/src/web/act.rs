//! Adobe Color Table (`.act`): 256 RGB entries, optionally followed by a
//! 2-byte colour count and a 2-byte transparent index. The trailer makes short
//! tables round-trip instead of being padded to 256 indistinguishable entries.

/// A loaded colour table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColorTable {
    pub colors: Vec<[u8; 3]>,
    pub transparent: Option<u8>,
}

impl ColorTable {
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}

/// Parse a 768-byte (full, opaque) or 772-byte (count + transparency) table.
pub fn parse_act(bytes: &[u8]) -> Option<ColorTable> {
    match bytes.len() {
        768 => Some(ColorTable {
            colors: bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
            transparent: None,
        }),
        772 => {
            let count = usize::from(u16::from_be_bytes([bytes[768], bytes[769]]));
            if count == 0 || count > 256 {
                // A bad trailer: the first 768 bytes are still a full table.
                return Some(ColorTable {
                    colors: bytes[..768]
                        .chunks_exact(3)
                        .map(|c| [c[0], c[1], c[2]])
                        .collect(),
                    transparent: None,
                });
            }
            let clear = u16::from_be_bytes([bytes[770], bytes[771]]);
            Some(ColorTable {
                colors: bytes[..count * 3]
                    .chunks_exact(3)
                    .map(|c| [c[0], c[1], c[2]])
                    .collect(),
                transparent: (clear < count as u16).then_some(clear as u8),
            })
        }
        _ => None,
    }
}

/// Encode a table. A full 256-colour opaque table writes 768 bytes; every other
/// table writes 772 with its count and the transparent index (0xFFFF = none).
pub fn encode_act(table: &ColorTable) -> Vec<u8> {
    let count = table.colors.len().clamp(1, 256);
    let mut out = vec![0u8; 768];
    for (i, color) in table.colors.iter().take(count).enumerate() {
        out[i * 3..i * 3 + 3].copy_from_slice(color);
    }
    if count == 256 && table.transparent.is_none() {
        return out;
    }
    out.extend_from_slice(&(count as u16).to_be_bytes());
    let clear = table
        .transparent
        .filter(|&i| usize::from(i) < count)
        .map_or(0xFFFF, u16::from);
    out.extend_from_slice(&clear.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(n: usize, transparent: Option<u8>) -> ColorTable {
        ColorTable {
            colors: (0..n).map(|i| [i as u8, (i * 3) as u8, 255]).collect(),
            transparent,
        }
    }

    #[test]
    fn act_roundtrip() {
        let short = table(16, None);
        let bytes = encode_act(&short);
        assert_eq!(bytes.len(), 772);
        assert_eq!(parse_act(&bytes), Some(short));

        let clear = table(16, Some(3));
        let bytes = encode_act(&clear);
        assert_eq!(bytes.len(), 772);
        assert_eq!(parse_act(&bytes), Some(clear));

        let full = table(256, None);
        let bytes = encode_act(&full);
        assert_eq!(bytes.len(), 768);
        assert_eq!(parse_act(&bytes), Some(full));

        let full_clear = table(256, Some(255));
        let bytes = encode_act(&full_clear);
        assert_eq!(bytes.len(), 772);
        assert_eq!(parse_act(&bytes), Some(full_clear));
        assert_eq!(parse_act(&[0u8; 7]), None);
    }

    #[test]
    fn act_invalid_count_falls_back_to_the_full_table() {
        let full = table(256, None);
        let mut bytes = encode_act(&full);
        bytes.resize(772, 0);
        for count in [0u16, 300] {
            bytes[768..770].copy_from_slice(&count.to_be_bytes());
            assert_eq!(parse_act(&bytes), Some(full.clone()), "count {count}");
        }
    }
}
