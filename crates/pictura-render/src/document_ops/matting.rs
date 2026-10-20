//! `Layer > Matting`: destructive edge cleanup on a single pixel layer.
//!
//! `Remove Black Matte` / `Remove White Matte` recover the un-composited colour
//! `out = (px - bg * (1 - a)) / a` for the colour channels, leaving alpha alone.
//! `Defringe` replaces the colour of the edge band — the solid pixels within a
//! width of the transparent region — with the nearest interior colour. Both are
//! gated by an optional coverage mask (the active selection), refuse a locked or
//! empty layer, and never change transparency.

use pictura_core::{layer_pixel_locked, Layer, LayerMask};

/// The background colour assumed to have been composited into the layer alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatteBackground {
    Black,
    White,
}

/// Why a matting op refused. A refusal leaves the layer bit-identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MattingError {
    /// The layer is pixel-locked.
    Locked,
    /// The layer carries no editable colour pixels (an empty rect or a missing
    /// or malformed colour plane).
    Empty,
}

impl std::fmt::Display for MattingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MattingError::Locked => write!(f, "layer is pixel-locked"),
            MattingError::Empty => write!(f, "layer has no editable pixels"),
        }
    }
}

impl std::error::Error for MattingError {}

/// Recover the layer's un-composited colour assuming a `background` was
/// composited into its alpha, gated by an optional coverage `mask`.
pub fn remove_matte(
    layer: &mut Layer,
    background: MatteBackground,
    mask: Option<&LayerMask>,
) -> Result<(), MattingError> {
    if layer_pixel_locked(layer) {
        return Err(MattingError::Locked);
    }
    let planes = color_planes(layer)?;
    let (w, h) = (layer.rect.width() as usize, layer.rect.height() as usize);
    let (ox, oy) = (layer.rect.left, layer.rect.top);
    let alpha: Vec<u8> = channel(layer, -1).map(<[u8]>::to_vec).unwrap_or_default();
    let bg = match background {
        MatteBackground::Black => 0.0,
        MatteBackground::White => 255.0,
    };

    for &c in &planes {
        let plane = channel_mut(layer, c).expect("colour plane validated above");
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let a = alpha.get(i).copied().unwrap_or(255);
                if a == 0 {
                    continue;
                }
                let cov = crate::filter::coverage(mask, ox + x as i32, oy + y as i32);
                if cov == 0 {
                    continue;
                }
                let p = plane[i] as f64;
                let an = a as f64 / 255.0;
                let target = ((p - bg * (1.0 - an)) / an).round().clamp(0.0, 255.0);
                plane[i] = blend(p, target, cov);
            }
        }
    }
    Ok(())
}

/// Replace the colour of the layer's edge pixels — solid pixels within `width`
/// of the transparent region — with the nearest interior pixel's colour, gated
/// by an optional coverage `mask`.
pub fn defringe(
    layer: &mut Layer,
    width: u32,
    mask: Option<&LayerMask>,
) -> Result<(), MattingError> {
    if layer_pixel_locked(layer) {
        return Err(MattingError::Locked);
    }
    let planes = color_planes(layer)?;
    let (w, h) = (layer.rect.width() as usize, layer.rect.height() as usize);
    let (ox, oy) = (layer.rect.left, layer.rect.top);
    // No alpha channel means no transparent pixels, so there is no edge band.
    let Some(alpha) = channel(layer, -1) else {
        return Ok(());
    };
    let alpha: Vec<u8> = alpha.to_vec();
    let n = w * h;
    let width_i = width as i32;
    let dist = transparent_distance(&alpha, w, h);

    let mut fill: Vec<Option<usize>> = vec![None; n];
    let mut queue = std::collections::VecDeque::new();
    for i in 0..n {
        if alpha.get(i).copied().unwrap_or(255) > 0 && dist[i] > width_i {
            fill[i] = Some(i);
            queue.push_back(i);
        }
    }
    let neighbors = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        for (dx, dy) in neighbors {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if fill[j].is_none() && alpha.get(j).copied().unwrap_or(255) > 0 && dist[j] <= width_i {
                fill[j] = fill[i];
                queue.push_back(j);
            }
        }
    }

    let orig: Vec<Vec<u8>> = planes
        .iter()
        .map(|&c| {
            channel(layer, c)
                .expect("colour plane validated above")
                .to_vec()
        })
        .collect();
    for (pi, &c) in planes.iter().enumerate() {
        let plane = channel_mut(layer, c).expect("colour plane validated above");
        for i in 0..n {
            let Some(src) = fill[i] else {
                continue;
            };
            let cov = crate::filter::coverage(mask, ox + (i % w) as i32, oy + (i / w) as i32);
            if cov == 0 {
                continue;
            }
            plane[i] = blend(plane[i] as f64, orig[pi][src] as f64, cov);
        }
    }
    Ok(())
}

/// The colour planes a matting op writes: all three, or channel 0 alone for a
/// Grayscale layer. A zero-area rect or a missing/malformed plane is `Empty`.
fn color_planes(layer: &Layer) -> Result<Vec<i16>, MattingError> {
    let (w, h) = (layer.rect.width(), layer.rect.height());
    if w <= 0 || h <= 0 {
        return Err(MattingError::Empty);
    }
    let n = w as usize * h as usize;
    let plane_len = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.len())
    };
    if plane_len(0) != Some(n) {
        return Err(MattingError::Empty);
    }
    match (plane_len(1), plane_len(2)) {
        (Some(_), Some(_)) => {
            for id in [1, 2] {
                if plane_len(id) != Some(n) {
                    return Err(MattingError::Empty);
                }
            }
            Ok(vec![0, 1, 2])
        }
        (None, None) => Ok(vec![0]),
        _ => Err(MattingError::Empty),
    }
}

/// Chebyshev distance from each pixel to the nearest `alpha == 0` pixel, via a
/// two-pass chamfer. A pixel with no transparent neighbour gets `i32::MAX`.
fn transparent_distance(alpha: &[u8], w: usize, h: usize) -> Vec<i32> {
    let n = w * h;
    let mut dist = vec![i32::MAX; n];
    for (i, slot) in dist.iter_mut().enumerate() {
        if alpha.get(i).copied().unwrap_or(255) == 0 {
            *slot = 0;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if dist[i] == 0 {
                continue;
            }
            let mut d = dist[i];
            if x > 0 {
                d = d.min(dist[i - 1].saturating_add(1));
            }
            if y > 0 {
                d = d.min(dist[i - w].saturating_add(1));
            }
            if x > 0 && y > 0 {
                d = d.min(dist[i - w - 1].saturating_add(1));
            }
            if x + 1 < w && y > 0 {
                d = d.min(dist[i - w + 1].saturating_add(1));
            }
            dist[i] = d;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if dist[i] == 0 {
                continue;
            }
            let mut d = dist[i];
            if x + 1 < w {
                d = d.min(dist[i + 1].saturating_add(1));
            }
            if y + 1 < h {
                d = d.min(dist[i + w].saturating_add(1));
            }
            if x + 1 < w && y + 1 < h {
                d = d.min(dist[i + w + 1].saturating_add(1));
            }
            if x > 0 && y + 1 < h {
                d = d.min(dist[i + w - 1].saturating_add(1));
            }
            dist[i] = d;
        }
    }
    dist
}

fn blend(orig: f64, target: f64, cov: u8) -> u8 {
    (orig + (target - orig) * (cov as f64 / 255.0))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn channel(layer: &Layer, id: i16) -> Option<&[u8]> {
    layer
        .channels
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.as_slice())
}

fn channel_mut(layer: &mut Layer, id: i16) -> Option<&mut [u8]> {
    layer
        .channels
        .iter_mut()
        .find(|c| c.id == id)
        .map(|c| c.data.as_mut_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BlendMode, Channel, ColorLabel, LockFlags, PsdRect};

    fn rect(w: i32, h: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        }
    }

    /// A `w`×`h` layer with the given per-channel colour and alpha.
    fn layer(w: i32, h: i32, colour: [u8; 3], alpha: Vec<u8>) -> Layer {
        let n = (w * h) as usize;
        let plane = |v: u8| vec![v; n];
        Layer {
            name: "px".into(),
            rect: rect(w, h),
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: plane(colour[0]).into(),
                },
                Channel {
                    id: 1,
                    data: plane(colour[1]).into(),
                },
                Channel {
                    id: 2,
                    data: plane(colour[2]).into(),
                },
                Channel {
                    id: -1,
                    data: alpha.into(),
                },
            ],
            children: pictura_core::LayerStack::default(),
            is_group: false,
            background: false,
            ..Default::default()
        }
    }

    fn chan(layer: &Layer, id: i16) -> &[u8] {
        channel(layer, id).expect("channel")
    }

    fn mask(cover: impl Fn(usize) -> u8, n: usize) -> LayerMask {
        LayerMask {
            rect: rect(n as i32, 1),
            default_color: 0,
            disabled: false,
            flags: 0,
            data: Some((0..n).map(cover).collect::<Vec<u8>>().into()),
            ..Default::default()
        }
    }

    #[test]
    fn remove_black_matte_divides_by_alpha() {
        // a = 128 (~0.502): un-matting 100 over black gives ~199.
        let mut l = layer(1, 1, [100, 50, 25], vec![128]);
        remove_matte(&mut l, MatteBackground::Black, None).unwrap();
        let a = 128.0_f64 / 255.0;
        assert_eq!(chan(&l, 0)[0], (100.0 / a).round() as u8);
        assert_eq!(chan(&l, 1)[0], (50.0 / a).round() as u8);
        assert_eq!(chan(&l, 2)[0], (25.0 / a).round() as u8);
    }

    #[test]
    fn remove_white_matte_subtracts_the_white_fraction() {
        let mut l = layer(1, 1, [200, 190, 180], vec![128]);
        remove_matte(&mut l, MatteBackground::White, None).unwrap();
        let a = 128.0_f64 / 255.0;
        let expect = |p: u8| ((p as f64 - 255.0 * (1.0 - a)) / a).round() as u8;
        assert_eq!(chan(&l, 0)[0], expect(200));
        assert_eq!(chan(&l, 1)[0], expect(190));
        assert_eq!(chan(&l, 2)[0], expect(180));
    }

    #[test]
    fn remove_matte_clamps_and_leaves_transparent_pixels() {
        // A white-matte subtraction would go negative; a zero-alpha pixel is skipped.
        let mut l = layer(2, 1, [10, 10, 10], vec![200, 0]);
        remove_matte(&mut l, MatteBackground::White, None).unwrap();
        assert_eq!(chan(&l, 0)[0], 0, "clamped to zero");
        assert_eq!(chan(&l, 0)[1], 10, "transparent pixel untouched");
        assert_eq!(chan(&l, -1), &[200, 0], "alpha unchanged");
    }

    #[test]
    fn remove_matte_honours_mask_coverage() {
        let mut l = layer(2, 1, [100, 100, 100], vec![128, 128]);
        let m = mask(|i| if i == 0 { 255 } else { 0 }, 2);
        remove_matte(&mut l, MatteBackground::Black, Some(&m)).unwrap();
        assert_ne!(chan(&l, 0)[0], 100, "covered pixel un-matted");
        assert_eq!(chan(&l, 0)[1], 100, "uncovered pixel untouched");
    }

    #[test]
    fn locked_and_empty_are_refused_without_change() {
        let mut locked = layer(2, 1, [100, 100, 100], vec![128, 128]);
        locked.lock = LockFlags::default().with(LockFlags::PIXELS, true);
        let before = locked.clone();
        assert_eq!(
            remove_matte(&mut locked, MatteBackground::Black, None),
            Err(MattingError::Locked)
        );
        assert_eq!(locked, before);

        let mut empty = layer(2, 1, [100, 100, 100], vec![128, 128]);
        empty.rect = rect(0, 1);
        assert_eq!(
            remove_matte(&mut empty, MatteBackground::Black, None),
            Err(MattingError::Empty)
        );

        let mut no_colour = layer(2, 1, [100, 100, 100], vec![128, 128]);
        no_colour.channels.retain(|c| c.id == -1);
        assert_eq!(
            remove_matte(&mut no_colour, MatteBackground::Black, None),
            Err(MattingError::Empty)
        );
    }

    #[test]
    fn defringe_replaces_the_edge_band_with_the_interior_colour() {
        // 5-pixel row: transparent, opaque*3, transparent. Both edge pixels are
        // off-colour; the middle pixel is the interior source.
        let mut l = layer(5, 1, [10, 10, 10], vec![0, 255, 255, 255, 0]);
        for c in &mut l.channels {
            if c.id == 2 {
                c.data = vec![10, 10, 200, 10, 200].into();
            }
        }
        defringe(&mut l, 1, None).unwrap();
        assert_eq!(chan(&l, 2)[1], 200, "edge takes the interior colour");
        assert_eq!(chan(&l, 2)[3], 200, "the other edge too");
        assert_eq!(chan(&l, 2)[2], 200, "interior unchanged");
        assert_eq!(chan(&l, -1), &[0, 255, 255, 255, 0], "alpha unchanged");
    }

    #[test]
    fn defringe_width_reaches_deeper_and_leaves_a_core() {
        // 7-pixel row: transparent, five opaque, transparent. The colour rises
        // 10..70 so each band pixel's nearest interior colour is distinct.
        let build = || {
            let mut l = layer(7, 1, [10, 10, 10], vec![0, 255, 255, 255, 255, 255, 0]);
            for c in &mut l.channels {
                if c.id == 0 {
                    c.data = vec![10, 20, 30, 40, 50, 60, 70].into();
                }
            }
            l
        };
        // Width 1: the band is index 1 and 5; the interior runs 2..=4.
        let mut w1 = build();
        defringe(&mut w1, 1, None).unwrap();
        assert_eq!(chan(&w1, 0), &[10, 30, 30, 40, 50, 50, 70]);
        // Width 2: the band reaches index 2 and 4; only index 3 is interior.
        let mut w2 = build();
        defringe(&mut w2, 2, None).unwrap();
        assert_eq!(chan(&w2, 0), &[10, 40, 40, 40, 40, 40, 70]);
    }

    #[test]
    fn defringe_on_an_opaque_layer_is_a_noop() {
        let mut l = layer(3, 1, [7, 8, 9], vec![255, 255, 255]);
        let before = l.clone();
        defringe(&mut l, 1, None).unwrap();
        assert_eq!(l, before);
    }

    #[test]
    fn defringe_honours_mask_coverage() {
        let mut l = layer(5, 1, [10, 10, 10], vec![0, 255, 255, 255, 0]);
        for c in &mut l.channels {
            if c.id == 2 {
                c.data = vec![10, 10, 200, 10, 200].into();
            }
        }
        let m = mask(|i| if i == 1 { 0 } else { 255 }, 5);
        defringe(&mut l, 1, Some(&m)).unwrap();
        assert_eq!(chan(&l, 2)[1], 10, "masked-out band pixel keeps its colour");
        assert_eq!(chan(&l, 2)[3], 200, "an uncovered band pixel changes");
    }
}
