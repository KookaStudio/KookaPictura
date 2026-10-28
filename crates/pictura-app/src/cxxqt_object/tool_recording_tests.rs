//! Tool-stroke recording: the capture path, the bound, and the replay path.

use super::document_session::DocumentSession;
use super::edit::Edit;
use super::impl_paint::tool_recording::{
    self, SharedToolStrokeRecorder, ToolStrokeError, ToolStrokeKind, ToolStrokeRecorder,
    ToolStrokeStep, MAX_TOOL_STROKE_BYTES, MAX_TOOL_STROKE_SAMPLES, TOOL_STROKE_FORMAT_VERSION,
};
use super::state::PictureViewRust;
use super::tests::pixel_layer;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use pictura_paint::spacing::SpacingMode;
use pictura_paint::{PaintMode, Rgba, StrokeConfig, StrokeSample};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct SinkState {
    active: bool,
    steps: Vec<ToolStrokeStep>,
    refused: Vec<ToolStrokeError>,
}

struct Sink(Rc<RefCell<SinkState>>);

impl ToolStrokeRecorder for Sink {
    fn tool_recording_active(&self) -> bool {
        self.0.borrow().active
    }

    fn record_tool_stroke(&mut self, step: ToolStrokeStep) -> Result<(), ToolStrokeError> {
        self.0.borrow_mut().steps.push(step);
        Ok(())
    }

    fn tool_stroke_refused(&mut self, error: &ToolStrokeError) {
        self.0.borrow_mut().refused.push(error.clone());
    }
}

fn install(active: bool) -> Rc<RefCell<SinkState>> {
    let state = Rc::new(RefCell::new(SinkState {
        active,
        steps: Vec::new(),
        refused: Vec::new(),
    }));
    let recorder: SharedToolStrokeRecorder = Rc::new(RefCell::new(Sink(state.clone())));
    tool_recording::install_tool_stroke_recorder(Some(recorder));
    tool_recording::cancel();
    let _ = tool_recording::take_tool_stroke_error();
    state
}

fn reset() {
    tool_recording::install_tool_stroke_recorder(None);
    tool_recording::cancel();
    let _ = tool_recording::take_tool_stroke_error();
}

fn document() -> Document {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 64, 64, (255, 255, 255))]);
    doc
}

fn view() -> PictureViewRust {
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(document()),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

fn brush_config() -> StrokeConfig {
    StrokeConfig {
        color: Rgba {
            r: 220,
            g: 20,
            b: 30,
            a: 255,
        },
        background: Rgba {
            r: 250,
            g: 250,
            b: 250,
            a: 255,
        },
        diameter: 9,
        hardness: 80,
        spacing: SpacingMode::Fixed(20),
        opacity: 70,
        flow: 60,
        mode: PaintMode::Normal,
        ..StrokeConfig::default()
    }
}

fn samples() -> Vec<StrokeSample> {
    (0..12)
        .map(|i| StrokeSample {
            x: 8.0 + i as f32 * 4.0,
            y: 20.0 + (i % 3) as f32 * 5.0,
            pressure: 0.5 + (i % 4) as f32 * 0.1,
        })
        .collect()
}

/// Paint and commit one stroke the way `end_paint` does; `record` feeds the
/// samples to the capture path and closes it.
fn paint(
    view: &mut PictureViewRust,
    cfg: StrokeConfig,
    samples: &[StrokeSample],
    record: bool,
) -> bool {
    if !view.begin_paint(cfg, "Brush") {
        tool_recording::finish(false);
        return false;
    }
    let finished = {
        let Some(mut session) = view.session.live.take_stroke_session() else {
            return false;
        };
        let Some(mut stroke) = session.stroke.take() else {
            return false;
        };
        let Some(doc) = view.session.doc.as_mut() else {
            return false;
        };
        for sample in samples {
            if record {
                tool_recording::sample(sample.x as f64, sample.y as f64, sample.pressure as f64);
            }
            stroke.sample(doc, *sample);
        }
        stroke.finish()
    };
    match finished {
        Some(rect) => {
            let _ = view.apply_edit("Brush", Edit::regions(vec![rect], false));
            tool_recording::finish(record);
            true
        }
        None => {
            tool_recording::finish(false);
            false
        }
    }
}

fn pixels(view: &PictureViewRust) -> Vec<u8> {
    let doc = view.session.doc.as_ref().expect("document");
    doc.layers[0]
        .channels
        .iter()
        .flat_map(|channel| channel.data.iter().copied())
        .collect()
}

fn recorded_step(sink: &Rc<RefCell<SinkState>>) -> ToolStrokeStep {
    let mut state = sink.borrow_mut();
    assert_eq!(state.steps.len(), 1, "one committed step");
    state.steps.remove(0)
}

#[test]
fn recording_off_leaves_the_stroke_unchanged() {
    let sink = install(false);
    let mut recorded = view();
    assert!(paint(&mut recorded, brush_config(), &samples(), true));

    let mut plain = view();
    assert!(paint(&mut plain, brush_config(), &samples(), false));

    assert_eq!(pixels(&recorded), pixels(&plain));
    assert_eq!(recorded.session.history.count(), 1);
    assert_eq!(plain.session.history.count(), 1);
    assert!(sink.borrow().steps.is_empty(), "no step while off");
    reset();
}

#[test]
fn a_recorded_brush_stroke_replays_the_same_geometry_and_history() {
    let sink = install(true);
    let all = samples();
    let mut recorded = view();
    assert!(paint(&mut recorded, brush_config(), &all, true));

    let step = recorded_step(&sink);
    assert_eq!(step.version, TOOL_STROKE_FORMAT_VERSION);
    assert_eq!(step.tool, "Brush");
    assert_eq!(step.kind, ToolStrokeKind::Paint);
    assert_eq!(step.samples.len(), all.len());

    let engine = step.to_engine_stroke();
    assert_eq!(step.payload_bytes(), engine.payload_len());
    assert_eq!(engine.version, TOOL_STROKE_FORMAT_VERSION);
    assert_eq!(engine.tool, "Brush");
    assert_eq!(engine.samples.len(), all.len());
    assert_eq!(
        engine
            .settings()
            .get("diameter")
            .and_then(|value| value.as_u64()),
        Some(9)
    );
    assert_eq!(
        engine
            .settings()
            .get("mode")
            .and_then(|value| value.as_str()),
        Some("normal")
    );

    let mut replayed = view();
    tool_recording::replay_tool_stroke(&mut replayed, &step).expect("replay commits");

    assert_eq!(pixels(&recorded), pixels(&replayed));
    assert_eq!(
        replayed.session.history.count(),
        recorded.session.history.count(),
        "one tool state"
    );
    reset();
}

#[test]
fn settings_are_captured_at_stroke_start() {
    let sink = install(true);
    let captured = brush_config();
    let mut first = view();
    assert!(paint(&mut first, captured, &samples(), true));
    let first_step = recorded_step(&sink);

    assert_eq!(first_step.settings.diameter, captured.diameter);
    assert_eq!(first_step.settings.hardness, captured.hardness);
    assert_eq!(first_step.settings.opacity, captured.opacity);
    assert_eq!(first_step.settings.color, captured.color);
    assert_eq!(first_step.settings.spacing, captured.spacing);

    let changed = StrokeConfig {
        diameter: 41,
        opacity: 25,
        ..captured
    };
    let mut second = view();
    assert!(paint(&mut second, changed, &samples(), true));
    let second_step = recorded_step(&sink);

    assert_eq!(second_step.settings.diameter, 41);
    assert_eq!(second_step.settings.opacity, 25);
    assert_ne!(first_step.settings.diameter, second_step.settings.diameter);
    reset();
}

#[test]
fn a_stroke_over_the_sample_bound_is_refused() {
    let sink = install(true);
    tool_recording::begin("Brush", &brush_config(), ToolStrokeKind::Paint);
    for i in 0..=MAX_TOOL_STROKE_SAMPLES {
        tool_recording::sample(i as f64, 0.0, 1.0);
    }
    tool_recording::finish(true);

    assert!(sink.borrow().steps.is_empty(), "the action is unchanged");
    assert_eq!(
        tool_recording::take_tool_stroke_error(),
        Some(ToolStrokeError::SampleLimit {
            count: MAX_TOOL_STROKE_SAMPLES + 1,
            limit: MAX_TOOL_STROKE_SAMPLES,
        })
    );
    reset();
}

#[test]
fn a_stroke_over_the_payload_bound_is_refused() {
    let sink = install(true);
    let tool = "x".repeat(MAX_TOOL_STROKE_BYTES);
    tool_recording::begin(&tool, &brush_config(), ToolStrokeKind::Paint);
    tool_recording::sample(1.0, 2.0, 1.0);
    tool_recording::finish(true);

    assert!(sink.borrow().steps.is_empty(), "the action is unchanged");
    assert_eq!(sink.borrow().refused.len(), 1);
    match tool_recording::take_tool_stroke_error() {
        Some(ToolStrokeError::PayloadLimit { bytes, limit }) => {
            assert!(bytes > limit);
            assert_eq!(limit, MAX_TOOL_STROKE_BYTES);
        }
        other => panic!("expected a payload-limit diagnostic, got {other:?}"),
    }
    reset();
}

#[test]
fn a_cancelled_stroke_is_not_recorded() {
    let sink = install(true);
    tool_recording::begin("Brush", &brush_config(), ToolStrokeKind::Paint);
    tool_recording::sample(4.0, 4.0, 1.0);
    tool_recording::cancel();
    tool_recording::finish(true);

    assert!(sink.borrow().steps.is_empty());
    reset();
}

#[test]
fn native_and_mask_begins_are_refused_instead_of_recorded() {
    for kind in [ToolStrokeKind::Native, ToolStrokeKind::Mask] {
        let sink = install(true);
        tool_recording::begin("Eraser", &brush_config(), kind);
        tool_recording::sample(4.0, 4.0, 1.0);
        tool_recording::finish(true);

        let expected = ToolStrokeError::ReplayUnsupported {
            tool: "Eraser".into(),
        };
        assert!(sink.borrow().steps.is_empty(), "no dead step is appended");
        assert_eq!(sink.borrow().refused, vec![expected.clone()]);
        assert_eq!(tool_recording::take_tool_stroke_error(), Some(expected));
        reset();
    }
}

#[test]
fn a_serialized_stroke_round_trips_through_the_replay_hook() {
    let sink = install(true);
    let all = samples();
    let mut recorded = view();
    assert!(paint(&mut recorded, brush_config(), &all, true));

    let payload = recorded_step(&sink).to_engine_stroke().to_json();
    let mut replayed = view();
    tool_recording::replay_tool_stroke_json(&mut replayed, &payload).expect("replay commits");

    assert_eq!(pixels(&recorded), pixels(&replayed));
    assert_eq!(
        replayed.session.history.count(),
        recorded.session.history.count()
    );
    assert!(sink.borrow().refused.is_empty());
    reset();
}

#[test]
fn a_bad_payload_or_version_is_refused_with_a_named_error() {
    let sink = install(true);
    let mut view = view();
    assert!(matches!(
        tool_recording::replay_tool_stroke_json(&mut view, "{"),
        Err(ToolStrokeError::InvalidPayload { .. })
    ));
    assert!(matches!(
        tool_recording::take_tool_stroke_error(),
        Some(ToolStrokeError::InvalidPayload { .. })
    ));

    let step = ToolStrokeStep {
        version: TOOL_STROKE_FORMAT_VERSION,
        tool: "Brush".into(),
        kind: ToolStrokeKind::Paint,
        settings: brush_config(),
        samples: samples(),
    };
    let mut engine = step.to_engine_stroke();
    engine.version = TOOL_STROKE_FORMAT_VERSION + 1;
    assert_eq!(
        tool_recording::replay_tool_stroke_json(&mut view, &engine.to_json()),
        Err(ToolStrokeError::UnsupportedVersion {
            found: TOOL_STROKE_FORMAT_VERSION + 1,
        })
    );
    assert_eq!(sink.borrow().refused.len(), 2);
    reset();
}

struct EngineSink {
    recorder: pictura_script::Recorder,
    library: pictura_script::ActionLibrary,
}

impl ToolStrokeRecorder for EngineSink {
    fn tool_recording_active(&self) -> bool {
        self.recorder.is_recording()
    }

    fn record_tool_stroke(&mut self, step: ToolStrokeStep) -> Result<(), ToolStrokeError> {
        self.recorder
            .record_tool_stroke(&mut self.library, true, step.to_engine_stroke())
            .map(|_| ())
            .map_err(|error| ToolStrokeError::RecorderRejected {
                reason: error.to_string(),
            })
    }
}

#[test]
fn a_recorded_stroke_reaches_the_engine_recorder() {
    use pictura_script::{ActionLibrary, Recorder, Selection, StepKind};

    let library = ActionLibrary::with_default();
    let mut recorder = Recorder::new();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("the default set has a recording target");
    let engine = Rc::new(RefCell::new(EngineSink { recorder, library }));
    let shared: SharedToolStrokeRecorder = engine.clone();
    tool_recording::install_tool_stroke_recorder(Some(shared));

    let mut view = view();
    assert!(paint(&mut view, brush_config(), &samples(), true));

    let sink = engine.borrow();
    let step = &sink.library.set(0).expect("set").actions[0]
        .steps
        .last()
        .expect("one tool step");
    match &step.kind {
        StepKind::ToolStroke { stroke } => {
            assert_eq!(stroke.tool, "Brush");
            assert_eq!(stroke.samples.len(), samples().len());
            assert_eq!(stroke.version, TOOL_STROKE_FORMAT_VERSION);
        }
        other => panic!("expected a tool stroke step, got {other:?}"),
    }
    drop(sink);
    reset();
}
