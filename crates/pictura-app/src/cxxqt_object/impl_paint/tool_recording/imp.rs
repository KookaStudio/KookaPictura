#![allow(dead_code)]

use super::*;

use super::super::super::edit::Edit;
use super::super::PictureViewRust;
use cxx_qt_lib::QString;
use pictura_paint::spacing::SpacingMode;
use pictura_paint::{PaintMode, Rgba, StrokeConfig, StrokeSample};
use pictura_script::action::{ToolSample, ToolStroke};
use pictura_script::command::CommandParams;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub(crate) const TOOL_STROKE_FORMAT_VERSION: u32 = ToolStroke::VERSION;
pub(crate) const MAX_TOOL_STROKE_SAMPLES: usize = pictura_script::recorder::MAX_TOOL_SAMPLES;
pub(crate) const MAX_TOOL_STROKE_BYTES: usize = pictura_script::recorder::MAX_TOOL_PAYLOAD_BYTES;

/// Which begin path opened the stroke, so the player knows how to replay it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolStrokeKind {
    /// `begin_paint` / `begin_brush`: replays through [`PictureViewRust::begin_paint`].
    Paint,
    /// A tool-specific `paint_tools::begin_*` stroke; the player replays it
    /// through that same begin function with `settings` and `samples`.
    Native,
    /// A mask stroke paints mask coverage, not layer pixels; there is no
    /// replay path, so the capture is refused.
    Mask,
}

/// One completed stroke, ready for the action engine.
#[derive(Debug)]
pub(crate) struct ToolStrokeStep {
    pub version: u32,
    pub tool: String,
    pub kind: ToolStrokeKind,
    pub settings: StrokeConfig,
    pub samples: Vec<StrokeSample>,
}

impl ToolStrokeStep {
    /// The encoded engine payload size, used by the bound check before the step
    /// reaches an action.
    pub(crate) fn payload_bytes(&self) -> usize {
        self.to_engine_stroke().payload_len()
    }

    /// Rebuild a step from the engine payload the player hands back.
    pub(crate) fn from_engine_stroke(stroke: &ToolStroke) -> Result<Self, ToolStrokeError> {
        if stroke.version != TOOL_STROKE_FORMAT_VERSION {
            return Err(ToolStrokeError::UnsupportedVersion {
                found: stroke.version,
            });
        }
        let settings = stroke.settings();
        let default = StrokeConfig::default();
        let unsigned = |key: &str| settings.get(key).and_then(|value| value.as_u64());
        let integer = |key: &str| settings.get(key).and_then(|value| value.as_i64());
        let boolean = |key: &str| settings.get(key).and_then(|value| value.as_bool());
        let byte = |key: &str| unsigned(key).map(|value| value.min(100) as u8);
        let wide = |key: &str| unsigned(key).map(|value| value.min(u16::MAX as u64) as u16);
        let count = |key: &str| unsigned(key).map(|value| value.clamp(1, 16) as u8);
        let kind = match settings.get_str("replay_kind") {
            Some("native") => ToolStrokeKind::Native,
            Some("mask") => ToolStrokeKind::Mask,
            _ => ToolStrokeKind::Paint,
        };
        let spacing = match settings.get_str("spacing_mode") {
            Some("velocity") => SpacingMode::VelocityDriven,
            _ => SpacingMode::Fixed(
                unsigned("spacing").map_or(25, |value| value.min(u16::MAX as u64) as u16),
            ),
        };
        let settings = StrokeConfig {
            color: unsigned("foreground").map_or(default.color, |value| {
                rgba_from_argb(value.min(u32::MAX as u64) as u32)
            }),
            background: unsigned("background").map_or(default.background, |value| {
                rgba_from_argb(value.min(u32::MAX as u64) as u32)
            }),
            diameter: unsigned("diameter")
                .map_or(default.diameter, |value| value.min(u32::MAX as u64) as u32),
            hardness: byte("hardness").unwrap_or(default.hardness),
            roundness: byte("roundness").unwrap_or(default.roundness),
            angle_deg: integer("angle").map_or(default.angle_deg, |value| {
                value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
            }),
            spacing,
            opacity: byte("opacity").unwrap_or(default.opacity),
            flow: byte("flow").unwrap_or(default.flow),
            mode: settings
                .get_str("mode")
                .map_or(default.mode, paint_mode_from_name),
            aliased: boolean("aliased").unwrap_or(default.aliased),
            square: boolean("square").unwrap_or(default.square),
            scatter: wide("scatter").unwrap_or(default.scatter),
            count: count("count").unwrap_or(default.count),
            size_jitter: byte("size_jitter").unwrap_or(default.size_jitter),
            angle_jitter: wide("angle_jitter").unwrap_or(default.angle_jitter),
            roundness_jitter: byte("roundness_jitter").unwrap_or(default.roundness_jitter),
            grain: byte("grain").unwrap_or(default.grain),
            wet_edges: boolean("wet_edges").unwrap_or(default.wet_edges),
            flip_x: boolean("flip_x").unwrap_or(default.flip_x),
            flip_y: boolean("flip_y").unwrap_or(default.flip_y),
            auto_erase: boolean("auto_erase").unwrap_or(default.auto_erase),
        }
        .sanitized();
        Ok(Self {
            version: stroke.version,
            tool: stroke.tool.clone(),
            kind,
            settings,
            samples: stroke
                .samples
                .iter()
                .map(|sample| StrokeSample {
                    x: sample.x as f32,
                    y: sample.y as f32,
                    pressure: sample.pressure as f32,
                })
                .collect(),
        })
    }

    /// The engine payload: the shared stroke settings plus ordered samples.
    /// Tool-specific begin options are not in [`StrokeConfig`], so the native
    /// families carry their tool identity and the player re-reads those options
    /// from the live tool before replay.
    pub(crate) fn to_engine_stroke(&self) -> ToolStroke {
        let mut settings = CommandParams::new();
        settings.set("foreground", argb(self.settings.color));
        settings.set("background", argb(self.settings.background));
        settings.set("diameter", self.settings.diameter);
        settings.set("hardness", self.settings.hardness);
        settings.set("roundness", self.settings.roundness);
        settings.set("angle", self.settings.angle_deg);
        settings.set("opacity", self.settings.opacity);
        settings.set("flow", self.settings.flow);
        settings.set("mode", paint_mode_name(self.settings.mode));
        settings.set("aliased", self.settings.aliased);
        settings.set("auto_erase", self.settings.auto_erase);
        settings.set("square", self.settings.square);
        settings.set("scatter", self.settings.scatter);
        settings.set("count", self.settings.count);
        settings.set("size_jitter", self.settings.size_jitter);
        settings.set("angle_jitter", self.settings.angle_jitter);
        settings.set("roundness_jitter", self.settings.roundness_jitter);
        settings.set("grain", self.settings.grain);
        settings.set("wet_edges", self.settings.wet_edges);
        settings.set("flip_x", self.settings.flip_x);
        settings.set("flip_y", self.settings.flip_y);
        match self.settings.spacing {
            SpacingMode::Fixed(percent) => {
                settings.set("spacing_mode", "fixed");
                settings.set("spacing", percent);
            }
            SpacingMode::VelocityDriven => settings.set("spacing_mode", "velocity"),
        }
        settings.set(
            "replay_kind",
            match self.kind {
                ToolStrokeKind::Paint => "paint",
                ToolStrokeKind::Native => "native",
                ToolStrokeKind::Mask => "mask",
            },
        );
        ToolStroke::new(
            self.tool.clone(),
            settings,
            self.samples
                .iter()
                .map(|sample| ToolSample {
                    x: sample.x as f64,
                    y: sample.y as f64,
                    pressure: sample.pressure as f64,
                    // The dab call carries no clock; playback timing comes from
                    // the action player's speed, as for every other step.
                    time: 0.0,
                })
                .collect(),
        )
    }
}

pub(super) fn argb(color: Rgba) -> u32 {
    (color.a as u32) << 24 | (color.r as u32) << 16 | (color.g as u32) << 8 | color.b as u32
}

pub(super) fn rgba_from_argb(color: u32) -> Rgba {
    Rgba {
        a: (color >> 24) as u8,
        r: (color >> 16) as u8,
        g: (color >> 8) as u8,
        b: color as u8,
    }
}

pub(super) fn paint_mode_name(mode: PaintMode) -> &'static str {
    match mode {
        PaintMode::Normal => "normal",
        PaintMode::Dissolve => "dissolve",
        PaintMode::Behind => "behind",
        PaintMode::Clear => "clear",
        PaintMode::Multiply => "multiply",
    }
}

pub(super) fn paint_mode_from_name(name: &str) -> PaintMode {
    match name {
        "dissolve" => PaintMode::Dissolve,
        "behind" => PaintMode::Behind,
        "clear" => PaintMode::Clear,
        "multiply" => PaintMode::Multiply,
        _ => PaintMode::Normal,
    }
}

/// A named diagnostic; the engine/panel can show it and the action stays as it
/// was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ToolStrokeError {
    /// More input samples arrived than one step may carry.
    SampleLimit { count: usize, limit: usize },
    /// The encoded payload would exceed the bound.
    PayloadLimit { bytes: usize, limit: usize },
    /// The step was written by a newer format.
    UnsupportedVersion { found: u32 },
    /// The serialized payload could not be read.
    InvalidPayload { reason: String },
    /// The player has no begin path for this tool yet.
    ReplayUnsupported { tool: String },
    /// The tool's begin call refused playback.
    ReplayRefused { tool: String },
    /// The replay painted no pixels.
    NothingPainted { tool: String },
    /// The recorder itself refused the step.
    RecorderRejected { reason: String },
}

impl fmt::Display for ToolStrokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SampleLimit { count, limit } => write!(
                f,
                "tool stroke refused: {count} samples exceed the {limit}-sample limit"
            ),
            Self::PayloadLimit { bytes, limit } => write!(
                f,
                "tool stroke refused: {bytes}-byte payload exceeds the {limit}-byte limit"
            ),
            Self::UnsupportedVersion { found } => write!(
                f,
                "tool stroke refused: format version {found} is not supported"
            ),
            Self::InvalidPayload { reason } => {
                write!(f, "tool stroke refused: invalid payload: {reason}")
            }
            Self::ReplayUnsupported { tool } => {
                write!(f, "tool stroke refused: {tool} has no replay path")
            }
            Self::ReplayRefused { tool } => {
                write!(f, "tool stroke refused: {tool} could not begin")
            }
            Self::NothingPainted { tool } => {
                write!(f, "tool stroke refused: {tool} painted no pixels")
            }
            Self::RecorderRejected { reason } => {
                write!(f, "tool stroke refused: {reason}")
            }
        }
    }
}

/// The Actions recorder seam. The app bridge installs its engine adapter; the
/// recorder decides whether Allow Tool Recording is on and appends the step.
pub(crate) trait ToolStrokeRecorder {
    fn tool_recording_active(&self) -> bool;
    fn record_tool_stroke(&mut self, step: ToolStrokeStep) -> Result<(), ToolStrokeError>;
    /// Called when the app refuses a stroke before it reaches the recorder, so
    /// the engine can surface the same named diagnostic.
    fn tool_stroke_refused(&mut self, _error: &ToolStrokeError) {}
}

pub(crate) type SharedToolStrokeRecorder = Rc<RefCell<dyn ToolStrokeRecorder>>;

pub(super) struct Capture {
    step: ToolStrokeStep,
}

#[derive(Default)]
pub(super) struct State {
    recorder: Option<SharedToolStrokeRecorder>,
    capture: Option<Capture>,
    last_error: Option<ToolStrokeError>,
}

thread_local! {
    // Qt paints on the GUI thread; tests get one capture per test thread.
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Install (or clear, with `None`) the engine recorder. Idempotent.
pub(crate) fn install_tool_stroke_recorder(recorder: Option<SharedToolStrokeRecorder>) {
    STATE.with(|state| state.borrow_mut().recorder = recorder);
}

/// Take the last refusal so the panel can surface its named diagnostic.
pub(crate) fn take_tool_stroke_error() -> Option<ToolStrokeError> {
    STATE.with(|state| state.borrow_mut().last_error.take())
}

/// Open a capture after a stroke began. Does nothing while recording is off.
pub(crate) fn begin(tool: &str, settings: &StrokeConfig, kind: ToolStrokeKind) {
    let refused = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.capture = None;
        state.last_error = None;
        let active = match state.recorder.as_ref() {
            Some(recorder) => recorder.borrow().tool_recording_active(),
            None => ffi::active_tool_recording(),
        };
        if !active {
            return false;
        }
        if matches!(kind, ToolStrokeKind::Native | ToolStrokeKind::Mask) {
            return true;
        }
        state.capture = Some(Capture {
            step: ToolStrokeStep {
                version: TOOL_STROKE_FORMAT_VERSION,
                tool: tool.to_string(),
                kind,
                settings: *settings,
                samples: Vec::new(),
            },
        });
        false
    });
    if refused {
        refuse(ToolStrokeError::ReplayUnsupported {
            tool: tool.to_string(),
        });
    }
}

/// Append one input sample. An over-limit stroke is refused as a whole; later
/// samples are ignored until the next begin.
pub(crate) fn sample(x: f64, y: f64, pressure: f64) {
    let overflow = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let capture = state.capture.as_mut()?;
        let count = capture.step.samples.len();
        if count >= MAX_TOOL_STROKE_SAMPLES {
            return Some(count + 1);
        }
        capture.step.samples.push(StrokeSample {
            x: x as f32,
            y: y as f32,
            pressure: pressure as f32,
        });
        None
    });
    if let Some(count) = overflow {
        refuse(ToolStrokeError::SampleLimit {
            count,
            limit: MAX_TOOL_STROKE_SAMPLES,
        });
    }
}

/// Store the diagnostic and let the recorder or bridge surface it.
pub(super) fn report(error: ToolStrokeError) {
    let recorder = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.last_error = Some(error.clone());
        state.recorder.clone()
    });
    if let Some(recorder) = recorder {
        recorder.borrow_mut().tool_stroke_refused(&error);
    } else {
        ffi::tool_stroke_refused(&QString::from(error.to_string()));
    }
}

/// Drop the capture, keep the diagnostic, and let the recorder surface it.
pub(super) fn refuse(error: ToolStrokeError) {
    STATE.with(|cell| cell.borrow_mut().capture = None);
    report(error);
}

/// Close a capture. Only a committed stroke reaches the recorder; a cancelled
/// or empty stroke is dropped without touching the action.
pub(crate) fn finish(committed: bool) {
    let outcome = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(capture) = state.capture.take() else {
            return Ok(None);
        };
        if !committed {
            return Ok(None);
        }
        let step = capture.step;
        let bytes = step.payload_bytes();
        if bytes > MAX_TOOL_STROKE_BYTES {
            return Err(ToolStrokeError::PayloadLimit {
                bytes,
                limit: MAX_TOOL_STROKE_BYTES,
            });
        }
        Ok(Some(step))
    });
    let step = match outcome {
        Ok(Some(step)) => step,
        Ok(None) => return,
        Err(error) => {
            refuse(error);
            return;
        }
    };
    let recorder = STATE.with(|cell| cell.borrow().recorder.clone());
    if let Some(recorder) = recorder {
        let outcome = recorder.borrow_mut().record_tool_stroke(step);
        if let Err(error) = outcome {
            report(error);
        }
    } else if ffi::active_tool_recording() {
        let payload = step.to_engine_stroke().to_json();
        let _ = ffi::record_active_tool_stroke(&QString::from(payload.as_str()));
    }
}

/// Drop a capture without recording it.
pub(crate) fn cancel() {
    STATE.with(|cell| cell.borrow_mut().capture = None);
}

/// Parse the engine payload, rebuild the step, and replay it. A refusal is
/// stored and reported before it is returned.
pub(crate) fn replay_tool_stroke_json(
    view: &mut PictureViewRust,
    payload: &str,
) -> Result<(), ToolStrokeError> {
    cancel();
    let result = ToolStroke::from_json(payload)
        .map_err(|reason| ToolStrokeError::InvalidPayload { reason })
        .and_then(|stroke| ToolStrokeStep::from_engine_stroke(&stroke))
        .and_then(|step| replay_tool_stroke(view, &step));
    if let Err(error) = &result {
        report(error.clone());
    }
    result
}

/// Replay a captured step on the same stroke engine.
///
/// Paint strokes (Brush/Pencil) replay through [`PictureViewRust::begin_paint`]
/// and commit one history state. Tool-specific families capture their identity
/// and shared settings; the C++ player replays those through the family's
/// `begin_*` function, which this Qt-free helper cannot reach.
pub(crate) fn replay_tool_stroke(
    view: &mut PictureViewRust,
    step: &ToolStrokeStep,
) -> Result<(), ToolStrokeError> {
    if step.version != TOOL_STROKE_FORMAT_VERSION {
        return Err(ToolStrokeError::UnsupportedVersion {
            found: step.version,
        });
    }
    if step.kind != ToolStrokeKind::Paint {
        return Err(ToolStrokeError::ReplayUnsupported {
            tool: step.tool.clone(),
        });
    }
    // Playback is not a user stroke: never let it extend a live capture.
    cancel();
    if !view.begin_paint(step.settings, &step.tool) {
        cancel();
        return Err(ToolStrokeError::ReplayRefused {
            tool: step.tool.clone(),
        });
    }
    cancel();
    let Some(mut session) = view.session.live.take_stroke_session() else {
        return Err(ToolStrokeError::ReplayRefused {
            tool: step.tool.clone(),
        });
    };
    let Some(mut stroke) = session.stroke.take() else {
        view.cancel_stroke_session(session);
        return Err(ToolStrokeError::ReplayRefused {
            tool: step.tool.clone(),
        });
    };
    let finished = {
        let Some(doc) = view.session.doc.as_mut() else {
            return Err(ToolStrokeError::ReplayRefused {
                tool: step.tool.clone(),
            });
        };
        for sample in &step.samples {
            stroke.sample(doc, *sample);
        }
        stroke.finish()
    };
    match finished {
        Some(rect) => {
            view.apply_edit(&step.tool, Edit::regions(vec![rect], false));
            Ok(())
        }
        None => Err(ToolStrokeError::NothingPainted {
            tool: step.tool.clone(),
        }),
    }
}
