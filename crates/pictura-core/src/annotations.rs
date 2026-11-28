//! Annotations: the marks the Eyedropper tool group leaves on a document to
//! *read* it rather than change it. None of them touch pixels.
//!
//! * **Color samplers** — persistent probes whose values the Info panel reads
//!   out; document state, capped at [`MAX_COLOR_SAMPLERS`].
//! * **Notes** — text pinned to a point; document state.
//! * **The ruler** — one measuring line. CS6 does not save it with the
//!   document, so it is view state and lives outside [`Annotations`].
//!
//! ponytail: samplers and notes are not yet written to or read from the PSD
//! (color samplers resource 1073, the `Anno` annotations block), which stay
//! preserved verbatim.
//!
//! Ported from photorust's `core/src/annotation.rs`
//! (<https://github.com/perfecto25/photorust>).

/// How many color samplers CS6 allows on one document
/// (`docs/03-tools/eyedropper-color-sampler-ruler.md`); later versions raised it.
pub const MAX_COLOR_SAMPLERS: usize = 4;

/// The two point-marker kinds, shared with the shell as plain integers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum MarkerKind {
    ColorSampler = 0,
    Note = 1,
}

impl MarkerKind {
    pub fn from_i32(v: i32) -> Option<MarkerKind> {
        match v {
            0 => Some(MarkerKind::ColorSampler),
            1 => Some(MarkerKind::Note),
            _ => None,
        }
    }
}

/// A point marker in document pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Marker {
    pub x: i32,
    pub y: i32,
    /// Note text; empty for a color sampler.
    pub text: String,
}

/// CS6's default Count mark colour (a light blue).
pub const DEFAULT_COUNT_COLOR: [u8; 3] = [0x2f, 0x9f, 0xd0];
/// CS6 Marker Size range (the options-bar scrubby slider).
pub const COUNT_MARKER_SIZE_MIN: u8 = 1;
pub const COUNT_MARKER_SIZE_MAX: u8 = 10;
/// CS6 Label Size range.
pub const COUNT_LABEL_SIZE_MIN: u8 = 8;
pub const COUNT_LABEL_SIZE_MAX: u8 = 72;

/// One Count tool group: a named, independently numbered set of marks with its
/// own visibility, colour, marker size, and label size (Photoshop Extended).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountGroup {
    pub name: String,
    pub visible: bool,
    pub color: [u8; 3],
    pub marker_size: u8,
    pub label_size: u8,
    marks: Vec<Marker>,
}

impl Default for CountGroup {
    fn default() -> Self {
        Self {
            name: "Count Group 1".to_string(),
            visible: true,
            color: DEFAULT_COUNT_COLOR,
            marker_size: 2,
            label_size: 12,
            marks: Vec::new(),
        }
    }
}

impl CountGroup {
    /// A new empty group named `name`, with the CS6 defaults.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn marks(&self) -> &[Marker] {
        &self.marks
    }

    pub fn count(&self) -> usize {
        self.marks.len()
    }

    pub fn marker(&self, index: usize) -> Option<&Marker> {
        self.marks.get(index)
    }

    /// Append a mark, returning its index (its 1-based number minus one).
    pub fn add_mark(&mut self, x: i32, y: i32) -> usize {
        self.marks.push(Marker {
            x,
            y,
            text: String::new(),
        });
        self.marks.len() - 1
    }

    /// Move mark `index`; false for an out-of-range index.
    pub fn move_mark(&mut self, index: usize, x: i32, y: i32) -> bool {
        match self.marks.get_mut(index) {
            Some(mark) => {
                mark.x = x;
                mark.y = y;
                true
            }
            None => false,
        }
    }

    /// Delete mark `index`, shifting later numbers down; false when out of range.
    pub fn remove_mark(&mut self, index: usize) -> bool {
        if index >= self.marks.len() {
            return false;
        }
        self.marks.remove(index);
        true
    }

    /// Delete every mark; false when there were none.
    pub fn clear(&mut self) -> bool {
        let had = !self.marks.is_empty();
        self.marks.clear();
        had
    }

    /// The mark within `radius` of `(x, y)`, nearest first.
    pub fn mark_at(&self, x: f64, y: f64, radius: f64) -> Option<usize> {
        nearest(&self.marks, x, y, radius)
    }
}

/// The document's color samplers, notes, and Count groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotations {
    samplers: Vec<Marker>,
    notes: Vec<Marker>,
    count_groups: Vec<CountGroup>,
    active_count_group: usize,
}

impl Default for Annotations {
    fn default() -> Self {
        Self {
            samplers: Vec::new(),
            notes: Vec::new(),
            // CS6 shows a "Count Group 1" before any mark is placed.
            count_groups: vec![CountGroup::default()],
            active_count_group: 0,
        }
    }
}

impl Annotations {
    fn list(&self, kind: MarkerKind) -> &Vec<Marker> {
        match kind {
            MarkerKind::ColorSampler => &self.samplers,
            MarkerKind::Note => &self.notes,
        }
    }

    fn list_mut(&mut self, kind: MarkerKind) -> &mut Vec<Marker> {
        match kind {
            MarkerKind::ColorSampler => &mut self.samplers,
            MarkerKind::Note => &mut self.notes,
        }
    }

    pub fn markers(&self, kind: MarkerKind) -> &[Marker] {
        self.list(kind)
    }

    pub fn marker(&self, kind: MarkerKind, index: usize) -> Option<&Marker> {
        self.list(kind).get(index)
    }

    /// Place a marker, returning its index; `None` (no change) once the
    /// samplers are full — CS6 refuses a fifth rather than evicting one.
    pub fn add(&mut self, kind: MarkerKind, x: i32, y: i32) -> Option<usize> {
        if kind == MarkerKind::ColorSampler && self.samplers.len() >= MAX_COLOR_SAMPLERS {
            return None;
        }
        let list = self.list_mut(kind);
        list.push(Marker {
            x,
            y,
            text: String::new(),
        });
        Some(list.len() - 1)
    }

    /// Move marker `index`; false for an out-of-range index.
    pub fn move_marker(&mut self, kind: MarkerKind, index: usize, x: i32, y: i32) -> bool {
        match self.list_mut(kind).get_mut(index) {
            Some(marker) => {
                marker.x = x;
                marker.y = y;
                true
            }
            None => false,
        }
    }

    /// Replace note `index`'s text; false for an out-of-range index.
    pub fn set_note_text(&mut self, index: usize, text: impl Into<String>) -> bool {
        match self.notes.get_mut(index) {
            Some(note) => {
                note.text = text.into();
                true
            }
            None => false,
        }
    }

    /// Delete marker `index`, shifting later ones down; false when out of range.
    pub fn remove(&mut self, kind: MarkerKind, index: usize) -> bool {
        let list = self.list_mut(kind);
        if index >= list.len() {
            return false;
        }
        list.remove(index);
        true
    }

    /// Delete every marker of `kind`; false when there were none.
    pub fn clear(&mut self, kind: MarkerKind) -> bool {
        let list = self.list_mut(kind);
        let had = !list.is_empty();
        list.clear();
        had
    }

    /// The marker of `kind` within `radius` of `(x, y)`, nearest first. The
    /// shell scales `radius` by zoom so the grab area keeps its screen size.
    pub fn marker_at(&self, kind: MarkerKind, x: f64, y: f64, radius: f64) -> Option<usize> {
        nearest(self.list(kind), x, y, radius)
    }

    // --- Count groups (Photoshop Extended) ---------------------------------

    pub fn count_groups(&self) -> &[CountGroup] {
        &self.count_groups
    }

    pub fn count_group(&self, index: usize) -> Option<&CountGroup> {
        self.count_groups.get(index)
    }

    pub fn count_group_mut(&mut self, index: usize) -> Option<&mut CountGroup> {
        self.count_groups.get_mut(index)
    }

    pub fn active_count_group(&self) -> usize {
        self.active_count_group
    }

    /// The active group; always present (there is at least one).
    pub fn active_group_mut(&mut self) -> &mut CountGroup {
        let index = self.active_count_group.min(self.count_groups.len() - 1);
        &mut self.count_groups[index]
    }

    /// Select group `index`; false when out of range.
    pub fn set_active_count_group(&mut self, index: usize) -> bool {
        if index >= self.count_groups.len() {
            return false;
        }
        self.active_count_group = index;
        true
    }

    /// Add a group named `name`, make it active, and return its index.
    pub fn add_count_group(&mut self, name: impl Into<String>) -> usize {
        self.count_groups.push(CountGroup::new(name));
        self.active_count_group = self.count_groups.len() - 1;
        self.active_count_group
    }

    /// Delete group `index`; false when it is the only group or out of range
    /// (CS6 always keeps one group). The active index follows the removal.
    pub fn remove_count_group(&mut self, index: usize) -> bool {
        if index >= self.count_groups.len() || self.count_groups.len() <= 1 {
            return false;
        }
        self.count_groups.remove(index);
        if self.active_count_group >= self.count_groups.len() {
            self.active_count_group = self.count_groups.len() - 1;
        }
        true
    }

    /// The number of marks in the active group (the options bar's Count field).
    pub fn active_count_total(&self) -> usize {
        self.count_groups[self.active_count_group.min(self.count_groups.len() - 1)].count()
    }
}

/// The index of the item within `radius` of `(x, y)`, nearest first.
fn nearest(markers: &[Marker], x: f64, y: f64, radius: f64) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, marker) in markers.iter().enumerate() {
        let d = (f64::from(marker.x) - x).hypot(f64::from(marker.y) - y);
        if d <= radius && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// A Ruler measuring line from `a` to `b`, in document pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ruler {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub protractor: Option<(bool, (f64, f64))>,
}

/// The Ruler readout: CS6's X, Y, W, H, A, D1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measurement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Degrees in `-180..=180`, anticlockwise from east as seen on screen, so a
    /// line running down-right reads negative, as CS6 reports it.
    pub angle: f64,
    pub distance: f64,
}

impl Ruler {
    /// Included angle and length of the optional second arm.
    pub fn protractor_measurement(&self) -> Option<(f64, f64)> {
        let (at_start, end) = self.protractor?;
        let (vertex, other) = if at_start {
            (self.a, self.b)
        } else {
            (self.b, self.a)
        };
        let u = (other.0 - vertex.0, other.1 - vertex.1);
        let v = (end.0 - vertex.0, end.1 - vertex.1);
        let length = v.0.hypot(v.1);
        let angle = (u.0 * v.1 - u.1 * v.0)
            .abs()
            .atan2(u.0 * v.0 + u.1 * v.1)
            .to_degrees();
        Some((angle, length))
    }

    pub fn measure(&self) -> Measurement {
        let dx = self.b.0 - self.a.0;
        let dy = self.b.1 - self.a.1;
        Measurement {
            x: self.a.0,
            y: self.a.1,
            width: dx,
            height: dy,
            // Document y grows downward; the reported angle does not.
            angle: (-dy).atan2(dx).to_degrees(),
            distance: dx.hypot(dy),
        }
    }

    /// Shift-drag: snap `b` to the nearest 45° ray from `a`, keeping the length
    /// of `b`'s projection onto that ray.
    pub fn constrained(a: (f64, f64), b: (f64, f64)) -> Ruler {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let step = std::f64::consts::FRAC_PI_4;
        let angle = (dy.atan2(dx) / step).round() * step;
        let (s, c) = angle.sin_cos();
        let along = dx * c + dy * s;
        Ruler {
            a,
            b: (a.0 + along * c, a.1 + along * s),
            protractor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn samplers_and_notes_are_independent() {
        let mut a = Annotations::default();
        a.add(MarkerKind::ColorSampler, 1, 1);
        a.add(MarkerKind::Note, 2, 2);
        a.add(MarkerKind::Note, 3, 3);
        assert_eq!(a.markers(MarkerKind::ColorSampler).len(), 1);
        assert!(a.clear(MarkerKind::Note));
        assert!(
            !a.clear(MarkerKind::Note),
            "clearing nothing reported a change"
        );
        assert_eq!(a.markers(MarkerKind::ColorSampler).len(), 1);
    }

    #[test]
    fn color_samplers_stop_at_four() {
        let mut a = Annotations::default();
        for i in 0..MAX_COLOR_SAMPLERS {
            assert_eq!(a.add(MarkerKind::ColorSampler, i as i32, 0), Some(i));
        }
        assert_eq!(a.add(MarkerKind::ColorSampler, 9, 9), None);
        assert_eq!(a.markers(MarkerKind::ColorSampler).len(), 4);
        assert!(a.remove(MarkerKind::ColorSampler, 0));
        assert_eq!(a.add(MarkerKind::ColorSampler, 9, 9), Some(3));
        for i in 0..20 {
            assert!(
                a.add(MarkerKind::Note, i, 0).is_some(),
                "notes are unlimited"
            );
        }
    }

    #[test]
    fn hit_testing_picks_the_nearest_within_range() {
        let mut a = Annotations::default();
        a.add(MarkerKind::ColorSampler, 10, 10);
        a.add(MarkerKind::ColorSampler, 14, 10);
        a.add(MarkerKind::Note, 40, 40);
        assert_eq!(
            a.marker_at(MarkerKind::ColorSampler, 11.0, 10.0, 5.0),
            Some(0)
        );
        assert_eq!(
            a.marker_at(MarkerKind::ColorSampler, 13.0, 10.0, 5.0),
            Some(1)
        );
        assert_eq!(a.marker_at(MarkerKind::ColorSampler, 40.0, 40.0, 5.0), None);
        assert_eq!(a.marker_at(MarkerKind::Note, 41.0, 41.0, 5.0), Some(0));
    }

    #[test]
    fn edits_address_markers_by_index() {
        let mut a = Annotations::default();
        for i in 0..3 {
            a.add(MarkerKind::Note, i * 10, 0);
        }
        assert!(a.set_note_text(1, "check this edge"));
        assert!(!a.set_note_text(9, "nope"));
        assert!(a.move_marker(MarkerKind::Note, 1, 30, 40));
        assert!(!a.move_marker(MarkerKind::Note, 9, 0, 0));
        assert!(a.remove(MarkerKind::Note, 0));
        assert!(!a.remove(MarkerKind::Note, 9));
        let moved = a.marker(MarkerKind::Note, 0).unwrap();
        assert_eq!(
            (moved.x, moved.y, moved.text.as_str()),
            (30, 40, "check this edge")
        );
    }

    #[test]
    fn the_ruler_reads_cs6_angles_and_lengths() {
        let flat = Ruler {
            a: (10.0, 20.0),
            b: (60.0, 20.0),
            protractor: None,
        }
        .measure();
        assert!(close(flat.x, 10.0) && close(flat.y, 20.0) && close(flat.width, 50.0));
        assert!(close(flat.angle, 0.0) && close(flat.distance, 50.0));
        let down = Ruler {
            a: (0.0, 0.0),
            b: (3.0, 4.0),
            protractor: None,
        }
        .measure();
        assert!(close(down.distance, 5.0) && close(down.height, 4.0));
        assert!(
            (down.angle + 53.130_102).abs() < 1e-4,
            "angle was {}",
            down.angle
        );
        let up = Ruler {
            a: (0.0, 10.0),
            b: (10.0, 0.0),
            protractor: None,
        }
        .measure();
        assert!(close(up.angle, 45.0));
    }

    #[test]
    fn protractor_reports_included_angle_and_second_length() {
        for vertex_at_start in [false, true] {
            let mut r = Ruler {
                a: (0.0, 0.0),
                b: (20.0, 0.0),
                protractor: None,
            };
            let x = if vertex_at_start { 0.0 } else { 20.0 };
            r.protractor = Some((vertex_at_start, (x, 10.0)));
            let (angle, distance) = r.protractor_measurement().unwrap();
            assert!(close(angle, 90.0));
            assert!(close(distance, 10.0));
            assert!(close(r.measure().distance, 20.0));
        }
    }

    #[test]
    fn shift_snaps_the_ruler_to_45_degrees() {
        let r = Ruler::constrained((0.0, 0.0), (10.0, 1.0));
        assert!(close(r.b.1, 0.0) && close(r.b.0, 10.0));
        let r = Ruler::constrained((0.0, 0.0), (10.0, 9.0));
        assert!(close(r.b.0, r.b.1) && close(r.measure().angle, -45.0));
        let r = Ruler::constrained((5.0, 5.0), (5.5, -20.0));
        assert!(close(r.b.0, 5.0) && close(r.measure().angle, 90.0));
    }

    #[test]
    fn marker_kind_round_trips_through_its_integer() {
        for kind in [MarkerKind::ColorSampler, MarkerKind::Note] {
            assert_eq!(MarkerKind::from_i32(kind as i32), Some(kind));
        }
        assert_eq!(MarkerKind::from_i32(2), None);
    }

    #[test]
    fn count_groups_are_independent_numbered_counters() {
        let mut a = Annotations::default();
        // A default group exists before any mark is placed.
        assert_eq!(a.count_groups().len(), 1);
        assert_eq!(a.active_count_group(), 0);
        let g0 = a.active_group_mut();
        g0.add_mark(3, 3);
        g0.add_mark(8, 4);
        assert_eq!(a.active_count_total(), 2);
        // A second group is independent.
        let second = a.add_count_group("Count Group 2");
        assert_eq!(second, 1);
        assert_eq!(a.active_count_total(), 0);
        a.active_group_mut().add_mark(1, 1);
        assert_eq!(a.count_group(0).unwrap().count(), 2);
        assert_eq!(a.count_group(1).unwrap().count(), 1);
        // Removing a mark shifts later numbers down.
        assert!(a.count_group_mut(0).unwrap().remove_mark(0));
        assert_eq!(a.count_group(0).unwrap().marker(0).unwrap().x, 8);
        // The only group cannot be deleted.
        assert!(a.remove_count_group(0));
        assert_eq!(a.count_groups().len(), 1);
        assert!(!a.remove_count_group(0));
    }

    #[test]
    fn count_group_defaults_match_cs6() {
        let g = CountGroup::default();
        assert_eq!(g.name, "Count Group 1");
        assert!(g.visible);
        assert_eq!(g.marker_size, 2);
        assert_eq!(g.label_size, 12);
    }
}
