//! The retouch tools over the session's working copy of the layer: Paste,
//! the Marquee and Transform (floating selections), Stamp and Brush. Every
//! gesture records one step of the dialog's undo.

use std::sync::Arc;

use super::canvas::{Canvas, PixelStep, Rect};
use super::editor::{State, VpEditor};
use super::float::{self, Floating, Marquee, Texture};
use super::geometry::{Validity, P2};
use super::heal::{blend, HealMode};
use super::space::{family_at, FamilyMap};
use super::stroke::{Ink, PaintOptions, Stroke};

/// Feather, Opacity, Heal and Move Mode for the Marquee.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarqueeOptions {
    pub feather: f64,
    pub opacity: f64,
    pub heal: HealMode,
    /// Move Mode: Source fills the marquee from where the pointer moves to.
    pub source: bool,
}

impl Default for MarqueeOptions {
    fn default() -> Self {
        MarqueeOptions {
            feather: 0.0,
            opacity: 1.0,
            heal: HealMode::Off,
            source: false,
        }
    }
}

/// Why Paste did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteError {
    NoPlane,
    Empty,
}

#[derive(Clone, Copy)]
enum Shape {
    Marquee { start: P2 },
    MoveMarquee { grab: P2 },
    Fill { grab: P2, at: Marquee },
    MoveFloat { grab: P2 },
    Scale { from: P2, scale: P2 },
    Rotate { from: f64, angle: f64 },
}

enum Gesture {
    Shape(Shape),
    Stroke(Box<Stroke>),
}

pub struct Retouch {
    pub(super) canvas: Canvas,
    original: Arc<Vec<u8>>,
    pub(super) backdrop: Option<Arc<Vec<u8>>>,
    gray: bool,
    pub marquee: MarqueeOptions,
    pub stamp: PaintOptions,
    pub aligned: bool,
    pub brush: PaintOptions,
    pub colour: [u8; 3],
    stamp_source: Option<(usize, usize, P2)>,
    stamp_anchor: Option<P2>,
    gesture: Option<(Gesture, State)>,
}

fn sub(a: P2, b: P2) -> P2 {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: P2, b: P2) -> P2 {
    (a.0 + b.0, a.1 + b.1)
}

impl VpEditor {
    /// Give the session the layer's pixels and the document composite with
    /// the layer hidden (straight RGBA, document size). `gray` reduces
    /// Brush Color to its luminance.
    pub fn attach(&mut self, canvas: Canvas, backdrop: Option<Vec<u8>>, gray: bool) {
        self.retouch = Some(Retouch {
            original: Arc::new(canvas.rgba.clone()),
            canvas,
            backdrop: backdrop.map(Arc::new),
            gray,
            marquee: MarqueeOptions::default(),
            stamp: PaintOptions::default(),
            aligned: true,
            brush: PaintOptions::default(),
            colour: [0, 0, 0],
            stamp_source: None,
            stamp_anchor: None,
            gesture: None,
        });
        self.touch_all();
    }

    pub fn retouch(&self) -> Option<&Retouch> {
        self.retouch.as_ref()
    }

    pub fn retouch_mut(&mut self) -> Option<&mut Retouch> {
        self.retouch.as_mut()
    }

    pub(super) fn map_of(&self, root: usize) -> Option<FamilyMap> {
        FamilyMap::new(&self.state.planes, root)
    }

    fn float_rect(&self) -> Rect {
        let Some(f) = &self.state.float else {
            return Rect::default();
        };
        self.map_of(f.family).map_or(Rect::default(), |m| {
            float::footprint(&m, &f.flat_corners(), Some(f.home))
        })
    }

    fn touch(&mut self, r: Rect) {
        self.dirty = self.dirty.union(r);
    }

    pub fn has_float(&self) -> bool {
        self.state.float.is_some()
    }

    pub fn has_marquee(&self) -> bool {
        self.state.marquee.is_some()
    }

    pub fn floating(&self) -> Option<&Floating> {
        self.state.float.as_ref()
    }

    pub fn marquee(&self) -> Option<&Marquee> {
        self.state.marquee.as_ref()
    }

    /// The floating selection's outline in the image.
    pub fn float_outline(&self) -> Vec<P2> {
        let Some(f) = &self.state.float else {
            return Vec::new();
        };
        self.map_of(f.family).map_or_else(Vec::new, |m| {
            float::outline(&m, &f.flat_corners(), Some(f.home))
        })
    }

    /// The floating selection's corners in the image (Transform's handles).
    pub fn float_corners(&self) -> Vec<P2> {
        let Some(f) = &self.state.float else {
            return Vec::new();
        };
        let Some(m) = self.map_of(f.family) else {
            return Vec::new();
        };
        f.flat_corners()
            .iter()
            .filter_map(|&c| m.to_image(c, Some(f.home)))
            .collect()
    }

    pub fn marquee_outline(&self) -> Vec<P2> {
        let Some(q) = &self.state.marquee else {
            return Vec::new();
        };
        self.map_of(q.family)
            .map_or_else(Vec::new, |m| float::outline(&m, &q.corners(), Some(q.home)))
    }

    /// The valid plane a paste goes into: the active one, else the newest.
    fn paste_plane(&self) -> Option<usize> {
        let planes = &self.state.planes;
        self.active
            .filter(|&i| planes.validity(i) == Validity::Valid)
            .or_else(|| {
                (0..planes.len())
                    .rev()
                    .find(|&i| planes.validity(i) == Validity::Valid)
            })
    }

    /// Paste `tex` as a floating selection centred in the active plane, one
    /// texel per flat unit, shrunk to fit within 80 % of the plane.
    pub fn paste(&mut self, tex: Texture) -> Result<(), PasteError> {
        if tex.width == 0 || tex.height == 0 {
            return Err(PasteError::Empty);
        }
        let plane = self.paste_plane().ok_or(PasteError::NoPlane)?;
        let root = self.state.planes.root_of(plane);
        let map = self.map_of(root).ok_or(PasteError::NoPlane)?;
        let corners = map.flat_corners(plane).ok_or(PasteError::NoPlane)?;
        self.drop_float();
        let lo = corners.iter().fold((f64::INFINITY, f64::INFINITY), |a, c| {
            (a.0.min(c.0), a.1.min(c.1))
        });
        let hi = corners
            .iter()
            .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, c| {
                (a.0.max(c.0), a.1.max(c.1))
            });
        let fit = (0.8 * (hi.0 - lo.0) / tex.width as f64)
            .min(0.8 * (hi.1 - lo.1) / tex.height as f64)
            .min(1.0);
        let before = self.state.clone();
        self.state.marquee = None;
        self.state.float = Some(Floating {
            tex,
            family: root,
            home: plane,
            centre: ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0),
            scale: (fit, fit),
            angle: 0.0,
            flip: false,
            flop: false,
        });
        self.record(before, None);
        let r = self.float_rect();
        self.touch(r);
        Ok(())
    }

    /// Composite the floating selection into the layer (Feather, Opacity and
    /// Heal from the Marquee options) as one undo step.
    pub fn drop_float(&mut self) -> bool {
        let Some(f) = self.state.float.clone() else {
            return false;
        };
        let before = self.state.clone();
        self.state.float = None;
        let step = self.composite(&f);
        self.record(before, step);
        true
    }

    /// Put `f` down into the canvas; the pixels it held before.
    fn composite(&mut self, f: &Floating) -> Option<PixelStep> {
        let map = self.map_of(f.family)?;
        let retouch = self.retouch.as_mut()?;
        let rect = float::footprint(&map, &f.flat_corners(), Some(f.home))
            .intersect(retouch.canvas.bounds());
        if rect.is_empty() {
            return None;
        }
        let o = retouch.marquee;
        let (src, mask) = float::render(&map, f, o.feather, o.opacity, rect);
        let step = PixelStep::capture(&retouch.canvas, rect);
        // Heal reads a ring of fixed pixels round the footprint.
        let (rect, src, mask) = pad(rect, &src, &mask);
        blend(&mut retouch.canvas, rect, &src, &mask, o.heal);
        self.touch(rect);
        Some(step)
    }

    /// Drop the floating selection and clear the marquee (Ctrl+D).
    pub fn deselect(&mut self) {
        self.drop_float();
        if self.state.marquee.is_some() {
            let before = self.state.clone();
            self.state.marquee = None;
            self.record(before, None);
        }
    }

    /// Whether image point `p` lies on the floating selection.
    fn on_float(&self, p: P2) -> Option<P2> {
        let f = self.state.float.as_ref()?;
        let (_, flat) = self.map_of(f.family)?.locate(p, Some(f.home))?;
        let t = f.to_texel(flat)?;
        (t.0 >= 0.0 && t.1 >= 0.0 && t.0 <= f.tex.width as f64 && t.1 <= f.tex.height as f64)
            .then_some(flat)
    }

    /// Marquee press: on the float, move it (Alt: leave a copy behind); in
    /// the marquee, Alt floats a copy, Ctrl or Source fills it from under
    /// the pointer, otherwise the marquee moves; elsewhere a new marquee
    /// starts and any float is dropped.
    pub fn marquee_press(&mut self, p: P2, alt: bool, ctrl: bool) -> bool {
        if self.retouch.is_none() {
            return false;
        }
        let before = self.state.clone();
        if let Some(flat) = self.on_float(p) {
            if alt {
                let f = self.state.float.clone().expect("on_float found it");
                if let Some(step) = self.composite(&f) {
                    self.record(before.clone(), Some(step));
                }
            }
            let centre = self.state.float.as_ref().expect("on_float found it").centre;
            return self.start(Shape::MoveFloat {
                grab: sub(centre, flat),
            });
        }
        if let Some(q) = self.state.marquee {
            if let Some(map) = self.map_of(q.family) {
                if let Some((_, flat)) = map.locate(p, Some(q.home)).filter(|(_, f)| q.contains(*f))
                {
                    let canvas = &self.retouch.as_ref().expect("checked").canvas;
                    let source = self.retouch.as_ref().expect("checked").marquee.source;
                    if alt || ctrl || source {
                        let Some(tex) = float::lift(&map, &q, canvas, (0.0, 0.0)) else {
                            return false;
                        };
                        let (lo, hi) = (q.min(), q.max());
                        self.state.float = Some(Floating {
                            tex,
                            family: q.family,
                            home: q.home,
                            centre: ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0),
                            scale: (1.0, 1.0),
                            angle: 0.0,
                            flip: false,
                            flop: false,
                        });
                        if alt {
                            self.state.marquee = None;
                            let centre = self.state.float.as_ref().expect("just set").centre;
                            return self.start(Shape::MoveFloat {
                                grab: sub(centre, flat),
                            });
                        }
                        return self.start(Shape::Fill { grab: flat, at: q });
                    }
                    return self.start(Shape::MoveMarquee { grab: flat });
                }
            }
        }
        self.drop_float();
        let before = self.state.clone();
        let Some((map, plane, flat)) = family_at(&self.state.planes, p, self.active) else {
            if self.state.marquee.take().is_some() {
                self.record(before, None);
            }
            return false;
        };
        self.state.marquee = Some(Marquee {
            family: map.root,
            home: plane,
            a: flat,
            b: flat,
        });
        self.start(Shape::Marquee { start: flat })
    }

    fn start(&mut self, g: Shape) -> bool {
        let before = self.state.clone();
        self.retouch.as_mut().expect("tools need a canvas").gesture =
            Some((Gesture::Shape(g), before));
        true
    }

    fn shape(&self) -> Option<Shape> {
        match self.retouch.as_ref()?.gesture.as_ref()? {
            (Gesture::Shape(s), _) => Some(*s),
            _ => None,
        }
    }

    fn set_shape(&mut self, s: Shape) {
        if let Some((g, _)) = self.retouch.as_mut().and_then(|r| r.gesture.as_mut()) {
            *g = Gesture::Shape(s);
        }
    }

    /// Marquee drag to `p`; `square` (Shift) keeps a new marquee square.
    pub fn marquee_drag(&mut self, p: P2, square: bool) {
        let Some(shape) = self.shape() else {
            return;
        };
        match shape {
            Shape::Marquee { start } => {
                let Some(q) = self.state.marquee else {
                    return;
                };
                let Some((_, flat)) = self
                    .map_of(q.family)
                    .and_then(|m| m.locate(p, Some(q.home)))
                else {
                    return;
                };
                let mut d = sub(flat, start);
                if square {
                    let side = d.0.abs().max(d.1.abs());
                    d = (side.copysign(d.0), side.copysign(d.1));
                }
                self.state.marquee = Some(Marquee {
                    b: add(start, d),
                    ..q
                });
            }
            Shape::MoveMarquee { grab } => {
                let Some(q) = self.state.marquee else {
                    return;
                };
                let Some((_, flat)) = self
                    .map_of(q.family)
                    .and_then(|m| m.locate(p, Some(q.home)))
                else {
                    return;
                };
                let d = sub(flat, grab);
                self.set_shape(Shape::MoveMarquee { grab: flat });
                self.state.marquee = Some(Marquee {
                    a: add(q.a, d),
                    b: add(q.b, d),
                    ..q
                });
            }
            Shape::Fill { grab, at } => {
                let Some(map) = self.map_of(at.family) else {
                    return;
                };
                let Some((_, flat)) = map.locate(p, Some(at.home)) else {
                    return;
                };
                let old = self.float_rect();
                let canvas = &self.retouch.as_ref().expect("checked").canvas;
                if let (Some(tex), Some(f)) = (
                    float::lift(&map, &at, canvas, sub(flat, grab)),
                    self.state.float.as_mut(),
                ) {
                    f.tex = tex;
                }
                self.touch(old);
            }
            Shape::MoveFloat { grab } => self.move_float(p, grab),
            Shape::Scale { .. } | Shape::Rotate { .. } => {}
        }
    }

    fn move_float(&mut self, p: P2, grab: P2) {
        let Some(f) = self.state.float.clone() else {
            return;
        };
        let old = self.float_rect();
        let here = self
            .map_of(f.family)
            .and_then(|m| {
                m.locate(p, Some(f.home))
                    .map(|(plane, flat)| (f.family, plane, flat))
            })
            .or_else(|| {
                family_at(&self.state.planes, p, None).map(|(m, plane, flat)| (m.root, plane, flat))
            });
        let Some((family, plane, flat)) = here else {
            return;
        };
        if let Some(f) = self.state.float.as_mut() {
            if family != f.family {
                f.family = family;
                f.home = plane;
            }
            f.centre = add(flat, grab);
        }
        let new = self.float_rect();
        self.touch(old.union(new));
    }

    /// End a Marquee or Transform gesture as one undo step.
    pub fn gesture_release(&mut self) {
        let Some((gesture, before)) = self.retouch.as_mut().and_then(|r| r.gesture.take()) else {
            return;
        };
        if let Gesture::Stroke(stroke) = gesture {
            let canvas = &mut self.retouch.as_mut().expect("checked").canvas;
            if let Some((step, rect)) = stroke.finish(canvas) {
                self.touch(rect);
                self.record(before, Some(step));
            }
            return;
        }
        if let Some(q) = self.state.marquee {
            // A click without a drag leaves no marquee.
            if q.a == q.b {
                self.state.marquee = None;
            }
        }
        self.record(before, None);
    }

    /// Double-click with the Marquee: select the whole plane under `p`.
    pub fn select_plane(&mut self, p: P2) -> bool {
        let Some((map, plane, _)) = family_at(&self.state.planes, p, self.active) else {
            return false;
        };
        let Some(c) = map.flat_corners(plane) else {
            return false;
        };
        let lo = c.iter().fold((f64::INFINITY, f64::INFINITY), |a, c| {
            (a.0.min(c.0), a.1.min(c.1))
        });
        let hi = c
            .iter()
            .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, c| {
                (a.0.max(c.0), a.1.max(c.1))
            });
        let before = self.state.clone();
        self.state.marquee = Some(Marquee {
            family: map.root,
            home: plane,
            a: lo,
            b: hi,
        });
        self.record(before, None);
        true
    }

    /// Transform press within `tol` px of a corner (scale), a little
    /// beyond one (rotate), or inside the float (move).
    pub fn transform_press(&mut self, p: P2, tol: f64) -> bool {
        let Some(f) = self.state.float.clone() else {
            return false;
        };
        let Some(map) = self.map_of(f.family) else {
            return false;
        };
        let corners = self.float_corners();
        let near = corners
            .iter()
            .map(|c| (c.0 - p.0).hypot(c.1 - p.1))
            .fold(f64::INFINITY, f64::min);
        let flat = map.locate(p, Some(f.home)).map(|(_, flat)| flat);
        if near <= tol {
            if let Some(flat) = flat {
                return self.start(Shape::Scale {
                    from: sub(flat, f.centre),
                    scale: f.scale,
                });
            }
        }
        if let Some(flat) = self.on_float(p) {
            return self.start(Shape::MoveFloat {
                grab: sub(f.centre, flat),
            });
        }
        if near <= tol * 4.0 {
            if let Some(flat) = flat.or_else(|| self.extrapolate(&map, &f, p)) {
                let d = sub(flat, f.centre);
                return self.start(Shape::Rotate {
                    from: d.1.atan2(d.0),
                    angle: f.angle,
                });
            }
        }
        false
    }

    /// The flat point under `p` through the float's own plane, even past it.
    fn extrapolate(&self, map: &FamilyMap, f: &Floating, p: P2) -> Option<P2> {
        // Walk toward the float's centre until the pointer's ray lands in a
        // plane, then carry the step back out.
        let c = map.to_image(f.centre, Some(f.home))?;
        (1..=8).find_map(|k| {
            let t = k as f64 / 8.0;
            let q = (p.0 + (c.0 - p.0) * t, p.1 + (c.1 - p.1) * t);
            let (_, fq) = map.locate(q, Some(f.home))?;
            let dir = sub(fq, f.centre);
            Some(add(
                f.centre,
                (dir.0 / (1.0 - t).max(1e-3), dir.1 / (1.0 - t).max(1e-3)),
            ))
        })
    }

    pub fn transform_drag(&mut self, p: P2, uniform: bool) {
        let Some(shape) = self.shape() else {
            return;
        };
        let Some(f) = self.state.float.clone() else {
            return;
        };
        let Some(map) = self.map_of(f.family) else {
            return;
        };
        let flat = map
            .locate(p, Some(f.home))
            .map(|(_, q)| q)
            .or_else(|| self.extrapolate(&map, &f, p));
        let old = self.float_rect();
        match shape {
            Shape::MoveFloat { grab } => return self.move_float(p, grab),
            Shape::Scale { from, scale } => {
                let Some(q) = flat else {
                    return;
                };
                let (s, c) = f.angle.sin_cos();
                let local = |d: P2| (d.0 * c + d.1 * s, -d.0 * s + d.1 * c);
                let (d0, d1) = (local(from), local(sub(q, f.centre)));
                let mut k = (
                    if d0.0.abs() > 1e-6 { d1.0 / d0.0 } else { 1.0 },
                    if d0.1.abs() > 1e-6 { d1.1 / d0.1 } else { 1.0 },
                );
                if uniform {
                    let u = (d1.0.hypot(d1.1) / d0.0.hypot(d0.1).max(1e-6)).max(1e-3);
                    k = (u, u);
                }
                let scale = (
                    (scale.0 * k.0.abs()).max(1e-3),
                    (scale.1 * k.1.abs()).max(1e-3),
                );
                if let Some(f) = self.state.float.as_mut() {
                    f.scale = scale;
                }
            }
            Shape::Rotate { from, angle } => {
                let Some(q) = flat else {
                    return;
                };
                let d = sub(q, f.centre);
                if let Some(f) = self.state.float.as_mut() {
                    f.angle = angle + d.1.atan2(d.0) - from;
                }
            }
            _ => return,
        }
        let new = self.float_rect();
        self.touch(old.union(new));
    }

    /// Transform's Flip (left–right) or Flop (top–bottom).
    pub fn mirror(&mut self, horizontal: bool) -> bool {
        if self.state.float.is_none() {
            return false;
        }
        let before = self.state.clone();
        if let Some(f) = self.state.float.as_mut() {
            if horizontal {
                f.flip = !f.flip;
            } else {
                f.flop = !f.flop;
            }
        }
        self.record(before, None);
        let r = self.float_rect();
        self.touch(r);
        true
    }

    /// Alt-click with the Stamp: clone from image point `p`.
    pub fn set_stamp_source(&mut self, p: P2) -> bool {
        let Some((map, plane, flat)) = family_at(&self.state.planes, p, self.active) else {
            return false;
        };
        let Some(retouch) = self.retouch.as_mut() else {
            return false;
        };
        retouch.stamp_source = Some((map.root, plane, flat));
        retouch.stamp_anchor = None;
        true
    }

    pub fn has_stamp_source(&self) -> bool {
        self.retouch
            .as_ref()
            .is_some_and(|r| r.stamp_source.is_some())
    }

    /// Start a Brush (`brush`) or Stamp stroke at image point `p`. False
    /// outside every plane, or for a Stamp with no source yet.
    pub fn stroke_press(&mut self, brush: bool, p: P2) -> bool {
        self.drop_float();
        let Some((map, plane, flat)) = family_at(&self.state.planes, p, self.active) else {
            return false;
        };
        let Some(retouch) = self.retouch.as_mut() else {
            return false;
        };
        let (ink, source_map, options) = if brush {
            let mut c = retouch.colour;
            if retouch.gray {
                let l =
                    (0.299 * c[0] as f64 + 0.587 * c[1] as f64 + 0.114 * c[2] as f64).round() as u8;
                c = [l, l, l];
            }
            (Ink::Colour(c), None, retouch.brush)
        } else {
            let Some((family, home, source)) = retouch.stamp_source else {
                return false;
            };
            let anchor = match retouch.stamp_anchor {
                Some(a) if retouch.aligned => a,
                _ => flat,
            };
            retouch.stamp_anchor = Some(anchor);
            let ink = Ink::Clone {
                source_family: family,
                source_home: home,
                source,
                anchor,
            };
            (
                ink,
                FamilyMap::new(&self.state.planes, family),
                retouch.stamp,
            )
        };
        let mut stroke = Stroke::new(map, source_map, plane, ink, options);
        let before = self.state.clone();
        let retouch = self.retouch.as_mut().expect("checked");
        let dirty = stroke.to(&mut retouch.canvas, p);
        retouch.gesture = Some((Gesture::Stroke(Box::new(stroke)), before));
        self.touch(dirty);
        true
    }

    pub fn stroke_drag(&mut self, p: P2) {
        let Some(retouch) = self.retouch.as_mut() else {
            return;
        };
        if let Some((Gesture::Stroke(stroke), _)) = retouch.gesture.as_mut() {
            let dirty = stroke.to(&mut retouch.canvas, p);
            self.touch(dirty);
        }
    }

    /// Whether the layer differs from how the session found it.
    pub fn pixels_changed(&self) -> bool {
        self.retouch
            .as_ref()
            .is_some_and(|r| r.canvas.rgba != *r.original)
    }

    /// The changed part of the layer, in layer pixels: `(x, y, w, h, rgba)`.
    pub fn changed_patch(&self) -> Option<(usize, usize, usize, usize, Vec<u8>)> {
        let r = self.retouch.as_ref()?;
        let (w, h) = (r.canvas.width, r.canvas.height);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                if r.canvas.rgba[i..i + 4] != r.original[i..i + 4] {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        if x1 <= x0 {
            return None;
        }
        let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
        for y in y0..y1 {
            out.extend_from_slice(&r.canvas.rgba[(y * w + x0) * 4..(y * w + x1) * 4]);
        }
        Some((x0, y0, x1 - x0, y1 - y0, out))
    }
}

/// `rect` grown by one pixel of zero cover on every side.
fn pad(rect: Rect, src: &[[f32; 4]], mask: &[f32]) -> (Rect, Vec<[f32; 4]>, Vec<f32>) {
    let (w, h) = (rect.width(), rect.height());
    let grown = Rect {
        x0: rect.x0 - 1,
        y0: rect.y0 - 1,
        x1: rect.x1 + 1,
        y1: rect.y1 + 1,
    };
    let gw = w + 2;
    let mut s = vec![[0f32; 4]; gw * (h + 2)];
    let mut m = vec![0f32; gw * (h + 2)];
    for y in 0..h {
        for x in 0..w {
            s[(y + 1) * gw + x + 1] = src[y * w + x];
            m[(y + 1) * gw + x + 1] = mask[y * w + x];
        }
    }
    (grown, s, m)
}
