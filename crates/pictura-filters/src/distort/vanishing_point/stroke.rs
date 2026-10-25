//! Stamp and Brush strokes: round dabs sized and spaced in a family's flat
//! space, so they foreshorten with the planes. A stroke keeps, per touched
//! 64-pixel tile, the layer as it was, the strongest coverage any dab gave
//! and the colour that dab put down. The canvas always shows the original
//! blended toward that colour by the coverage times Opacity, so overlapping
//! dabs do not build up; Heal re-blends the whole stroke when it ends.

use std::collections::HashMap;

use super::canvas::{bilinear, Canvas, PixelStep, Rect};
use super::geometry::P2;
use super::heal::{blend, HealMode};
use super::space::FamilyMap;

const TILE: i32 = 64;

/// Diameter, Hardness, Opacity and Heal for Stamp or Brush.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintOptions {
    /// Flat units (≈ px on a square-on plane).
    pub diameter: f64,
    /// `0..=1`: the share of the radius painted at full strength.
    pub hardness: f64,
    pub opacity: f64,
    pub heal: HealMode,
}

impl Default for PaintOptions {
    fn default() -> Self {
        PaintOptions {
            diameter: 100.0,
            hardness: 1.0,
            opacity: 1.0,
            heal: HealMode::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ink {
    Colour([u8; 3]),
    /// Clone from the source family's flat point `source + (dab − anchor)`.
    Clone {
        source_family: usize,
        source_home: usize,
        source: P2,
        anchor: P2,
    },
}

struct Tile {
    orig: Vec<u8>,
    cover: Vec<f32>,
    colour: Vec<[f32; 4]>,
}

pub struct Stroke {
    map: FamilyMap,
    source_map: Option<FamilyMap>,
    home: usize,
    ink: Ink,
    options: PaintOptions,
    last: Option<P2>,
    tiles: HashMap<(i32, i32), Tile>,
}

fn tile_of(x: i32, y: i32) -> (i32, i32) {
    (x.div_euclid(TILE), y.div_euclid(TILE))
}

impl Stroke {
    pub fn new(
        map: FamilyMap,
        source_map: Option<FamilyMap>,
        home: usize,
        ink: Ink,
        options: PaintOptions,
    ) -> Self {
        Stroke {
            map,
            source_map,
            home,
            ink,
            options,
            last: None,
            tiles: HashMap::new(),
        }
    }

    /// The layer as it was before the stroke at document `(x, y)`.
    fn original(&self, canvas: &Canvas, x: i32, y: i32) -> [u8; 4] {
        let key = tile_of(x, y);
        match self.tiles.get(&key) {
            Some(t) => {
                let i = (((y - key.1 * TILE) * TILE + (x - key.0 * TILE)) * 4) as usize;
                [t.orig[i], t.orig[i + 1], t.orig[i + 2], t.orig[i + 3]]
            }
            None => canvas.get(x, y),
        }
    }

    /// Bilinear sample of the layer as it was before the stroke.
    fn sample_original(&self, canvas: &Canvas, p: P2) -> [f32; 4] {
        bilinear(p, |x, y| self.original(canvas, x as i32, y as i32))
    }

    fn tile(&mut self, canvas: &Canvas, key: (i32, i32)) -> &mut Tile {
        self.tiles.entry(key).or_insert_with(|| {
            let r = Rect {
                x0: key.0 * TILE,
                y0: key.1 * TILE,
                x1: key.0 * TILE + TILE,
                y1: key.1 * TILE + TILE,
            };
            Tile {
                orig: canvas.read(r),
                cover: vec![0.0; (TILE * TILE) as usize],
                colour: vec![[0.0; 4]; (TILE * TILE) as usize],
            }
        })
    }

    /// Extend the stroke to image point `p`; the pixels it changed.
    pub fn to(&mut self, canvas: &mut Canvas, p: P2) -> Rect {
        let Some((_, flat)) = self.map.locate(p, Some(self.home)) else {
            self.last = None;
            return Rect::default();
        };
        let spacing = (self.options.diameter * 0.25).max(1.0);
        let mut dabs = Vec::new();
        match self.last {
            None => dabs.push(flat),
            Some(last) => {
                let (dx, dy) = (flat.0 - last.0, flat.1 - last.1);
                let n = ((dx.hypot(dy)) / spacing).floor() as usize;
                for k in 1..=n {
                    let t = k as f64 * spacing / dx.hypot(dy);
                    dabs.push((last.0 + dx * t, last.1 + dy * t));
                }
            }
        }
        let mut dirty = Rect::default();
        for d in dabs {
            dirty = dirty.union(self.dab(canvas, d));
            self.last = Some(d);
        }
        dirty
    }

    fn dab(&mut self, canvas: &mut Canvas, c: P2) -> Rect {
        let r = (self.options.diameter / 2.0).max(0.5);
        let inner = r * self.options.hardness.clamp(0.0, 1.0);
        let ring: Vec<P2> = (0..16)
            .filter_map(|k| {
                let a = k as f64 * std::f64::consts::TAU / 16.0;
                self.map
                    .to_image((c.0 + r * a.cos(), c.1 + r * a.sin()), Some(self.home))
            })
            .collect();
        if ring.is_empty() {
            return Rect::default();
        }
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in &ring {
            b = [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)];
        }
        let rect = Rect::around(b, 2).intersect(canvas.bounds());
        let opacity = self.options.opacity.clamp(0.0, 1.0) as f32;
        let live = self.options.heal == HealMode::Off;
        for y in rect.y0..rect.y1 {
            for x in rect.x0..rect.x1 {
                let p = (x as f64 + 0.5, y as f64 + 0.5);
                let Some((_, q)) = self.map.locate(p, Some(self.home)) else {
                    continue;
                };
                let d = (q.0 - c.0).hypot(q.1 - c.1);
                let cov = if d <= inner {
                    1.0
                } else if d >= r {
                    0.0
                } else {
                    let t = (r - d) / (r - inner).max(1e-6);
                    t * t * (3.0 - 2.0 * t)
                } as f32;
                if cov <= 0.0 {
                    continue;
                }
                let ink = match self.ink {
                    Ink::Colour(rgb) => [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, 1.0],
                    Ink::Clone {
                        source_home,
                        source,
                        anchor,
                        ..
                    } => {
                        let s = (source.0 + q.0 - anchor.0, source.1 + q.1 - anchor.1);
                        let map = self.source_map.as_ref().unwrap_or(&self.map);
                        match map.to_image(s, Some(source_home)) {
                            Some(sp) if map.locate(sp, Some(source_home)).is_some() => {
                                self.sample_original(canvas, sp)
                            }
                            _ => continue,
                        }
                    }
                };
                let key = tile_of(x, y);
                let tile = self.tile(canvas, key);
                let i = ((y - key.1 * TILE) * TILE + (x - key.0 * TILE)) as usize;
                if cov <= tile.cover[i] {
                    continue;
                }
                tile.cover[i] = cov;
                tile.colour[i] = ink;
                let o = &tile.orig[i * 4..i * 4 + 4];
                let (oa, m) = (o[3] as f32 / 255.0, cov * opacity * ink[3]);
                if !live {
                    continue;
                }
                let a = oa + m * (1.0 - oa);
                let mut px = [0u8; 4];
                for ch in 0..3 {
                    let v = if a > 0.0 {
                        (o[ch] as f32 * oa * (1.0 - m) + ink[ch] * m) / a
                    } else {
                        ink[ch]
                    };
                    px[ch] = v.round().clamp(0.0, 255.0) as u8;
                }
                px[3] = (a * 255.0).round() as u8;
                canvas.set(x, y, px);
            }
        }
        rect
    }

    /// Finish the stroke: apply Heal, and return the undo step (the layer's
    /// bytes from before, over the touched tiles) and the pixels changed.
    pub fn finish(self, canvas: &mut Canvas) -> Option<(PixelStep, Rect)> {
        let mut bounds = Rect::default();
        for &(tx, ty) in self.tiles.keys() {
            bounds = bounds.union(Rect {
                x0: tx * TILE,
                y0: ty * TILE,
                x1: tx * TILE + TILE,
                y1: ty * TILE + TILE,
            });
        }
        let bounds = bounds.intersect(canvas.bounds());
        if bounds.is_empty() {
            return None;
        }
        let (w, h) = (bounds.width(), bounds.height());
        let mut before = vec![0u8; w * h * 4];
        let mut src = vec![[0f32; 4]; w * h];
        let mut mask = vec![0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (bounds.x0 + x as i32, bounds.y0 + y as i32);
                let o = self.original(canvas, dx, dy);
                let i = y * w + x;
                before[i * 4..i * 4 + 4].copy_from_slice(&o);
                if let Some(t) = self.tiles.get(&tile_of(dx, dy)) {
                    let key = tile_of(dx, dy);
                    let k = ((dy - key.1 * TILE) * TILE + (dx - key.0 * TILE)) as usize;
                    src[i] = t.colour[k];
                    mask[i] = t.cover[k] * self.options.opacity.clamp(0.0, 1.0) as f32;
                }
            }
        }
        let step = PixelStep::from_bytes(bounds, before);
        if self.options.heal != HealMode::Off {
            // Heal blends from the original, so put it back first.
            let mut restore = step.clone();
            restore.swap(canvas);
            blend(canvas, bounds, &src, &mask, self.options.heal);
        }
        Some((step, bounds))
    }
}
