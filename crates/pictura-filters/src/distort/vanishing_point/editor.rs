//! The Vanishing Point dialog's editing state over its planes: the active
//! plane, the gesture in progress, and the dialog's own undo and redo.

use super::geometry::P2;
use super::planes::{Part, VpPlanes};

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

#[derive(Clone, Debug)]
pub struct VpEditor {
    planes: VpPlanes,
    active: Option<usize>,
    undo: Vec<VpPlanes>,
    redo: Vec<VpPlanes>,
    drag: Option<Drag>,
    before: Option<VpPlanes>,
}

impl VpEditor {
    pub fn new(planes: VpPlanes) -> Self {
        let active = planes.len().checked_sub(1);
        VpEditor {
            planes,
            active,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            before: None,
        }
    }

    pub fn planes(&self) -> &VpPlanes {
        &self.planes
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    pub fn set_active(&mut self, i: Option<usize>) {
        self.active = i.filter(|&i| i < self.planes.len());
    }

    fn record(&mut self, before: VpPlanes) {
        if before != self.planes {
            self.undo.push(before);
            if self.undo.len() > UNDO_DEPTH {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    fn edit(&mut self, f: impl FnOnce(&mut VpPlanes)) {
        let before = self.planes.clone();
        f(&mut self.planes);
        self.record(before);
    }

    /// Add a root plane on `q` and make it active.
    pub fn create(&mut self, q: [P2; 4]) -> usize {
        let mut index = 0;
        self.edit(|planes| index = planes.add(q));
        self.active = Some(index);
        index
    }

    pub fn hit(&self, p: P2, tol: f64) -> Option<(usize, Part)> {
        self.planes.hit(p, tol, self.active)
    }

    /// Make the next plane under `p` after the active one active (CS6's
    /// Ctrl-click through overlapping planes).
    pub fn cycle(&mut self, p: P2) -> Option<usize> {
        let under = self.planes.planes_at(p);
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
        if plane >= self.planes.len() {
            return false;
        }
        self.before = Some(self.planes.clone());
        let drag = match (part, mode) {
            (Part::Edge(k), DragMode::TearOff) => self.planes.tear_off(plane, k).map(Drag::TearOff),
            (Part::Edge(2), DragMode::Angle) if self.planes.angle_editable(plane) => {
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
        match self.drag {
            Some(Drag::Corner(i, k)) => self.planes.move_corner(i, k, p),
            Some(Drag::Edge(i, k)) => self.planes.move_edge(i, k, p),
            Some(Drag::Slide(i, from)) => {
                self.planes.slide(i, from, p);
                self.drag = Some(Drag::Slide(i, p));
            }
            Some(Drag::TearOff(c)) => self.planes.extend(c, p),
            Some(Drag::Angle(c)) => self.planes.aim(c, p),
            None => {}
        }
    }

    pub fn end(&mut self) {
        self.drag = None;
        if let Some(before) = self.before.take() {
            self.record(before);
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
        let Some(i) = self.active.filter(|&i| self.planes.angle_editable(i)) else {
            return false;
        };
        self.edit(|planes| {
            planes.set_angle(i, degrees);
        });
        true
    }

    /// Delete the active plane and the planes hinged on it.
    pub fn delete_active(&mut self) -> bool {
        let Some(i) = self.active.take() else {
            return false;
        };
        self.edit(|planes| planes.remove(i));
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
        let Some(state) = from.pop() else {
            return false;
        };
        to.push(std::mem::replace(&mut self.planes, state));
        self.active = self.active.filter(|&i| i < self.planes.len());
        true
    }

    pub fn into_planes(self) -> VpPlanes {
        self.planes
    }
}
