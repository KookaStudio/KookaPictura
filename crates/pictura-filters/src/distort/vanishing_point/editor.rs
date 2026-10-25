//! The Vanishing Point dialog's editing state: the planes, the floating
//! selection and the marquee, the active plane, the gesture in progress, and
//! the dialog's own undo and redo. The retouch tools (`tools.rs`) add the
//! working copy of the layer; their pixel edits undo through the same stack.

use super::canvas::{PixelStep, Rect};
use super::float::{Floating, Marquee};
use super::geometry::P2;
use super::planes::{Part, VpPlanes};
use super::tools::Retouch;

/// How a node drag was started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragMode {
    Normal,
    /// Ctrl on an edge node: tear off a related plane.
    TearOff,
    /// Alt on a related plane's far edge node: turn it about its hinge.
    Angle,
}

#[derive(Clone, Copy, Debug)]
enum Drag {
    Corner(usize, usize),
    Edge(usize, usize),
    Slide(usize, P2),
    TearOff(usize),
    Angle(usize),
}

/// The dialog's undo keeps this many steps.
const UNDO_DEPTH: usize = 100;

/// Everything the dialog's undo restores but the layer's pixels.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct State {
    pub planes: VpPlanes,
    pub float: Option<Floating>,
    pub marquee: Option<Marquee>,
}

struct Step {
    state: State,
    pixels: Option<PixelStep>,
}

pub struct VpEditor {
    pub(super) state: State,
    pub(super) active: Option<usize>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    drag: Option<Drag>,
    before: Option<State>,
    pub(super) retouch: Option<Retouch>,
    /// Document pixels the preview must recompose.
    pub(super) dirty: Rect,
}

impl VpEditor {
    pub fn new(planes: VpPlanes) -> Self {
        let active = planes.len().checked_sub(1);
        VpEditor {
            state: State {
                planes,
                float: None,
                marquee: None,
            },
            active,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            before: None,
            retouch: None,
            dirty: Rect::default(),
        }
    }

    pub fn planes(&self) -> &VpPlanes {
        &self.state.planes
    }

    /// Start over on `planes` (Last Filter Settings): no floating selection,
    /// marquee or undo; the working copy of the layer stays.
    pub fn reset_planes(&mut self, planes: VpPlanes) {
        self.active = planes.len().checked_sub(1);
        self.state = State {
            planes,
            float: None,
            marquee: None,
        };
        self.undo.clear();
        self.redo.clear();
        self.touch_all();
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    pub fn set_active(&mut self, i: Option<usize>) {
        self.active = i.filter(|&i| i < self.state.planes.len());
    }

    /// Mark the whole document for recomposing (a plane edit moves a float).
    pub(super) fn touch_all(&mut self) {
        let (w, h) = self.state.planes.size();
        self.dirty = Rect {
            x0: 0,
            y0: 0,
            x1: w as i32,
            y1: h as i32,
        };
    }

    /// Have the preview recompose everything (an option the float shows
    /// changed).
    pub fn invalidate(&mut self) {
        self.touch_all();
    }

    /// Take the pixels the preview must recompose since the last call.
    pub fn take_dirty(&mut self) -> Option<Rect> {
        let r = std::mem::take(&mut self.dirty);
        (!r.is_empty()).then_some(r)
    }

    /// Record an undo step from `before` (and the pixels `pixels` held).
    pub(super) fn record(&mut self, before: State, pixels: Option<PixelStep>) {
        if pixels.is_none() && before == self.state {
            return;
        }
        if self.state.float.is_some() || before.float.is_some() {
            self.touch_all();
        }
        self.undo.push(Step {
            state: before,
            pixels,
        });
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn edit(&mut self, f: impl FnOnce(&mut VpPlanes)) {
        let before = self.state.clone();
        f(&mut self.state.planes);
        self.record(before, None);
    }

    /// Add a root plane on `q` and make it active.
    pub fn create(&mut self, q: [P2; 4]) -> usize {
        let mut index = 0;
        self.edit(|planes| index = planes.add(q));
        self.active = Some(index);
        index
    }

    pub fn hit(&self, p: P2, tol: f64) -> Option<(usize, Part)> {
        self.state.planes.hit(p, tol, self.active)
    }

    /// Make the next plane under `p` after the active one active (CS6's
    /// Ctrl-click through overlapping planes).
    pub fn cycle(&mut self, p: P2) -> Option<usize> {
        let under = self.state.planes.planes_at(p);
        let next = match self.active.and_then(|a| under.iter().position(|&i| i == a)) {
            Some(at) => under.get(at + 1).or(under.first()).copied(),
            None => under.last().copied(),
        };
        if next.is_some() {
            self.active = next;
        }
        next
    }

    /// Start dragging `part` of `plane` from `p`. False when the gesture does
    /// not apply (a tear-off from an invalid plane, an Alt-drag that is not
    /// on a turnable plane's far edge).
    pub fn begin(&mut self, plane: usize, part: Part, mode: DragMode, p: P2) -> bool {
        if plane >= self.state.planes.len() {
            return false;
        }
        self.before = Some(self.state.clone());
        let planes = &mut self.state.planes;
        let drag = match (part, mode) {
            (Part::Edge(k), DragMode::TearOff) => planes.tear_off(plane, k).map(Drag::TearOff),
            (Part::Edge(2), DragMode::Angle) if planes.angle_editable(plane) => {
                Some(Drag::Angle(plane))
            }
            (_, DragMode::Angle) => None,
            (Part::Corner(k), _) => Some(Drag::Corner(plane, k)),
            (Part::Edge(k), _) => Some(Drag::Edge(plane, k)),
            (Part::Inside, _) => Some(Drag::Slide(plane, p)),
        };
        self.active = Some(match drag {
            Some(Drag::TearOff(child)) => child,
            _ => plane,
        });
        self.drag = drag;
        if drag.is_none() {
            self.before = None;
        }
        drag.is_some()
    }

    pub fn drag_to(&mut self, p: P2) {
        let planes = &mut self.state.planes;
        match self.drag {
            Some(Drag::Corner(i, k)) => planes.move_corner(i, k, p),
            Some(Drag::Edge(i, k)) => planes.move_edge(i, k, p),
            Some(Drag::Slide(i, from)) => {
                planes.slide(i, from, p);
                self.drag = Some(Drag::Slide(i, p));
            }
            Some(Drag::TearOff(c)) => planes.extend(c, p),
            Some(Drag::Angle(c)) => planes.aim(c, p),
            None => return,
        }
        if self.state.float.is_some() {
            self.touch_all();
        }
    }

    pub fn end(&mut self) {
        self.drag = None;
        if let Some(before) = self.before.take() {
            self.record(before, None);
        }
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Set the active family's Grid Size.
    pub fn set_grid(&mut self, size: f64) {
        if let Some(i) = self.active {
            self.edit(|planes| planes.set_grid(i, size));
        }
    }

    /// Set the active plane's angle; false when it may not turn.
    pub fn set_angle(&mut self, degrees: f64) -> bool {
        let Some(i) = self.active.filter(|&i| self.state.planes.angle_editable(i)) else {
            return false;
        };
        self.edit(|planes| {
            planes.set_angle(i, degrees);
        });
        true
    }

    /// Delete the active plane and the planes hinged on it. A floating
    /// selection and the marquee go with them, since plane indices shift.
    pub fn delete_active(&mut self) -> bool {
        let Some(i) = self.active.take() else {
            return false;
        };
        let before = self.state.clone();
        self.state.planes.remove(i);
        self.state.float = None;
        self.state.marquee = None;
        self.record(before, None);
        true
    }

    pub fn undo(&mut self) -> bool {
        self.step(true)
    }

    pub fn redo(&mut self) -> bool {
        self.step(false)
    }

    fn step(&mut self, back: bool) -> bool {
        let (from, to) = if back {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some(mut step) = from.pop() else {
            return false;
        };
        std::mem::swap(&mut self.state, &mut step.state);
        if let (Some(pixels), Some(retouch)) = (step.pixels.as_mut(), self.retouch.as_mut()) {
            pixels.swap(&mut retouch.canvas);
        }
        to.push(step);
        self.active = self.active.filter(|&i| i < self.state.planes.len());
        self.touch_all();
        true
    }

    pub fn into_planes(self) -> VpPlanes {
        self.state.planes
    }
}
