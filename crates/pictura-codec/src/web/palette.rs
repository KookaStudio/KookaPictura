//! Packed indexed images and the colour-table operations the Save for Web
//! dialog applies without re-quantizing.

use super::Indexed;

/// Sort order for the colour table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSort {
    Hue,
    Luminance,
    Popularity,
}

/// The bridge's packed form: `[n_hi, n_lo]`, `n` RGBA entries, then one index
/// per pixel.
pub fn pack(indexed: &Indexed) -> Vec<u8> {
    let n = indexed.palette.len().min(256);
    let mut out = (n as u16).to_be_bytes().to_vec();
    for entry in indexed.palette.iter().take(n) {
        out.extend_from_slice(entry);
    }
    out.extend_from_slice(&indexed.indices);
    out
}

/// Unpack [`pack`] when the buffer matches `width` x `height`.
pub fn unpack(bytes: &[u8], width: u32, height: u32) -> Option<Indexed> {
    let n = usize::from(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]));
    if n == 0 || bytes.len() != 2 + n * 4 + width as usize * height as usize {
        return None;
    }
    let palette = bytes[2..2 + n * 4]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    Some(Indexed {
        width,
        height,
        palette,
        indices: bytes[2 + n * 4..].to_vec(),
    })
}

/// Change one palette entry; pixels using it change with it.
pub fn edit(indexed: &Indexed, index: u8, rgba: [u8; 4]) -> Indexed {
    let mut out = indexed.clone();
    if let Some(entry) = out.palette.get_mut(usize::from(index)) {
        *entry = rgba;
    }
    out
}

/// Remove one entry, remapping the pixels that used it to the nearest remaining
/// colour.
pub fn delete(indexed: &Indexed, index: u8) -> Indexed {
    let Some(removed) = indexed.palette.get(usize::from(index)).copied() else {
        return indexed.clone();
    };
    let mut palette = indexed.palette.clone();
    palette.remove(usize::from(index));
    if palette.is_empty() {
        palette.push([0, 0, 0, 255]);
    }
    let indices = indexed
        .indices
        .iter()
        .map(|&old| {
            if old == index {
                nearest(removed, &palette)
            } else if old > index {
                old - 1
            } else {
                old
            }
        })
        .collect();
    Indexed {
        width: indexed.width,
        height: indexed.height,
        palette,
        indices,
    }
}

/// Reorder the opaque entries and keep the transparent entry last.
pub fn sort(indexed: &Indexed, by: PaletteSort) -> Indexed {
    let count = |i: usize| {
        indexed
            .indices
            .iter()
            .filter(|&&index| usize::from(index) == i)
            .count()
    };
    let mut order: Vec<usize> = (0..indexed.palette.len())
        .filter(|&i| indexed.palette[i][3] != 0)
        .collect();
    match by {
        PaletteSort::Hue => {
            order.sort_by(|&a, &b| hue(indexed.palette[a]).total_cmp(&hue(indexed.palette[b])))
        }
        PaletteSort::Luminance => order.sort_by(|&a, &b| {
            luminance(indexed.palette[a]).total_cmp(&luminance(indexed.palette[b]))
        }),
        PaletteSort::Popularity => order.sort_by_key(|&i| std::cmp::Reverse(count(i))),
    }
    if let Some(clear) = indexed.palette.iter().position(|c| c[3] == 0) {
        order.push(clear);
    }
    let mut mapping = vec![0u8; indexed.palette.len()];
    for (new, &old) in order.iter().enumerate() {
        mapping[old] = new as u8;
    }
    Indexed {
        width: indexed.width,
        height: indexed.height,
        palette: order.iter().map(|&i| indexed.palette[i]).collect(),
        indices: indexed
            .indices
            .iter()
            .map(|&i| mapping[usize::from(i)])
            .collect(),
    }
}

/// Make one entry transparent (or opaque again). Mapping clears every other
/// transparent entry, so the table holds exactly one.
pub fn map_transparency(indexed: &Indexed, index: u8, transparent: bool) -> Indexed {
    let mut out = indexed.clone();
    for (i, entry) in out.palette.iter_mut().enumerate() {
        if transparent {
            entry[3] = if i == usize::from(index) { 0 } else { 255 };
        } else if i == usize::from(index) {
            entry[3] = 255;
        }
    }
    out
}

fn nearest(rgba: [u8; 4], palette: &[[u8; 4]]) -> u8 {
    let mut best = (u32::MAX, 0u8);
    for (i, color) in palette.iter().enumerate() {
        let d: u32 = (0..3)
            .map(|ch| {
                let e = i32::from(rgba[ch]) - i32::from(color[ch]);
                (e * e) as u32
            })
            .sum();
        if d < best.0 {
            best = (d, i as u8);
        }
    }
    best.1
}

fn hue(color: [u8; 4]) -> f32 {
    let (r, g, b) = (
        f32::from(color[0]) / 255.0,
        f32::from(color[1]) / 255.0,
        f32::from(color[2]) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max == min {
        return 0.0;
    }
    let d = max - min;
    let h = if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0).rem_euclid(360.0)
}

fn luminance(color: [u8; 4]) -> f32 {
    0.299 * f32::from(color[0]) + 0.587 * f32::from(color[1]) + 0.114 * f32::from(color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed() -> Indexed {
        Indexed {
            width: 4,
            height: 1,
            palette: vec![
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [10, 250, 10, 255],
                [0, 0, 255, 0],
            ],
            indices: vec![0, 1, 2, 3],
        }
    }

    #[test]
    fn palette_edit_changes_one_entry() {
        let out = edit(&indexed(), 1, [1, 2, 3, 255]);
        assert_eq!(out.palette[1], [1, 2, 3, 255]);
        assert_eq!(out.palette[0], [255, 0, 0, 255]);
        assert_eq!(out.palette[2], [10, 250, 10, 255]);
        assert_eq!(out.indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn palette_delete_remaps_nearest() {
        let out = delete(&indexed(), 1);
        assert_eq!(out.palette.len(), 3);
        assert_eq!(out.palette[1], [10, 250, 10, 255]);
        assert_eq!(out.indices, vec![0, 1, 1, 2]);
    }

    #[test]
    fn palette_map_transparency_moves_entry() {
        let out = map_transparency(&indexed(), 0, true);
        assert_eq!(out.palette[0], [255, 0, 0, 0]);
        assert_eq!(out.palette[3], [0, 0, 255, 255]);
        assert_eq!(out.palette.iter().filter(|c| c[3] == 0).count(), 1);
        let back = map_transparency(&out, 0, false);
        assert_eq!(back.palette[0], [255, 0, 0, 255]);
        assert!(back.palette.iter().all(|c| c[3] == 255));
    }
}
