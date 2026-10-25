//! The plane set as bytes, for the project's own image resource: what the
//! user drew, not what was solved from it, so a reopened document solves
//! the same planes again.
//!
//! Big-endian: `u16` version (1), `u32` width and height, `u32` plane
//! count, then each plane in order. A root is `0u8`, its four corners, its
//! anchor, unit and grid size; a related plane is `1u8`, its parent's index
//! (`u32`, an earlier plane), its parent edge (`u8`), angle, side, the four
//! span fractions and the four corners last shown. Every number after the
//! counts is an `f64`.

use super::geometry::P2;
use super::planes::{Shape, VpPlanes};

const VERSION: u16 = 1;
const ROOT: u8 = 0;
const CHILD: u8 = 1;

fn put(out: &mut Vec<u8>, values: &[f64]) {
    for v in values {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn flat(q: &[P2; 4]) -> [f64; 8] {
    std::array::from_fn(|k| if k % 2 == 0 { q[k / 2].0 } else { q[k / 2].1 })
}

struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let (head, rest) = self.0.split_first_chunk::<N>()?;
        self.0 = rest;
        Some(*head)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take::<1>().map(|[b]| b)
    }

    fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_be_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_be_bytes)
    }

    fn f64(&mut self) -> Option<f64> {
        self.take()
            .map(f64::from_be_bytes)
            .filter(|v| v.is_finite())
    }

    fn quad(&mut self) -> Option<[P2; 4]> {
        let mut q = [(0.0, 0.0); 4];
        for p in &mut q {
            *p = (self.f64()?, self.f64()?);
        }
        Some(q)
    }
}

impl VpPlanes {
    pub fn to_bytes(&self) -> Vec<u8> {
        let (width, height) = self.size();
        let mut out = Vec::with_capacity(14 + self.shapes.len() * 137);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&(self.shapes.len() as u32).to_be_bytes());
        for shape in &self.shapes {
            match shape {
                Shape::Root {
                    corners,
                    anchor,
                    unit,
                    grid,
                } => {
                    out.push(ROOT);
                    put(&mut out, &flat(corners));
                    put(&mut out, &[anchor.0, anchor.1, *unit, *grid]);
                }
                Shape::Child {
                    parent,
                    edge,
                    angle,
                    side,
                    span,
                    last,
                } => {
                    out.push(CHILD);
                    out.extend_from_slice(&(*parent as u32).to_be_bytes());
                    out.push(*edge as u8);
                    put(&mut out, &[*angle, *side]);
                    put(&mut out, span);
                    put(&mut out, &flat(last));
                }
            }
        }
        out
    }

    /// The planes `bytes` hold, or `None` for another version, a truncated
    /// or overlong record, a number that is not finite, or a related plane
    /// whose parent does not come before it.
    pub fn from_bytes(bytes: &[u8]) -> Option<VpPlanes> {
        let mut r = Bytes(bytes);
        if r.u16()? != VERSION {
            return None;
        }
        let (width, height) = (r.u32()?, r.u32()?);
        let count = r.u32()? as usize;
        let mut planes = VpPlanes::new(width, height);
        for i in 0..count {
            let shape = match r.u8()? {
                ROOT => {
                    let corners = r.quad()?;
                    let anchor = (r.f64()?, r.f64()?);
                    let unit = r.f64().filter(|&u| u > 0.0)?;
                    let grid = r.f64().filter(|&g| g >= 1.0)?;
                    Shape::Root {
                        corners,
                        anchor,
                        unit,
                        grid,
                    }
                }
                CHILD => {
                    let parent = r.u32().map(|p| p as usize).filter(|&p| p < i)?;
                    let edge = r.u8().map(usize::from).filter(|&e| e < 4)?;
                    let (angle, side) = (r.f64()?, r.f64()?);
                    let span = [r.f64()?, r.f64()?, r.f64()?, r.f64()?];
                    Shape::Child {
                        parent,
                        edge,
                        angle,
                        side,
                        span,
                        last: r.quad()?,
                    }
                }
                _ => return None,
            };
            planes.shapes.push(shape);
        }
        if !r.0.is_empty() {
            return None;
        }
        planes.solve();
        Some(planes)
    }
}
