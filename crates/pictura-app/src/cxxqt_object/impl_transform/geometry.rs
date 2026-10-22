use super::super::document_session::TransformSession;
use pictura_core::PsdRect;

/// Screen-pixel tolerance for a scale-handle hit.
const HANDLE_TOLERANCE: f64 = 6.0;
/// Screen-pixel band outside a corner that starts a rotation.
const ROTATE_BAND: f64 = 20.0;
/// Handle indices above the eight scale handles.
pub(super) const ROTATE_HANDLE: i32 = 8;
pub(super) const MOVE_HANDLE: i32 = 9;
/// The reference point's drag handle.
pub(super) const PIVOT_HANDLE: i32 = 10;

/// Centre of `rect` as a document-space point.
pub(super) fn rect_center(rect: PsdRect) -> (f64, f64) {
    (
        (rect.left as f64 + rect.right as f64) / 2.0,
        (rect.top as f64 + rect.bottom as f64) / 2.0,
    )
}

/// Source-rect corners in order top-left, top-right, bottom-right, bottom-left.
pub(super) fn source_corners(rect: PsdRect) -> [(f64, f64); 4] {
    [
        (rect.left as f64, rect.top as f64),
        (rect.right as f64, rect.top as f64),
        (rect.right as f64, rect.bottom as f64),
        (rect.left as f64, rect.bottom as f64),
    ]
}

/// The session map's parameters: `M(p) = P + A·(p − P) + (dx, dy)` with
/// `A = R(θ)·S·K` and `K = [[1, tan(skew_x)], [tan(skew_y), 1]]` (H/V shear in
/// the local frame). Copyable, so a commit can snapshot it before mutating.
#[derive(Clone, Copy)]
pub(super) struct MapParams {
    pub(super) scale_x: f64,
    pub(super) scale_y: f64,
    pub(super) angle: f64,
    pub(super) dx: f64,
    pub(super) dy: f64,
    pub(super) pivot: (f64, f64),
    pub(super) skew_x: f64,
    pub(super) skew_y: f64,
}

impl MapParams {
    /// The live parameters of `session`.
    pub(super) fn of(session: &TransformSession) -> Self {
        Self {
            scale_x: session.scale_x,
            scale_y: session.scale_y,
            angle: session.angle,
            dx: session.dx,
            dy: session.dy,
            pivot: session.pivot,
            skew_x: session.skew_x,
            skew_y: session.skew_y,
        }
    }

    /// The 2×2 linear part `A = R(θ)·S·K`.
    pub(super) fn linear(&self) -> [[f64; 2]; 2] {
        let (sin, cos) = self.angle.sin_cos();
        let (th, tv) = (self.skew_x.tan(), self.skew_y.tan());
        let sk = [
            [self.scale_x, self.scale_x * th],
            [self.scale_y * tv, self.scale_y],
        ];
        [
            [
                cos * sk[0][0] - sin * sk[1][0],
                cos * sk[0][1] - sin * sk[1][1],
            ],
            [
                sin * sk[0][0] + cos * sk[1][0],
                sin * sk[0][1] + cos * sk[1][1],
            ],
        ]
    }

    /// The map applied to a document-space point.
    pub(super) fn map_point(&self, p: (f64, f64)) -> (f64, f64) {
        let a = self.linear();
        let ux = p.0 - self.pivot.0;
        let uy = p.1 - self.pivot.1;
        (
            self.pivot.0 + a[0][0] * ux + a[0][1] * uy + self.dx,
            self.pivot.1 + a[1][0] * ux + a[1][1] * uy + self.dy,
        )
    }

    /// The corners of `rect` under the map.
    pub(super) fn map_quad(&self, rect: PsdRect) -> [(f64, f64); 4] {
        let mut out = source_corners(rect);
        for corner in &mut out {
            *corner = self.map_point(*corner);
        }
        out
    }

    /// Whether the map is the identity.
    pub(super) fn is_identity(&self) -> bool {
        self.scale_x == 1.0
            && self.scale_y == 1.0
            && self.angle == 0.0
            && self.skew_x == 0.0
            && self.skew_y == 0.0
            && self.dx == 0.0
            && self.dy == 0.0
    }
}

/// The live parameters of `session` (`MapParams::of`).
pub(super) fn linear(session: &TransformSession) -> [[f64; 2]; 2] {
    MapParams::of(session).linear()
}

/// The source-rect corners under the session map.
pub(super) fn map_quad(session: &TransformSession) -> [(f64, f64); 4] {
    MapParams::of(session).map_quad(session.orig_rect)
}

/// Apply the session's live transform to `doc` at `path` with `interpolation`:
/// the projective op for a quad-driven session ([`quad_driven`]), the affine
/// quad when a shear is set, else the similarity op re-centred on `orig_rect`
/// (so an integer move keeps its unmodeled raw channels).
pub(super) fn apply_session_transform(
    doc: &mut pictura_core::Document,
    path: &str,
    projective: bool,
    quad: [(f64, f64); 4],
    params: MapParams,
    orig_rect: PsdRect,
    interpolation: pictura_render::Interpolation,
) -> bool {
    if projective {
        return pictura_render::transform_layer_quad_with(doc, path, quad, interpolation);
    }
    if params.skew_x != 0.0 || params.skew_y != 0.0 {
        return pictura_render::transform_layer_quad_with(
            doc,
            path,
            params.map_quad(orig_rect),
            interpolation,
        );
    }
    let (cx, cy) = rect_center(orig_rect);
    let a = params.linear();
    let (px, py) = params.pivot;
    let t = (
        params.dx + (px - cx) - (a[0][0] * (px - cx) + a[0][1] * (py - cy)),
        params.dy + (py - cy) - (a[1][0] * (px - cx) + a[1][1] * (py - cy)),
    );
    pictura_render::transform_layer_with(
        doc,
        path,
        pictura_render::LayerTransform {
            scale_x: params.scale_x,
            scale_y: params.scale_y,
            angle_radians: params.angle,
            dx: t.0,
            dy: t.1,
        },
        interpolation,
    )
}

/// The map with the parameters captured at press (the anchor of a scale drag).
fn start_map_point(session: &TransformSession, p: (f64, f64)) -> (f64, f64) {
    let [sx, sy, angle, dx, dy] = session.start;
    let (sin, cos) = angle.sin_cos();
    let (th, tv) = (session.skew_x.tan(), session.skew_y.tan());
    let ux = p.0 - session.pivot.0;
    let uy = p.1 - session.pivot.1;
    let kx = ux + th * uy;
    let ky = tv * ux + uy;
    (
        session.pivot.0 + cos * sx * kx - sin * sy * ky + dx,
        session.pivot.1 + sin * sx * kx + cos * sy * ky + dy,
    )
}

/// Set the reference point, compensating the translation so the rendered quad
/// does not move: `t' = t + (P − A·P) − (P' − A·P')`.
pub(super) fn set_pivot(session: &mut TransformSession, x: f64, y: f64) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    let a = linear(session);
    let (px, py) = session.pivot;
    let old = (
        px - (a[0][0] * px + a[0][1] * py),
        py - (a[1][0] * px + a[1][1] * py),
    );
    let new = (
        x - (a[0][0] * x + a[0][1] * y),
        y - (a[1][0] * x + a[1][1] * y),
    );
    session.dx += old.0 - new.0;
    session.dy += old.1 - new.1;
    session.pivot = (x, y);
}

/// The nine anchors of `quad` in row-major order (corners, edge midpoints,
/// centre).
pub(super) fn quad_anchors(quad: &[(f64, f64); 4]) -> [(f64, f64); 9] {
    let (tl, tr, br, bl) = (quad[0], quad[1], quad[2], quad[3]);
    [
        tl,
        ((tl.0 + tr.0) / 2.0, (tl.1 + tr.1) / 2.0),
        tr,
        ((tl.0 + bl.0) / 2.0, (tl.1 + bl.1) / 2.0),
        (
            (tl.0 + tr.0 + br.0 + bl.0) / 4.0,
            (tl.1 + tr.1 + br.1 + bl.1) / 4.0,
        ),
        ((tr.0 + br.0) / 2.0, (tr.1 + br.1) / 2.0),
        bl,
        ((bl.0 + br.0) / 2.0, (bl.1 + br.1) / 2.0),
        br,
    ]
}

/// Whether `(x, y)` hits the reference point within the handle tolerance.
pub(super) fn pivot_hit(session: &TransformSession, x: f64, y: f64, zoom: f64) -> bool {
    if !session.show_pivot || !x.is_finite() || !y.is_finite() {
        return false;
    }
    let zoom = if zoom.is_finite() && zoom > 1e-9 {
        zoom
    } else {
        1.0
    };
    let tol = HANDLE_TOLERANCE / zoom;
    dist2((x, y), session.pivot) <= tol * tol
}

/// Move the reference point to `(x, y)`, snapping to the live quad's nine
/// anchors within the handle tolerance.
pub(super) fn pivot_drag(session: &mut TransformSession, x: f64, y: f64, zoom: f64) {
    let zoom = if zoom.is_finite() && zoom > 1e-9 {
        zoom
    } else {
        1.0
    };
    let tol = HANDLE_TOLERANCE / zoom;
    let quad = session_quad(session);
    let mut target = (x, y);
    let mut best = tol * tol;
    for anchor in quad_anchors(&quad) {
        let d = dist2((x, y), anchor);
        if d <= best {
            best = d;
            target = anchor;
        }
    }
    set_pivot(session, target.0, target.1);
}

/// `p' = c + R(θ)·(S·(p − c)) + (dx, dy)` for one point.
#[cfg(test)]
fn forward_point(
    rect: PsdRect,
    sx: f64,
    sy: f64,
    angle: f64,
    dx: f64,
    dy: f64,
    p: (f64, f64),
) -> (f64, f64) {
    let (c_x, c_y) = rect_center(rect);
    let (sin, cos) = angle.sin_cos();
    let ux = (p.0 - c_x) * sx;
    let uy = (p.1 - c_y) * sy;
    (
        c_x + cos * ux - sin * uy + dx,
        c_y + sin * ux + cos * uy + dy,
    )
}

/// The transformed quad corners in [`source_corners`] order.
#[cfg(test)]
pub(super) fn transform_quad_points(
    rect: PsdRect,
    sx: f64,
    sy: f64,
    angle: f64,
    dx: f64,
    dy: f64,
) -> [(f64, f64); 4] {
    let mut out = source_corners(rect);
    for corner in &mut out {
        *corner = forward_point(rect, sx, sy, angle, dx, dy, *corner);
    }
    out
}

/// The session's live target quad: the stored projective quad, or in `Free`
/// mode the similarity applied to the perspective base or the source rect.
pub(super) fn session_quad(session: &TransformSession) -> [(f64, f64); 4] {
    if let Some(quad) = session.quad {
        return quad;
    }
    match session.base_quad {
        Some(base) => {
            let params = MapParams::of(session);
            base.map(|p| params.map_point(p))
        }
        None => map_quad(session),
    }
}

/// Whether the session commits and previews through the projective quad op: a
/// projective mode, or `Free` with a perspective base.
pub(super) fn quad_driven(session: &TransformSession) -> bool {
    session.mode.is_projective() || session.base_quad.is_some()
}

/// Whether a candidate quad is too degenerate to keep: a non-finite corner,
/// under 1 px² of area, or three consecutive corners on one line.
pub(super) fn quad_is_degenerate(quad: &[(f64, f64); 4]) -> bool {
    if quad.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return true;
    }
    let mut area2 = 0.0;
    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        area2 += a.0 * b.1 - b.0 * a.1;
    }
    if area2.abs() / 2.0 < 1.0 {
        return true;
    }
    for i in 0..4 {
        let (a, b, c) = (quad[i], quad[(i + 1) % 4], quad[(i + 2) % 4]);
        let cross = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        if cross.abs() < 1e-6 {
            return true;
        }
    }
    false
}

/// Store `quad` on the session unless it is degenerate; returns whether it was
/// accepted.
pub(super) fn store_quad(session: &mut TransformSession, quad: [(f64, f64); 4]) -> bool {
    if quad_is_degenerate(&quad) {
        return false;
    }
    session.quad = Some(quad);
    true
}

/// Solve the homography sending `src[i]` to `dst[i]` and return its nine
/// coefficients in the order `QTransform(m11,m12,m13,m21,m22,m23,m31,m32,m33)`
/// expects, so the C++ preview maps source document points onto the quad.
pub(super) fn projective_coefficients(
    src: [(f64, f64); 4],
    dst: [(f64, f64); 4],
) -> Option<[f64; 9]> {
    for (x, y) in src.iter().chain(dst.iter()) {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
    }
    let mut a = [[0.0f64; 8]; 8];
    let mut b = [0.0f64; 8];
    for i in 0..4 {
        let (x, y) = src[i];
        let (u, v) = dst[i];
        a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u];
        b[2 * i] = u;
        a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v];
        b[2 * i + 1] = v;
    }
    let s = gaussian_solve(a, b)?;
    let h = [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7], 1.0];
    // QTransform maps x'=(m11·x+m21·y+m31)/w and y'=(m12·x+m22·y+m32)/w with
    // w=m13·x+m23·y+m33, hence the transposed placement of the homography.
    Some([h[0], h[3], h[6], h[1], h[4], h[7], h[2], h[5], h[8]])
}

/// Solve the 8×8 linear system by Gaussian elimination with partial pivoting.
// ponytail: a local copy of the engine's solver (`pictura-render` is frozen for
// this change). Expose the engine helper if a second caller ever appears.
fn gaussian_solve(mut a: [[f64; 8]; 8], mut b: [f64; 8]) -> Option<[f64; 8]> {
    const PIVOT_EPSILON: f64 = 1e-12;
    for col in 0..8 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for (offset, row) in a[(col + 1)..].iter().enumerate() {
            let v = row[col].abs();
            if v > best {
                best = v;
                pivot = col + 1 + offset;
            }
        }
        if !best.is_finite() || best < PIVOT_EPSILON {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let prow = a[col];
        let p = prow[col];
        for row in (col + 1)..8 {
            let factor = a[row][col] / p;
            if factor == 0.0 {
                continue;
            }
            for k in col..8 {
                a[row][k] -= factor * prow[k];
            }
            b[row] -= factor * b[col];
        }
    }
    let mut x = [0.0f64; 8];
    for i in (0..8).rev() {
        let row = a[i];
        let mut s = b[i];
        for k in (i + 1)..8 {
            s -= row[k] * x[k];
        }
        x[i] = s / row[i];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Source coordinate of handle `h` (4..=7 are edge midpoints).
pub(super) fn handle_source_point(rect: PsdRect, h: i32) -> (f64, f64) {
    let (left, top) = (rect.left as f64, rect.top as f64);
    let (right, bottom) = (rect.right as f64, rect.bottom as f64);
    let (mx, my) = ((left + right) / 2.0, (top + bottom) / 2.0);
    match h {
        0 => (left, top),
        1 => (right, top),
        2 => (right, bottom),
        3 => (left, bottom),
        4 => (mx, top),
        5 => (right, my),
        6 => (mx, bottom),
        _ => (left, my),
    }
}

/// The anchor handle opposite `h` (the pivot a scale keeps fixed).
pub(super) fn opposite_handle(h: i32) -> i32 {
    match h {
        0 => 2,
        1 => 3,
        2 => 0,
        3 => 1,
        4 => 6,
        5 => 7,
        6 => 4,
        _ => 5,
    }
}

/// The eight handle positions of the current quad.
fn handle_points(quad: &[(f64, f64); 4]) -> [(f64, f64); 8] {
    [
        quad[0],
        quad[1],
        quad[2],
        quad[3],
        ((quad[0].0 + quad[1].0) / 2.0, (quad[0].1 + quad[1].1) / 2.0),
        ((quad[1].0 + quad[2].0) / 2.0, (quad[1].1 + quad[2].1) / 2.0),
        ((quad[2].0 + quad[3].0) / 2.0, (quad[2].1 + quad[3].1) / 2.0),
        ((quad[3].0 + quad[0].0) / 2.0, (quad[3].1 + quad[0].1) / 2.0),
    ]
}

fn dist2(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)
}

/// Translate the session by the pointer delta since press.
pub(super) fn gesture_translate(session: &mut TransformSession, x: f64, y: f64) {
    session.dx = session.start[3] + (x - session.press_x);
    session.dy = session.start[4] + (y - session.press_y);
}

/// Rotate about the reference point's current image position; Shift snaps to
/// 15° steps. The translation is unchanged, so that point stays fixed.
pub(super) fn gesture_rotate(session: &mut TransformSession, x: f64, y: f64, shift: bool) {
    let pivot = (
        session.pivot.0 + session.start[3],
        session.pivot.1 + session.start[4],
    );
    let a0 = (session.press_y - pivot.1).atan2(session.press_x - pivot.0);
    let a1 = (y - pivot.1).atan2(x - pivot.0);
    let mut angle = session.start[2] + (a1 - a0);
    if shift {
        let step = std::f64::consts::PI / 12.0;
        angle = (angle / step).round() * step;
    }
    session.angle = angle;
    session.dx = session.start[3];
    session.dy = session.start[4];
}

/// Scale about the opposite handle so it stays fixed; Shift locks the aspect
/// ratio and the transformed rect is clamped to at least 1 px per axis. The
/// current H/V shear is kept.
pub(super) fn gesture_scale(session: &mut TransformSession, x: f64, y: f64, shift: bool) -> bool {
    let rect = session.orig_rect;
    let [start_sx, start_sy, start_angle, ..] = session.start;
    let h = session.handle;
    let drag_src = handle_source_point(rect, h);
    let anchor_src = handle_source_point(rect, opposite_handle(h));
    let anchor_doc = start_map_point(session, anchor_src);
    let (th, tv) = (session.skew_x.tan(), session.skew_y.tan());
    let ux = drag_src.0 - anchor_src.0;
    let uy = drag_src.1 - anchor_src.1;
    // The source vector through the fixed shear, then the inverse rotation.
    let kx = ux + th * uy;
    let ky = tv * ux + uy;
    let (sin, cos) = start_angle.sin_cos();
    let vx = x - anchor_doc.0;
    let vy = y - anchor_doc.1;
    let wx = cos * vx + sin * vy;
    let wy = -sin * vx + cos * vy;
    let mut new_sx = if kx.abs() > 1e-12 { wx / kx } else { start_sx };
    let mut new_sy = if ky.abs() > 1e-12 { wy / ky } else { start_sy };
    // Shift locks the aspect ratio on a corner only; an edge handle always
    // scales one axis.
    if shift && h <= 3 {
        let uniform = if new_sx.abs() > new_sy.abs() {
            new_sx
        } else {
            new_sy
        };
        new_sx = uniform;
        new_sy = uniform;
    }
    let min_sx = 1.0 / rect.width() as f64;
    let min_sy = 1.0 / rect.height() as f64;
    if new_sx.abs() < min_sx {
        new_sx = if new_sx < 0.0 { -min_sx } else { min_sx };
    }
    if new_sy.abs() < min_sy {
        new_sy = if new_sy < 0.0 { -min_sy } else { min_sy };
    }
    if !new_sx.is_finite() || !new_sy.is_finite() {
        return false;
    }
    session.scale_x = new_sx;
    session.scale_y = new_sy;
    session.angle = start_angle;
    // Keep the anchor fixed: `t = anchor_doc − P − A·(anchor_src − P)`.
    let a = linear(session);
    let ax = anchor_src.0 - session.pivot.0;
    let ay = anchor_src.1 - session.pivot.1;
    session.dx = anchor_doc.0 - session.pivot.0 - (a[0][0] * ax + a[0][1] * ay);
    session.dy = anchor_doc.1 - session.pivot.1 - (a[1][0] * ax + a[1][1] * ay);
    true
}

/// Convex point-in-quad test using consistent cross-product signs.
fn point_in_quad(p: (f64, f64), quad: &[(f64, f64); 4]) -> bool {
    let mut sign = 0i32;
    for i in 0..4 {
        let a = quad[i];
        let b = quad[(i + 1) % 4];
        let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        if cross.abs() < 1e-9 {
            continue;
        }
        let s = if cross > 0.0 { 1 } else { -1 };
        if sign == 0 {
            sign = s;
        } else if sign != s {
            return false;
        }
    }
    true
}

/// Which part of the session `(x, y)` hits: 0..=7 handle, 8 rotate, 9 move, -1
/// nothing. Test convenience over [`hit_test_quad`] in `Free` mode.
#[cfg(test)]
pub(super) fn hit_test(
    rect: PsdRect,
    sx: f64,
    sy: f64,
    angle: f64,
    dx: f64,
    dy: f64,
    x: f64,
    y: f64,
    zoom: f64,
) -> i32 {
    let quad = transform_quad_points(rect, sx, sy, angle, dx, dy);
    hit_test_quad(&quad, false, x, y, zoom)
}

/// Hit-test the explicit `quad`. In a projective mode only the eight handles are
/// active: a press inside the quad away from a handle returns -1 (no move or
/// rotate). `Free` keeps the move/rotate affordances.
pub(super) fn hit_test_quad(
    quad: &[(f64, f64); 4],
    projective: bool,
    x: f64,
    y: f64,
    zoom: f64,
) -> i32 {
    if !x.is_finite() || !y.is_finite() {
        return -1;
    }
    let zoom = if zoom.is_finite() && zoom > 1e-9 {
        zoom
    } else {
        1.0
    };
    let p = (x, y);
    let tol = HANDLE_TOLERANCE / zoom;
    for (i, h) in handle_points(quad).iter().enumerate() {
        if dist2(p, *h) <= tol * tol {
            return i as i32;
        }
    }
    if projective {
        return -1;
    }
    if point_in_quad(p, quad) {
        return MOVE_HANDLE;
    }
    // The rotate band follows the whole perimeter, so a press just outside any
    // side rotates, not only near a corner.
    let band = ROTATE_BAND / zoom;
    for i in 0..4 {
        if point_segment_dist2(p, quad[i], quad[(i + 1) % 4]) <= band * band {
            return ROTATE_HANDLE;
        }
    }
    -1
}

/// Squared distance from `p` to the segment `a`–`b`.
fn point_segment_dist2(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let (wx, wy) = (p.0 - a.0, p.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 > 0.0 {
        ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (wx - t * vx, wy - t * vy);
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::super::super::document_session::TransformMode;
    use super::super::convex::quad_is_convex;
    use super::super::distort::gesture_distort;
    use super::*;

    fn rect4() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        }
    }

    fn rect100() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 100,
            right: 100,
        }
    }

    fn session() -> TransformSession {
        TransformSession {
            path: "0".to_string(),
            orig_rect: rect4(),
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            dx: 0.0,
            dy: 0.0,
            handle: -1,
            press_x: 0.0,
            press_y: 0.0,
            start: [1.0, 1.0, 0.0, 0.0, 0.0],
            dragging: true,
            mode: TransformMode::Free,
            type_layer: false,
            quad: None,
            base_quad: None,
            free_perspective: false,
            perspective_side: None,
            start_quad: source_corners(rect4()),
            pivot: (2.0, 2.0),
            pivot_origin: (2.0, 2.0),
            start_pivot: (2.0, 2.0),
            show_pivot: true,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: pictura_render::Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            lifted: None,
            untrimmed: None,
            history_label: "Free Transform".to_string(),
        }
    }

    fn projective() -> TransformSession {
        let mut s = session();
        s.mode = TransformMode::Distort;
        s.quad = Some(source_corners(rect4()));
        s
    }

    #[test]
    fn quad_anchors_are_row_major() {
        let a = quad_anchors(&source_corners(rect4()));
        assert_eq!(a[0], (0.0, 0.0));
        assert_eq!(a[1], (2.0, 0.0));
        assert_eq!(a[2], (4.0, 0.0));
        assert_eq!(a[3], (0.0, 2.0));
        assert_eq!(a[4], (2.0, 2.0));
        assert_eq!(a[5], (4.0, 2.0));
        assert_eq!(a[6], (0.0, 4.0));
        assert_eq!(a[8], (4.0, 4.0));
    }

    #[test]
    fn pivot_drag_snaps_to_an_anchor() {
        let mut s = session();
        set_pivot(&mut s, 10.0, 10.0);
        pivot_drag(&mut s, 0.5, 0.5, 1.0);
        assert_eq!(s.pivot, (0.0, 0.0));
    }

    #[test]
    fn pivot_drag_away_keeps_the_pointer() {
        let mut s = session();
        pivot_drag(&mut s, 8.0, 9.0, 1.0);
        assert_eq!(s.pivot, (8.0, 9.0));
    }

    #[test]
    fn moving_the_pivot_keeps_the_quad_fixed() {
        let mut s = session();
        s.scale_x = 2.0;
        s.angle = 0.3;
        s.dx = 3.0;
        let before = session_quad(&s);
        set_pivot(&mut s, 0.0, 0.0);
        let after = session_quad(&s);
        for (b, a) in before.iter().zip(after.iter()) {
            assert!((b.0 - a.0).abs() < 1e-9 && (b.1 - a.1).abs() < 1e-9);
        }
    }

    #[test]
    fn rotation_keeps_the_pivot_image_point_fixed() {
        let mut s = session();
        s.dx = 3.0;
        s.dy = -1.0;
        s.angle = 0.4;
        let before = MapParams::of(&s).map_point(s.pivot);
        s.angle = 1.1;
        let after = MapParams::of(&s).map_point(s.pivot);
        assert!((before.0 - after.0).abs() < 1e-12 && (before.1 - after.1).abs() < 1e-12);
    }

    #[test]
    fn h_shear_moves_the_left_edge() {
        let mut s = session();
        s.skew_x = (30f64).to_radians();
        let q = session_quad(&s);
        let expected = (30f64).to_radians().tan() * 2.0;
        assert!((q[3].0 - expected).abs() < 1e-9, "left edge x {}", q[3].0);
        assert!((q[3].1 - 4.0).abs() < 1e-9);
    }

    #[test]
    fn pivot_hit_respects_the_show_flag() {
        let mut s = session();
        assert!(pivot_hit(&s, 2.0, 2.0, 1.0));
        assert!(!pivot_hit(&s, 10.0, 10.0, 1.0));
        s.show_pivot = false;
        assert!(!pivot_hit(&s, 2.0, 2.0, 1.0));
    }

    #[test]
    fn hit_test_classifies_handles_move_and_rotate() {
        let r = rect100();
        assert_eq!(hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0), 0);
        assert_eq!(hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, 100.0, 100.0, 1.0), 2);
        assert_eq!(hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, 50.0, 0.0, 1.0), 4);
        assert_eq!(
            hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, 50.0, 50.0, 1.0),
            MOVE_HANDLE
        );
        assert_eq!(
            hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, -10.0, -10.0, 1.0),
            ROTATE_HANDLE
        );
        assert_eq!(hit_test(r, 1.0, 1.0, 0.0, 0.0, 0.0, 300.0, 300.0, 1.0), -1);
    }

    #[test]
    fn rotate_band_follows_every_side() {
        let q = source_corners(rect100());
        // Just outside each edge midpoint, within the 20 px band.
        assert_eq!(hit_test_quad(&q, false, 50.0, -10.0, 1.0), ROTATE_HANDLE);
        assert_eq!(hit_test_quad(&q, false, 110.0, 50.0, 1.0), ROTATE_HANDLE);
        assert_eq!(hit_test_quad(&q, false, 50.0, 110.0, 1.0), ROTATE_HANDLE);
        assert_eq!(hit_test_quad(&q, false, -10.0, 50.0, 1.0), ROTATE_HANDLE);
        // Beyond the band nothing is hit.
        assert_eq!(hit_test_quad(&q, false, 50.0, -30.0, 1.0), -1);
        // The band is measured in screen pixels: 10 px out is inside at 1x,
        // outside the 5 px band at 4x.
        assert_eq!(hit_test_quad(&q, false, 50.0, -10.0, 4.0), -1);
        // A projective mode keeps only the handles active outside.
        assert_eq!(hit_test_quad(&q, true, 50.0, -10.0, 1.0), -1);
    }

    #[test]
    fn identity_quad_is_the_source_rect() {
        let q = transform_quad_points(rect4(), 1.0, 1.0, 0.0, 0.0, 0.0);
        assert_eq!(q, [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    }

    #[test]
    fn opposite_handles_are_paired() {
        assert_eq!(opposite_handle(0), 2);
        assert_eq!(opposite_handle(1), 3);
        assert_eq!(opposite_handle(4), 6);
        assert_eq!(opposite_handle(5), 7);
        for h in 0..8 {
            assert_eq!(opposite_handle(opposite_handle(h)), h);
        }
    }

    #[test]
    fn corner_scale_keeps_the_opposite_corner_fixed() {
        let mut s = session();
        s.handle = 0;
        s.press_x = 0.0;
        s.press_y = 0.0;
        assert!(gesture_scale(&mut s, -4.0, -4.0, false));
        assert!((s.scale_x - 2.0).abs() < 1e-9);
        assert!((s.scale_y - 2.0).abs() < 1e-9);
        let anchor = forward_point(
            s.orig_rect,
            s.scale_x,
            s.scale_y,
            s.angle,
            s.dx,
            s.dy,
            (4.0, 4.0),
        );
        assert!((anchor.0 - 4.0).abs() < 1e-9 && (anchor.1 - 4.0).abs() < 1e-9);
        let dragged = forward_point(
            s.orig_rect,
            s.scale_x,
            s.scale_y,
            s.angle,
            s.dx,
            s.dy,
            (0.0, 0.0),
        );
        assert!((dragged.0 + 4.0).abs() < 1e-9 && (dragged.1 + 4.0).abs() < 1e-9);
    }

    #[test]
    fn shift_on_a_corner_locks_the_two_scales_equal() {
        let mut s = session();
        s.handle = 0;
        assert!(gesture_scale(&mut s, -4.0, -8.0, true));
        assert!((s.scale_x - s.scale_y).abs() < 1e-12);
    }

    #[test]
    fn shift_on_an_edge_handle_leaves_the_other_axis() {
        let mut s = session();
        s.handle = 4;
        assert!(gesture_scale(&mut s, 2.0, -4.0, true));
        assert!((s.scale_x - 1.0).abs() < 1e-9, "x stays 1.0");
        assert!((s.scale_y - 2.0).abs() < 1e-9, "y scales to 2.0");
    }

    #[test]
    fn scale_clamps_to_at_least_one_pixel_per_axis() {
        let mut s = session();
        s.handle = 0;
        assert!(gesture_scale(&mut s, 4.0 - 0.001, 4.0 - 0.001, false));
        assert!((s.scale_x.abs() - 0.25).abs() < 1e-9);
        assert!((s.scale_y.abs() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn rotation_tracks_the_pointer_and_snaps() {
        let mut s = session();
        s.handle = ROTATE_HANDLE;
        s.press_x = 4.0;
        s.press_y = 2.0;
        gesture_rotate(&mut s, 2.0, 4.0, false);
        assert!((s.angle - std::f64::consts::FRAC_PI_2).abs() < 1e-9);

        let mut snapped = session();
        snapped.handle = ROTATE_HANDLE;
        snapped.press_x = 4.0;
        snapped.press_y = 2.0;
        let target: f64 = 0.2;
        gesture_rotate(
            &mut snapped,
            2.0 + target.cos() * 2.0,
            2.0 + target.sin() * 2.0,
            true,
        );
        assert!((snapped.angle - std::f64::consts::PI / 12.0).abs() < 1e-9);
    }

    #[test]
    fn translation_accumulates_from_press() {
        let mut s = session();
        s.handle = MOVE_HANDLE;
        s.press_x = 1.0;
        s.press_y = 1.0;
        gesture_translate(&mut s, 3.0, 5.0);
        assert!((s.dx - 2.0).abs() < 1e-12 && (s.dy - 4.0).abs() < 1e-12);
    }

    #[test]
    fn degenerate_gesture_is_refused() {
        let mut s = projective();
        let before = s.quad.unwrap();
        assert!(!gesture_distort(&mut s, 0, f64::NAN, 0.0));
        assert_eq!(s.quad.unwrap(), before);
        assert!(gesture_distort(&mut s, 0, 4.0, 0.0));
        assert!(quad_is_convex(&s.quad.unwrap(), 1.0, 0.0));
    }

    #[test]
    fn projective_hit_test_ignores_the_inside() {
        let q = source_corners(rect100());
        assert_eq!(hit_test_quad(&q, true, 50.0, 50.0, 1.0), -1);
        assert_eq!(hit_test_quad(&q, true, 0.0, 0.0, 1.0), 0);
        assert_eq!(hit_test_quad(&q, true, 50.0, 0.0, 1.0), 4);
        assert_eq!(hit_test_quad(&q, false, 50.0, 50.0, 1.0), MOVE_HANDLE);
    }

    #[test]
    fn projective_coefficients_map_the_source_corners() {
        let src = source_corners(rect4());
        let dst = [(1.0, 1.0), (5.0, 0.0), (4.0, 5.0), (0.0, 3.0)];
        let c = projective_coefficients(src, dst).unwrap();
        // QTransform's map: x'=(m11·x+m21·y+m31)/w, y'=(m12·x+m22·y+m32)/w.
        let map = |x: f64, y: f64| {
            let w = c[2] * x + c[5] * y + c[8];
            (
                (c[0] * x + c[3] * y + c[6]) / w,
                (c[1] * x + c[4] * y + c[7]) / w,
            )
        };
        for (s, d) in src.iter().zip(dst.iter()) {
            let p = map(s.0, s.1);
            assert!((p.0 - d.0).abs() < 1e-9 && (p.1 - d.1).abs() < 1e-9);
        }
    }
}
