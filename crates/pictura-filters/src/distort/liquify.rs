//! Liquify (`FILT-090`): a displacement mesh edited by brush dabs, then used
//! to resample the layer once.
//!
//! The mesh holds, for every node of a regular grid (one node per `cell`
//! layer pixels), the offset `d` such that output pixel `p` shows the source
//! at `p + d(p)` — an inverse map, so resampling never leaves holes. Every
//! tool is written as "where does the new output at `p` come from in the
//! current output": `q = M(p)` (Forward Warp `p - kΔ`, Twirl a rotation about
//! the brush centre, Pucker/Bloat a scaling about it), so the new offset is
//! `d'(p) = q + d(q) - p`, read bilinearly from the mesh as it was before the
//! dab. Reconstruct scales `d` toward zero; Freeze/Thaw paint a mask that
//! scales every tool down.
//!
//! Ported from photocraft's Filter ▸ Liquify (ArtCraft Team and the PhotoCraft
//! contributors, MIT OR Apache-2.0). Mirror and Turbulence are Kooka's own.
//! Adobe's mesh math is closed: the tool strengths are tuned by eye, an
//! approximation, not parity.

use pictura_core::PsdRect;

mod resample;
#[cfg(test)]
mod tests;

pub use resample::{liquify, LiquifyProxy};

/// CS6's largest Liquify brush.
pub const LIQUIFY_MAX_BRUSH: f64 = 15_000.0;

/// The distortion and mask tools of the Liquify tool column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquifyTool {
    ForwardWarp,
    Reconstruct,
    Twirl,
    Pucker,
    Bloat,
    PushLeft,
    Mirror,
    Turbulence,
    Freeze,
    Thaw,
}

impl LiquifyTool {
    /// Tools that keep working while the brush is held still, at Brush Rate.
    pub fn acts_while_held(self) -> bool {
        !matches!(
            self,
            LiquifyTool::ForwardWarp | LiquifyTool::PushLeft | LiquifyTool::Mirror
        )
    }
}

/// One brush: a tool and the Tool Options sliders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquifyBrush {
    pub tool: LiquifyTool,
    /// Diameter in layer pixels, `1..=LIQUIFY_MAX_BRUSH`.
    pub size: f64,
    /// Edge feathering, 0 (hard) ..= 100 (soft).
    pub density: f64,
    /// Strength per drag, 0 ..= 100.
    pub pressure: f64,
    /// Speed of the tools that act while held, 0 ..= 100.
    pub rate: f64,
    /// Turbulence's scramble tightness, 0 ..= 100.
    pub jitter: f64,
    /// `Alt`: Twirl turns counterclockwise, Pucker bloats and Bloat puckers,
    /// Push Left pushes right, Mirror reflects the other side.
    pub reverse: bool,
}

impl LiquifyBrush {
    pub fn new(tool: LiquifyTool, size: f64) -> Self {
        Self {
            tool,
            size,
            density: 50.0,
            pressure: 100.0,
            rate: 80.0,
            jitter: 50.0,
            reverse: false,
        }
    }

    fn radius(&self) -> f64 {
        (self.size.clamp(1.0, LIQUIFY_MAX_BRUSH) / 2.0).max(0.5)
    }
}

/// The displacement mesh plus the freeze mask over a `width`×`height` layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LiquifyMesh {
    width: u32,
    height: u32,
    /// Layer pixels per node.
    cell: f64,
    cols: usize,
    rows: usize,
    /// Inverse displacement per node, row-major.
    d: Vec<[f32; 2]>,
    /// Freeze per node, 0 free ..= 1 frozen.
    freeze: Vec<f32>,
}

/// Layer pixels per node: 2 up to about 4 MP, then coarser so a PSB-sized
/// layer's mesh stays a few tens of megabytes.
fn auto_cell(width: u32, height: u32) -> f64 {
    let area = u64::from(width) * u64::from(height);
    if area <= 4_200_000 {
        2.0
    } else if area <= 32_000_000 {
        4.0
    } else {
        8.0
    }
}

impl LiquifyMesh {
    /// An identity mesh over a `width`×`height` layer (each at least 1).
    pub fn new(width: u32, height: u32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let cell = auto_cell(width, height);
        let cols = (f64::from(width) / cell).ceil() as usize + 2;
        let rows = (f64::from(height) / cell).ceil() as usize + 2;
        Self {
            width,
            height,
            cell,
            cols,
            rows,
            d: vec![[0.0; 2]; cols * rows],
            freeze: vec![0.0; cols * rows],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_identity(&self) -> bool {
        self.d.iter().all(|v| v[0] == 0.0 && v[1] == 0.0)
    }

    /// Layer position of node `(i, j)`; node 0 sits on the first pixel centre.
    fn node_pos(&self, i: usize, j: usize) -> [f64; 2] {
        [0.5 + i as f64 * self.cell, 0.5 + j as f64 * self.cell]
    }

    fn to_grid(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - 0.5) / self.cell, (y - 0.5) / self.cell)
    }

    /// Bilinear displacement at a layer point (zero beyond the mesh).
    pub fn displacement(&self, x: f64, y: f64) -> [f64; 2] {
        let (gx, gy) = self.to_grid(x, y);
        bilinear(&self.d, self.cols, self.rows, gx, gy)
    }

    /// Bilinear freeze at a layer point, 0 ..= 1.
    pub fn freeze_at(&self, x: f64, y: f64) -> f32 {
        let (gx, gy) = self.to_grid(x, y);
        let gx = gx.clamp(0.0, (self.cols - 1) as f64);
        let gy = gy.clamp(0.0, (self.rows - 1) as f64);
        let (i, j) = (
            (gx as usize).min(self.cols - 2),
            (gy as usize).min(self.rows - 2),
        );
        let (fx, fy) = ((gx - i as f64) as f32, (gy - j as f64) as f32);
        let at = |i: usize, j: usize| self.freeze[j * self.cols + i];
        (at(i, j) * (1.0 - fx) + at(i + 1, j) * fx) * (1.0 - fy)
            + (at(i, j + 1) * (1.0 - fx) + at(i + 1, j + 1) * fx) * fy
    }

    /// Take `other`'s freeze mask when it covers the same layer size.
    pub fn copy_freeze_from(&mut self, other: &LiquifyMesh) {
        if (other.width, other.height) == (self.width, self.height) {
            self.freeze.clone_from(&other.freeze);
        }
    }

    /// The Reconstruct button: scale every unfrozen offset toward zero by
    /// `amount` percent (100 restores the unfrozen picture exactly).
    pub fn reconstruct(&mut self, amount: f64) {
        let k = (amount / 100.0).clamp(0.0, 1.0) as f32;
        for (v, f) in self.d.iter_mut().zip(&self.freeze) {
            let m = 1.0 - k * (1.0 - f);
            v[0] *= m;
            v[1] *= m;
        }
    }

    /// Restore All: drop every distortion, frozen areas included. The mask
    /// stays.
    pub fn restore_all(&mut self) {
        self.d.fill([0.0; 2]);
    }

    /// Mask All.
    pub fn freeze_all(&mut self) {
        self.freeze.fill(1.0);
    }

    /// Mask None.
    pub fn thaw_all(&mut self) {
        self.freeze.fill(0.0);
    }

    /// Invert All.
    pub fn invert_freeze(&mut self) {
        self.freeze.iter_mut().for_each(|f| *f = 1.0 - *f);
    }

    /// This mesh stretched to a `width`×`height` layer, offsets scaled with
    /// it — Load Mesh's "scaled to fit", corner-anchored. The freeze mask is
    /// not carried.
    pub fn scaled_to(&self, width: u32, height: u32) -> Self {
        let mut out = Self::new(width, height);
        if out.width == self.width && out.height == self.height {
            out.d.clone_from(&self.d);
            return out;
        }
        let sx = f64::from(self.width) / f64::from(out.width);
        let sy = f64::from(self.height) / f64::from(out.height);
        for j in 0..out.rows {
            for i in 0..out.cols {
                let p = out.node_pos(i, j);
                let v = self.displacement(p[0] * sx, p[1] * sy);
                out.d[j * out.cols + i] = [(v[0] / sx) as f32, (v[1] / sy) as f32];
            }
        }
        out
    }

    /// Starts a stroke at `p` (`[x, y, pressure 0..=1]`, layer pixels): the
    /// tools that act while held dab once there. Returns the layer rectangle
    /// that changed.
    pub fn stroke_begin(&mut self, brush: &LiquifyBrush, p: [f64; 3]) -> Option<PsdRect> {
        brush
            .tool
            .acts_while_held()
            .then(|| self.dab(brush, [p[0], p[1]], [0.0, 0.0], p[2]))
            .flatten()
    }

    /// One Brush Rate tick with the brush held still at `p`.
    pub fn hold(&mut self, brush: &LiquifyBrush, p: [f64; 3]) -> Option<PsdRect> {
        self.stroke_begin(brush, p)
    }

    /// Continues a stroke from `a` to `b`, dabbing every fifth of the brush
    /// radius.
    pub fn stroke_to(&mut self, brush: &LiquifyBrush, a: [f64; 3], b: [f64; 3]) -> Option<PsdRect> {
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        if len.is_nan() || len <= 0.0 {
            return None;
        }
        let held = brush.tool.acts_while_held();
        let spacing = (brush.radius() * 0.2).max(0.5);
        let steps = (len / spacing).ceil().max(1.0) as usize;
        let (k0, k1) = self.reachable_steps(a, b, steps, brush.radius(), held)?;
        let mut dirty = None;
        for k in k0.max(1)..=k1.min(steps) {
            let t0 = (k - 1) as f64 / steps as f64;
            let t1 = k as f64 / steps as f64;
            let c = [a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1];
            let delta = [(b[0] - a[0]) * (t1 - t0), (b[1] - a[1]) * (t1 - t0)];
            let pressure = a[2] + (b[2] - a[2]) * t1;
            // The dragging tools centre the dab where the brush was: they
            // carry what was under it.
            let centre = if held {
                c
            } else {
                [c[0] - delta[0], c[1] - delta[1]]
            };
            dirty = union(dirty, self.dab(brush, centre, delta, pressure));
        }
        dirty
    }

    /// The steps `k0..=k1` of a segment walked in `steps` whose dab can reach
    /// a node, or `None`. A segment to a far-off point would otherwise walk
    /// one dab per spacing step along its whole length.
    fn reachable_steps(
        &self,
        a: [f64; 3],
        b: [f64; 3],
        steps: usize,
        r: f64,
        held: bool,
    ) -> Option<(usize, usize)> {
        let pad = r + 2.0 * self.cell;
        let lo = [0.5 - pad, 0.5 - pad];
        let hi = [
            lo[0] + (self.cols - 1) as f64 * self.cell + 2.0 * pad,
            lo[1] + (self.rows - 1) as f64 * self.cell + 2.0 * pad,
        ];
        let (n, o) = (steps as f64, if held { 0.0 } else { 1.0 });
        let (mut k0, mut k1) = (1.0f64, n);
        for ax in 0..2 {
            let d = b[ax] - a[ax];
            if d == 0.0 {
                if a[ax].is_nan() || a[ax] < lo[ax] || a[ax] > hi[ax] {
                    return None;
                }
                continue;
            }
            let (u0, u1) = ((lo[ax] - a[ax]) / d, (hi[ax] - a[ax]) / d);
            k0 = k0.max((u0.min(u1) * n + o).floor() - 1.0);
            k1 = k1.min((u0.max(u1) * n + o).ceil() + 1.0);
        }
        if k0.is_nan() || k1.is_nan() || k0 > k1 {
            return None;
        }
        Some((k0 as usize, k1 as usize))
    }

    /// One dab centred on `c` with brush motion `delta`.
    fn dab(
        &mut self,
        brush: &LiquifyBrush,
        c: [f64; 2],
        delta: [f64; 2],
        point_pressure: f64,
    ) -> Option<PsdRect> {
        let r = brush.radius();
        let strength = (brush.pressure / 100.0).clamp(0.0, 1.0) * point_pressure.clamp(0.0, 1.0);
        let rate = (brush.rate / 100.0).clamp(0.0, 1.0);
        let density = (brush.density / 100.0).clamp(0.0, 1.0);
        if strength.is_nan() || strength <= 0.0 || !c[0].is_finite() || !c[1].is_finite() {
            return None;
        }
        let (gx0, gy0) = self.to_grid(c[0] - r, c[1] - r);
        let (gx1, gy1) = self.to_grid(c[0] + r, c[1] + r);
        let i0 = gx0.floor().max(0.0) as usize;
        let j0 = gy0.floor().max(0.0) as usize;
        let i1 = (gx1.ceil().max(-1.0) as isize).min(self.cols as isize - 1);
        let j1 = (gy1.ceil().max(-1.0) as isize).min(self.rows as isize - 1);
        if i1 < i0 as isize || j1 < j0 as isize {
            return None;
        }
        let (i1, j1) = (i1 as usize, j1 as usize);
        let mirror_normal = mirror_normal(brush, delta);
        if brush.tool == LiquifyTool::Mirror && mirror_normal.is_none() {
            return None;
        }
        // Every node reads the mesh as it was before this dab.
        let before = Window::capture(&self.d, self.cols, (i0, j0, i1, j1), r, delta, self.cell);
        let old =
            |gx: f64, gy: f64| -> [f64; 2] { before.sample(&self.d, self.cols, self.rows, gx, gy) };
        let inv_cell = 1.0 / self.cell;
        let mut writes: Vec<(usize, [f32; 2], f32)> = Vec::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let p = self.node_pos(i, j);
                let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
                let t2 = (dx * dx + dy * dy) / (r * r);
                if t2 >= 1.0 {
                    continue;
                }
                let soft = (1.0 - t2) * (1.0 - t2);
                let fall = (1.0 - density) + density * soft;
                let idx = j * self.cols + i;
                let k_raw = strength * fall;
                if matches!(brush.tool, LiquifyTool::Freeze | LiquifyTool::Thaw) {
                    let f = f64::from(self.freeze[idx]);
                    let v = if brush.tool == LiquifyTool::Freeze {
                        (f + k_raw).min(1.0)
                    } else {
                        (f - k_raw).max(0.0)
                    };
                    writes.push((idx, self.d[idx], v as f32));
                    continue;
                }
                let k = k_raw * (1.0 - f64::from(self.freeze[idx]));
                if k <= 0.0 {
                    continue;
                }
                let cur = self.d[idx];
                let warp = |q: [f64; 2]| -> [f64; 2] {
                    let (qx, qy) = (
                        (q[0] - p[0]) * inv_cell + i as f64,
                        (q[1] - p[1]) * inv_cell + j as f64,
                    );
                    let dq = old(qx, qy);
                    [q[0] + dq[0] - p[0], q[1] + dq[1] - p[1]]
                };
                let new = match brush.tool {
                    LiquifyTool::Reconstruct => {
                        let a = (1.0 - k * (0.1 + 0.4 * rate)) as f32;
                        [cur[0] * a, cur[1] * a]
                    }
                    LiquifyTool::Mirror => {
                        let Some(n) = mirror_normal else { continue };
                        let s = dx * n[0] + dy * n[1];
                        if s <= 0.0 {
                            continue;
                        }
                        // The near side shows the far side reflected across
                        // the stroke; the far side stays put, so overlapping
                        // dabs keep reflecting the same pixels.
                        let m = warp([p[0] - 2.0 * s * n[0], p[1] - 2.0 * s * n[1]]);
                        let t = k.min(1.0);
                        let lerp = |a: f32, b: f64| (f64::from(a) + (b - f64::from(a)) * t) as f32;
                        [lerp(cur[0], m[0]), lerp(cur[1], m[1])]
                    }
                    tool => {
                        let q = source_point(tool, brush, p, c, delta, k, rate, r);
                        let v = warp(q);
                        [v[0] as f32, v[1] as f32]
                    }
                };
                writes.push((idx, new, self.freeze[idx]));
            }
        }
        for (idx, d, f) in writes {
            self.d[idx] = d;
            self.freeze[idx] = f;
        }
        let pad = r.ceil() as i32 + 2;
        clip(
            PsdRect {
                left: c[0].floor() as i32 - pad,
                top: c[1].floor() as i32 - pad,
                right: c[0].ceil() as i32 + pad,
                bottom: c[1].ceil() as i32 + pad,
            },
            self.width,
            self.height,
        )
    }

    /// Catmull-Rom displacement at grid coordinates.
    fn sample_cubic(&self, gx: f64, gy: f64) -> [f64; 2] {
        let (ix, iy) = (gx.floor() as isize, gy.floor() as isize);
        let wx = catmull_rom(gx - gx.floor());
        let wy = catmull_rom(gy - gy.floor());
        let mut acc = [0.0f64; 2];
        for (jj, wyj) in wy.iter().enumerate() {
            let y = (iy - 1 + jj as isize).clamp(0, self.rows as isize - 1) as usize;
            let row = &self.d[y * self.cols..(y + 1) * self.cols];
            for (ii, wxi) in wx.iter().enumerate() {
                let x = (ix - 1 + ii as isize).clamp(0, self.cols as isize - 1) as usize;
                let k = wxi * wyj;
                acc[0] += f64::from(row[x][0]) * k;
                acc[1] += f64::from(row[x][1]) * k;
            }
        }
        acc
    }
}

/// Where a dab of a moving tool reads the current output for node `p`.
#[allow(clippy::too_many_arguments)]
fn source_point(
    tool: LiquifyTool,
    brush: &LiquifyBrush,
    p: [f64; 2],
    c: [f64; 2],
    delta: [f64; 2],
    k: f64,
    rate: f64,
    r: f64,
) -> [f64; 2] {
    let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
    let sign = if brush.reverse { -1.0 } else { 1.0 };
    match tool {
        LiquifyTool::ForwardWarp => [p[0] - k * delta[0], p[1] - k * delta[1]],
        // Left of the stroke direction with y down: (dx, dy) → (dy, -dx).
        LiquifyTool::PushLeft => [p[0] - sign * k * delta[1], p[1] + sign * k * delta[0]],
        LiquifyTool::Twirl => {
            // Content turns clockwise by `th`, so read the point turned back.
            let th = sign * k * rate * 0.12;
            let (sn, cs) = (-th).sin_cos();
            [c[0] + dx * cs - dy * sn, c[1] + dx * sn + dy * cs]
        }
        LiquifyTool::Pucker | LiquifyTool::Bloat => {
            let toward = (tool == LiquifyTool::Pucker) != brush.reverse;
            let a = if toward {
                1.0 + k * rate * 0.06
            } else {
                1.0 - k * rate * 0.06
            };
            [c[0] + dx * a, c[1] + dy * a]
        }
        LiquifyTool::Turbulence => {
            // Higher jitter scrambles at a finer scale.
            let jitter = (brush.jitter / 100.0).clamp(0.0, 1.0);
            let scale = (r * (1.0 - 0.85 * jitter)).max(2.0);
            let n = turbulence_noise(p[0] / scale, p[1] / scale);
            let amp = k * rate * 0.06 * scale;
            [p[0] + amp * n[0], p[1] + amp * n[1]]
        }
        _ => p,
    }
}

/// The unit normal pointing at the side Mirror overwrites: left of the stroke
/// (right when reversed). `None` for a still brush.
fn mirror_normal(brush: &LiquifyBrush, delta: [f64; 2]) -> Option<[f64; 2]> {
    let len = delta[0].hypot(delta[1]);
    (brush.tool == LiquifyTool::Mirror && len > 0.0).then(|| {
        let sign = if brush.reverse { -1.0 } else { 1.0 };
        [sign * delta[1] / len, -sign * delta[0] / len]
    })
}

/// A smooth vector value noise in `[-1, 1]²`, fixed seed.
fn turbulence_noise(x: f64, y: f64) -> [f64; 2] {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (ix as i64, iy as i64);
    std::array::from_fn(|axis| {
        let at = |i: i64, j: i64| lattice(i, j, axis as u64);
        let top = at(ix, iy) + (at(ix + 1, iy) - at(ix, iy)) * sx;
        let bottom = at(ix, iy + 1) + (at(ix + 1, iy + 1) - at(ix, iy + 1)) * sx;
        top + (bottom - top) * sy
    })
}

fn lattice(i: i64, j: i64, axis: u64) -> f64 {
    let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ axis.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// The nodes a dab reads, copied before it writes, plus a margin wide enough
/// for its farthest read.
struct Window {
    i0: usize,
    j0: usize,
    w: usize,
    h: usize,
    nodes: Vec<[f32; 2]>,
}

impl Window {
    /// The nodes `i0..=i1` × `j0..=j1` of `d` plus the margin a dab of radius
    /// `r` and motion `delta` can read beyond them — Mirror reads up to a
    /// diameter away.
    fn capture(
        d: &[[f32; 2]],
        cols: usize,
        (i0, j0, i1, j1): (usize, usize, usize, usize),
        r: f64,
        delta: [f64; 2],
        cell: f64,
    ) -> Self {
        let rows = d.len() / cols;
        let m = ((delta[0].abs().max(delta[1].abs()) + 2.0 * r) / cell).ceil() as usize + 2;
        let (si0, sj0) = (i0.saturating_sub(m), j0.saturating_sub(m));
        let (si1, sj1) = ((i1 + m).min(cols - 1), (j1 + m).min(rows - 1));
        let (w, h) = (si1 - si0 + 1, sj1 - sj0 + 1);
        let mut nodes = Vec::with_capacity(w * h);
        for j in sj0..=sj1 {
            nodes.extend_from_slice(&d[j * cols + si0..=j * cols + si1]);
        }
        Self {
            i0: si0,
            j0: sj0,
            w,
            h,
            nodes,
        }
    }

    fn sample(&self, live: &[[f32; 2]], cols: usize, rows: usize, gx: f64, gy: f64) -> [f64; 2] {
        let (lx, ly) = (gx - self.i0 as f64, gy - self.j0 as f64);
        if lx >= 0.0 && ly >= 0.0 && lx <= (self.w - 1) as f64 && ly <= (self.h - 1) as f64 {
            bilinear(&self.nodes, self.w, self.h, lx, ly)
        } else {
            // Outside the window the dab writes nothing.
            bilinear(live, cols, rows, gx, gy)
        }
    }
}

fn bilinear(d: &[[f32; 2]], w: usize, h: usize, gx: f64, gy: f64) -> [f64; 2] {
    if !(gx > -1.0 && gy > -1.0 && gx < w as f64 && gy < h as f64) {
        return [0.0, 0.0];
    }
    let gx = gx.clamp(0.0, (w - 1) as f64);
    let gy = gy.clamp(0.0, (h - 1) as f64);
    let (i, j) = (
        (gx as usize).min(w.saturating_sub(2)),
        (gy as usize).min(h.saturating_sub(2)),
    );
    let (fx, fy) = (gx - i as f64, gy - j as f64);
    let (i1, j1) = ((i + 1).min(w - 1), (j + 1).min(h - 1));
    let at = |x: usize, y: usize| d[y * w + x];
    let (a, b, c, e) = (at(i, j), at(i1, j), at(i, j1), at(i1, j1));
    let l = |k: usize| {
        (f64::from(a[k]) * (1.0 - fx) + f64::from(b[k]) * fx) * (1.0 - fy)
            + (f64::from(c[k]) * (1.0 - fx) + f64::from(e[k]) * fx) * fy
    };
    [l(0), l(1)]
}

fn catmull_rom(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

fn clip(r: PsdRect, width: u32, height: u32) -> Option<PsdRect> {
    let out = PsdRect {
        left: r.left.max(0),
        top: r.top.max(0),
        right: r.right.min(width as i32),
        bottom: r.bottom.min(height as i32),
    };
    (out.left < out.right && out.top < out.bottom).then_some(out)
}

fn union(a: Option<PsdRect>, b: Option<PsdRect>) -> Option<PsdRect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(PsdRect {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        }),
        (a, None) => a,
        (None, b) => b,
    }
}
