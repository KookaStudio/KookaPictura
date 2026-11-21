//! Opt-in runtime metrics (`PICTURA_METRICS`): named phase totals and one
//! record per committed edit, printed as `pictura metrics:` lines and served to
//! the control server as JSON.
//!
//! Every edit runs on the GUI thread, so the state is thread-local. With
//! metrics off nothing is printed or retained; the composite never depends on
//! this module.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// How many committed edits the snapshot keeps.
const EDIT_RING: usize = 32;
const FRAME_RING: usize = 1024;
const CPU_FRAME_BUDGET_MS: f64 = 16.6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Phase {
    pub(crate) count: u32,
    pub(crate) total: Duration,
    pub(crate) max: Duration,
}

impl Phase {
    fn add(&mut self, dt: Duration) {
        self.count += 1;
        self.total += dt;
        self.max = self.max.max(dt);
    }
}

/// One committed edit.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EditRecord {
    pub(crate) label: String,
    pub(crate) composited: bool,
    pub(crate) wall: Duration,
    pub(crate) phases: Vec<(String, Phase)>,
    pub(crate) canvas_bytes: u64,
    pub(crate) gpu_ms: Option<f64>,
}

#[derive(Default)]
pub(super) struct FrameRecord {
    pub(super) canvas_id: u64,
    pub(super) input_count: u32,
    pub(super) input_ns: u64,
    pub(super) tile_gather_ns: u64,
    pub(super) paint_ns: u64,
    pub(super) interval_ns: Option<u64>,
    pub(super) gpu_ns: Option<u64>,
}

impl FrameRecord {
    fn write_json(&self, out: &mut String) {
        let ns_to_ms = |ns: u64| ns as f64 / 1_000_000.0;
        let input_ms = ns_to_ms(self.input_ns);
        let paint_ms = ns_to_ms(self.paint_ns);
        let _ = write!(
            out,
            "{{\"canvas_id\":{},\"boundary\":\"canvas_paint\",\"input_count\":{},\"input_ms\":{input_ms:.3},\"tile_gather_ms\":{:.3},\"qt_draw_ms\":{:.3},\"paint_ms\":{paint_ms:.3},\"cpu_budget_slack_ms\":{:.3},\"gpu_present_ms\":null",
            self.canvas_id,
            self.input_count,
            ns_to_ms(self.tile_gather_ns),
            ns_to_ms(self.paint_ns.saturating_sub(self.tile_gather_ns)),
            CPU_FRAME_BUDGET_MS - input_ms - paint_ms,
        );
        if let Some(ns) = self.interval_ns {
            let _ = write!(out, ",\"interval_ms\":{:.3}", ns_to_ms(ns));
        }
        if let Some(ns) = self.gpu_ns {
            let _ = write!(out, ",\"gpu_composite_ms\":{:.3}", ns_to_ms(ns));
        }
        out.push('}');
    }
}

#[derive(Default)]
pub(crate) struct Metrics {
    on: bool,
    phases: Vec<(String, Phase)>,
    edits: VecDeque<EditRecord>,
    open: Option<(EditRecord, Instant)>,
    frames: Option<VecDeque<FrameRecord>>,
    frame_records_evicted: u64,
    frame_generation: u64,
}

fn add_phase(phases: &mut Vec<(String, Phase)>, label: &str, dt: Duration) {
    match phases.iter_mut().find(|(l, _)| l == label) {
        Some((_, p)) => p.add(dt),
        None => {
            let mut p = Phase::default();
            p.add(dt);
            phases.push((label.to_string(), p));
        }
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

impl Metrics {
    pub(crate) fn new(on: bool) -> Self {
        Self {
            on,
            frame_generation: u64::from(on),
            ..Self::default()
        }
    }

    fn record_frame(&mut self, record: impl FnOnce() -> FrameRecord) {
        if !self.on {
            return;
        }
        let frames = self
            .frames
            .get_or_insert_with(|| VecDeque::with_capacity(FRAME_RING));
        if frames.len() == FRAME_RING {
            frames.pop_front();
            self.frame_records_evicted += 1;
        }
        frames.push_back(record());
    }

    pub(crate) fn record_phase(&mut self, label: &str, dt: Duration) {
        if !self.on {
            return;
        }
        add_phase(&mut self.phases, label, dt);
        if let Some((record, _)) = self.open.as_mut() {
            add_phase(&mut record.phases, label, dt);
        }
    }

    /// Open a record for `label`, unless one is already open: the QObject
    /// opens it before the mutation runs, and the transaction's own call
    /// (the only one in a direct `apply_edit`) then keeps it.
    pub(crate) fn begin_edit(&mut self, label: &str) {
        if !self.on || self.open.is_some() {
            return;
        }
        // Drop what earlier work left so the record sees only its own edit.
        let _ = pictura_render::metrics::take_largest_canvas();
        let _ = pictura_render::metrics::take_device_ms();
        let record = EditRecord {
            label: label.to_string(),
            ..EditRecord::default()
        };
        self.open = Some((record, Instant::now()));
    }

    pub(crate) fn mark_composited(&mut self) {
        if let Some((record, _)) = self.open.as_mut() {
            record.composited = true;
        }
    }

    /// Drop the open record: the mutation changed nothing.
    pub(crate) fn cancel_edit(&mut self) {
        self.open = None;
    }

    /// Close the open record, keep it, and return it for printing.
    pub(crate) fn end_edit(&mut self) -> Option<EditRecord> {
        let (mut record, start) = self.open.take()?;
        record.wall = start.elapsed();
        record.canvas_bytes = pictura_render::metrics::take_largest_canvas();
        record.gpu_ms = pictura_render::metrics::take_device_ms();
        if self.edits.len() == EDIT_RING {
            self.edits.pop_front();
        }
        self.edits.push_back(record.clone());
        Some(record)
    }

    #[cfg(test)]
    pub(crate) fn phases(&self) -> &[(String, Phase)] {
        &self.phases
    }

    #[cfg(test)]
    pub(crate) fn edits(&self) -> impl Iterator<Item = &EditRecord> {
        self.edits.iter()
    }

    /// The snapshot as a JSON object, or `None` when metrics are off.
    pub(crate) fn snapshot_json(&mut self, reset: bool) -> Option<String> {
        if !self.on {
            return None;
        }
        let mut out = String::from("{\"enabled\":true,\"phases\":{");
        for (i, (label, p)) in self.phases.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{}:{{\"count\":{},\"total_ms\":{:.3},\"max_ms\":{:.3}}}",
                json_string(label),
                p.count,
                ms(p.total),
                ms(p.max)
            );
        }
        let counters = pictura_render::metrics::counters();
        let _ = write!(
            out,
            "}},\"counters\":{{\"resident_hits\":{},\"resident_misses\":{},\"resident_entries\":{},\"resident_bytes\":{},\"source_hits\":{},\"source_misses\":{},\"source_bytes\":{},\"effect_decodes\":{},\"declines\":{{",
            counters.resident_hits,
            counters.resident_misses,
            counters.resident_entries,
            counters.resident_bytes,
            counters.source_hits,
            counters.source_misses,
            counters.source_bytes,
            counters.effect_decodes
        );
        for (i, (reason, n)) in counters.declines.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}:{n}", json_string(reason));
        }
        out.push_str("},\"layer_declines\":{");
        for (i, (reason, n)) in counters.layer_declines.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}:{n}", json_string(reason));
        }
        out.push_str("}},\"edits\":[");
        for (i, e) in self.edits.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"label\":{},\"composited\":{},\"wall_ms\":{:.3},\"canvas_bytes\":{}",
                json_string(&e.label),
                e.composited,
                ms(e.wall),
                e.canvas_bytes
            );
            if let Some(gpu) = e.gpu_ms {
                let _ = write!(out, ",\"gpu_ms\":{gpu:.3}");
            }
            out.push_str(",\"phases\":{");
            for (j, (label, p)) in e.phases.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{}:{:.3}", json_string(label), ms(p.total));
            }
            out.push_str("}}");
        }
        out.push_str("],\"frames\":[");
        if let Some(frames) = &self.frames {
            for (index, frame) in frames.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                frame.write_json(&mut out);
            }
        }
        let _ = write!(
            out,
            "],\"frame_records_evicted\":{}}}",
            self.frame_records_evicted
        );
        if reset {
            self.phases.clear();
            self.edits.clear();
            if let Some(frames) = &mut self.frames {
                frames.clear();
            }
            self.frame_records_evicted = 0;
            self.frame_generation += 1;
        }
        Some(out)
    }
}

pub(super) fn frame_generation() -> u64 {
    METRICS.with(|m| m.borrow().frame_generation)
}

pub(super) fn new_frame_sample(
    blend_ns: Option<u64>,
) -> Option<std::sync::Arc<pictura_render::FrameTileSample>> {
    let nanos = blend_ns?;
    let generation = frame_generation();
    (generation != 0).then(|| pictura_render::FrameTileSample::new(generation, nanos))
}

pub(super) fn record_frame(generation: u64, record: FrameRecord) {
    METRICS.with(|cell| {
        let mut metrics = cell.borrow_mut();
        if generation == metrics.frame_generation {
            metrics.record_frame(|| record);
        }
    });
}

/// `"…"` with the JSON escapes a label can need.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The stderr line for one edit record. Fields are `key=value`; the label is
/// quoted because history labels contain spaces.
pub(crate) fn edit_line(record: &EditRecord) -> String {
    let mut line = format!(
        "pictura metrics: edit label={} composited={} wall_ms={:.3} canvas_bytes={}",
        json_string(&record.label),
        record.composited,
        ms(record.wall),
        record.canvas_bytes
    );
    if let Some(gpu) = record.gpu_ms {
        let _ = write!(line, " gpu_ms={gpu:.3}");
    }
    for (label, p) in &record.phases {
        let _ = write!(line, " {label}_ms={:.3}", ms(p.total));
    }
    line
}

thread_local! {
    static METRICS: RefCell<Metrics> =
        RefCell::new(Metrics::new(pictura_render::metrics::enabled()));
}

/// Whether metrics are on for this thread.
pub(crate) fn enabled() -> bool {
    METRICS.with(|m| m.borrow().on)
}

/// Accumulate `dt` under `label` (and into the open edit record).
pub(crate) fn record_phase(label: &str, dt: Duration) {
    METRICS.with(|m| m.borrow_mut().record_phase(label, dt));
}

/// Record a phase timed on the Qt side and print its line.
pub(crate) fn record_bridge_phase(label: &str, micros: i64) {
    if !enabled() {
        return;
    }
    let dt = Duration::from_micros(micros.max(0) as u64);
    record_phase(label, dt);
    eprintln!(
        "pictura metrics: phase label={} ms={:.3}",
        json_string(label),
        ms(dt)
    );
}

pub(crate) fn begin_edit(label: &str) {
    METRICS.with(|m| m.borrow_mut().begin_edit(label));
}

pub(crate) fn cancel_edit() {
    METRICS.with(|m| m.borrow_mut().cancel_edit());
}

pub(crate) fn mark_composited() {
    METRICS.with(|m| m.borrow_mut().mark_composited());
}

/// Close the open edit record and print it.
pub(crate) fn end_edit() {
    if let Some(record) = METRICS.with(|m| m.borrow_mut().end_edit()) {
        eprintln!("{}", edit_line(&record));
    }
}

/// The control-server snapshot, or `None` when metrics are off.
pub(crate) fn snapshot_json(reset: bool) -> Option<String> {
    METRICS.with(|m| m.borrow_mut().snapshot_json(reset))
}

/// Read this thread's metrics state.
#[cfg(test)]
pub(crate) fn with<T>(f: impl FnOnce(&Metrics) -> T) -> T {
    METRICS.with(|m| f(&m.borrow()))
}

/// One phase's accumulated nanoseconds on this thread, when recorded.
#[cfg(test)]
pub(crate) fn phase_ns(label: &str) -> Option<u64> {
    with(|metrics| {
        metrics
            .phases
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, phase)| phase.total.as_nanos() as u64)
    })
}

/// Turn metrics on (or off) for the calling test thread only.
#[cfg(test)]
pub(crate) fn set_for_this_thread(on: bool) {
    METRICS.with(|m| *m.borrow_mut() = Metrics::new(on));
}

impl super::qobject::PictureView {
    pub fn record_metric_phase(&self, label: &cxx_qt_lib::QString, micros: i64) {
        record_bridge_phase(&label.to_string(), micros);
    }

    pub fn metrics_snapshot(&self, reset: bool) -> cxx_qt_lib::QString {
        cxx_qt_lib::QString::from(snapshot_json(reset).unwrap_or_default().as_str())
    }
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod tests;
