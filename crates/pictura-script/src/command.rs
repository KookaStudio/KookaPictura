//! Command identity, classification, and the versioned parameter payload the
//! recorder and player exchange with the C++ command bus.

use crate::command_catalogue;
use serde_json::{Map, Value};
use std::fmt;

/// A catalogue command id. The id is the dispatch key and never changes when a
/// label or shortcut changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for CommandId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CommandId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// How an executable is represented in an action set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandClass {
    /// A real Adobe event id and descriptor schema.
    AdobeVerified,
    /// A `pictura.<command-id>` event carrying the app's JSON payload.
    AppNative,
    /// Never recorded; only insertable through Insert Menu Item.
    NonRecordable,
}

/// The event id written for a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventId {
    /// A four-character Adobe event id.
    Adobe(&'static str),
    /// An Adobe event id selected only when a parameter has `value`.
    AdobeWhen {
        code: &'static str,
        parameter: &'static str,
        value: &'static str,
    },
    /// `pictura.<command-id>`.
    AppNative,
}

impl EventId {
    /// The four-character Adobe code, when this is an Adobe event.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::Adobe(code) | Self::AdobeWhen { code, .. } => Some(code),
            Self::AppNative => None,
        }
    }

    pub fn is_adobe(self) -> bool {
        self.code().is_some()
    }
}

/// One parameter carried by a command step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterSpec {
    /// The JSON name used by the C++ params object.
    pub name: &'static str,
    /// The four-character Adobe descriptor key, when the parameter has one.
    pub key: Option<&'static str>,
    /// The Adobe unit code for a `UnitFloat` value, when the parameter has one.
    pub unit: Option<&'static str>,
}

const fn param(name: &'static str, key: &'static str) -> ParameterSpec {
    ParameterSpec {
        name,
        key: Some(key),
        unit: None,
    }
}

const fn unit_param(name: &'static str, key: &'static str, unit: &'static str) -> ParameterSpec {
    ParameterSpec {
        name,
        key: Some(key),
        unit: Some(unit),
    }
}

/// Whether executing the command normally opens a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogMode {
    None,
    Optional,
    Required,
    /// Shown off and not editable, but playback opens the dialog.
    Specific,
}

/// One classified executable: its Adobe event mapping and parameter schema.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub class: CommandClass,
    pub event: EventId,
    pub parameters: &'static [ParameterSpec],
    pub dialog: DialogMode,
}

impl CommandDefinition {
    pub fn command_id(&self) -> CommandId {
        CommandId::new(self.id)
    }

    pub fn is_recordable(&self) -> bool {
        self.class != CommandClass::NonRecordable
    }

    /// The event id to write: the app-native form expands the command id.
    pub fn event_id(&self) -> String {
        match self.event {
            EventId::Adobe(code) | EventId::AdobeWhen { code, .. } => code.to_string(),
            EventId::AppNative => format!("pictura.{}", self.id),
        }
    }
}

const ROTATE_PARAMS: &[ParameterSpec] = &[unit_param("angle", "Angl", "#Ang")];
const FLIP_PARAMS: &[ParameterSpec] = &[param("axis", "Axis")];
const FILL_PARAMS: &[ParameterSpec] = &[
    param("contents", "Fl  "),
    unit_param("opacity", "Opct", "#Prc"),
    param("mode", "Md  "),
    param("color", "Clr "),
];
const SAVE_PARAMS: &[ParameterSpec] = &[param("as", "As  "), param("in", "In  ")];
const IMAGE_SIZE_PARAMS: &[ParameterSpec] = &[
    unit_param("width", "Wdth", "#Pxl"),
    unit_param("height", "Hght", "#Pxl"),
    unit_param("resolution", "Rslt", "#Rsl"),
    param("constrainProportions", "CnsP"),
    param("resample", "Rsz "),
];
const CANVAS_SIZE_PARAMS: &[ParameterSpec] = &[
    unit_param("width", "Wdth", "#Pxl"),
    unit_param("height", "Hght", "#Pxl"),
    param("anchor", "Anch"),
];
const GAUSSIAN_PARAMS: &[ParameterSpec] = &[unit_param("radius", "Rds ", "#Pxl")];
const MODE_PARAMS: &[ParameterSpec] = &[param("mode", "Md  ")];

/// The Adobe-verified families this release ships.
#[rustfmt::skip]
static VERIFIED: &[CommandDefinition] = &[
    CommandDefinition { id: "file.save", label: "Save", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("save"), parameters: SAVE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "file.saveAs", label: "Save As…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("save"), parameters: SAVE_PARAMS, dialog: DialogMode::Required },
    CommandDefinition { id: "filter.blur.gaussian.blur", label: "Gaussian Blur", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("GsnB"), parameters: GAUSSIAN_PARAMS, dialog: DialogMode::Optional },
    CommandDefinition { id: "layer.flatten.image", label: "Flatten Image", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("FltI"), parameters: &[], dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.bitmap", label: "Bitmap", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "bitmap" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.grayscale", label: "Grayscale", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "grayscale" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.indexed.color", label: "Indexed Color", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "indexed" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.rgb.color", label: "RGB Color", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "rgb" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.cmyk.color", label: "CMYK Color", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "cmyk" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.mode.lab.color", label: "Lab Color", class: CommandClass::AdobeVerified,
        event: EventId::AdobeWhen { code: "CnvM", parameter: "mode", value: "lab" }, parameters: MODE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.image.size", label: "Image Size…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("ImgS"), parameters: IMAGE_SIZE_PARAMS, dialog: DialogMode::Optional },
    CommandDefinition { id: "image.canvas.size", label: "Canvas Size…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Cnvs"), parameters: CANVAS_SIZE_PARAMS, dialog: DialogMode::Optional },
    CommandDefinition { id: "layer.hide.layers", label: "Hide Layers", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Hd  "), parameters: &[], dialog: DialogMode::None },
    CommandDefinition { id: "layer.contentOptions", label: "Layer Content Options…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("setd"), parameters: &[], dialog: DialogMode::Required },
    CommandDefinition { id: "edit.fill", label: "Fill…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Fl  "), parameters: FILL_PARAMS, dialog: DialogMode::Required },
    CommandDefinition { id: "edit.freeTransform", label: "Free Transform", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Trnf"), parameters: &[], dialog: DialogMode::None },
    CommandDefinition { id: "edit.transform.rotate180", label: "Rotate 180°", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "edit.transform.rotate90cw", label: "Rotate 90° Clockwise", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "edit.transform.rotate90ccw", label: "Rotate 90° Counter Clockwise", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "edit.transform.flipHorizontal", label: "Flip Horizontal", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Flp "), parameters: FLIP_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "edit.transform.flipVertical", label: "Flip Vertical", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Flp "), parameters: FLIP_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.rotate180", label: "180°", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.rotate90cw", label: "90° CW", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.rotate90ccw", label: "90° CCW", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.rotate.arbitrary", label: "Arbitrary…", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Rtte"), parameters: ROTATE_PARAMS, dialog: DialogMode::Optional },
    CommandDefinition { id: "image.flipHorizontal", label: "Flip Canvas Horizontal", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Flp "), parameters: FLIP_PARAMS, dialog: DialogMode::None },
    CommandDefinition { id: "image.flipVertical", label: "Flip Canvas Vertical", class: CommandClass::AdobeVerified,
        event: EventId::Adobe("Flp "), parameters: FLIP_PARAMS, dialog: DialogMode::None },
];

/// Commands the default set or the recorder needs that are not marked
/// implemented in the catalogue (the C++ frame lights them up by id).
#[rustfmt::skip]
static EXTRA: &[CommandDefinition] = &[
    CommandDefinition { id: "select.color.range", label: "Color Range…", class: CommandClass::AppNative,
        event: EventId::AppNative, parameters: &[], dialog: DialogMode::Required },
    CommandDefinition { id: "select.refine.edge", label: "Refine Edge…", class: CommandClass::AppNative,
        event: EventId::AppNative, parameters: &[], dialog: DialogMode::Required },
];

/// Pure navigation, inspection, and preference commands that an action must
/// never capture.
#[rustfmt::skip]
static NON_RECORDABLE: &[&str] = &[
    "file.exit",
    "edit.undo", "edit.redo", "edit.stepForward", "edit.stepBackward",
    "edit.purge.undo", "edit.purge.histories", "edit.purge.clipboard", "edit.purge.all",
    "edit.keyboardShortcuts",
    "edit.preferences.general", "edit.preferences.interface", "edit.preferences.performance",
    "edit.preferences.unitsRulers", "edit.preferences.guidesGridSlices", "edit.preferences.type",
    "type.panels.character", "type.panels.paragraph", "type.panels.paragraphStyles",
    "view.zoomIn", "view.zoomOut", "view.fitOnScreen", "view.actualPixels", "view.freePan",
    "view.snapCanvasToWorkspace", "view.screenMode.standard", "view.screenMode.fullWithMenuBar",
    "view.screenMode.full", "view.rulers", "view.lockGuides", "view.clearGuides", "view.newGuide",
    "view.show.selectionEdges", "view.show.guides", "view.show.smartGuides", "view.toolHints",
    "view.options",
    "window.workspace.essentials", "window.workspace.painting", "window.workspace.photography",
    "window.workspace.typography", "window.workspace.new", "window.workspace.delete",
    "window.workspace.reset", "window.workspace.keyboardShortcuts",
    "window.uiSize", "window.uiEnlarge", "window.uiDecrease", "window.uiReset",
    "window.panels.actions", "window.panels.adjustments", "window.panels.brushes",
    "window.panels.channels", "window.panels.character", "window.panels.cloneSource",
    "window.panels.color", "window.panels.glyphs", "window.panels.gradients",
    "window.panels.histogram", "window.panels.history", "window.panels.info",
    "window.panels.layers", "window.panels.navigator", "window.panels.notes",
    "window.panels.paragraph", "window.panels.paragraphStyles", "window.panels.paths",
    "window.panels.patterns", "window.panels.properties", "window.panels.swatches",
    "window.panels.tools",
    "help.search", "help.system.info", "help.submit.issue", "help.about",
    "help.developer.designSystem",
];

/// The definition for `id`, auto-classifying catalogue commands as app-native
/// unless they are listed non-recordable or Adobe-verified.
pub fn definition(id: &str) -> Option<CommandDefinition> {
    if let Some(def) = VERIFIED.iter().find(|def| def.id == id) {
        return Some(def.clone());
    }
    if let Some(def) = EXTRA.iter().find(|def| def.id == id) {
        return Some(def.clone());
    }
    let spec = command_catalogue::find(id)?;
    if !spec.implemented {
        return None;
    }
    Some(CommandDefinition {
        id: spec.id,
        label: spec.label,
        class: if NON_RECORDABLE.contains(&id) {
            CommandClass::NonRecordable
        } else {
            CommandClass::AppNative
        },
        event: EventId::AppNative,
        parameters: &[],
        dialog: DialogMode::Optional,
    })
}

/// Every classified executable: the implemented catalogue plus the extras.
pub fn definitions() -> Vec<CommandDefinition> {
    let mut out: Vec<CommandDefinition> = VERIFIED.to_vec();
    out.extend(EXTRA.iter().cloned());
    for spec in command_catalogue::command_catalogue() {
        if spec.implemented && !out.iter().any(|def| def.id == spec.id) {
            if let Some(def) = definition(spec.id) {
                out.push(def);
            }
        }
    }
    out
}

/// The Adobe code for an event, or the app-native event id.
pub fn command_id_for_event(event: &str, params: &CommandParams) -> Option<&'static str> {
    if event == "CnvM" {
        return match params.get_str("mode")? {
            "bitmap" => Some("image.mode.bitmap"),
            "grayscale" => Some("image.mode.grayscale"),
            "indexed" => Some("image.mode.indexed.color"),
            "rgb" => Some("image.mode.rgb.color"),
            "cmyk" => Some("image.mode.cmyk.color"),
            "lab" => Some("image.mode.lab.color"),
            _ => None,
        };
    }
    if event == "save" {
        return Some(if params.get("as").is_some() {
            "file.saveAs"
        } else {
            "file.save"
        });
    }
    if event == "Rtte" {
        return rotate_command_id(params);
    }
    if event == "Flp " {
        return flip_command_id(params);
    }
    VERIFIED
        .iter()
        .find(|def| def.event.code() == Some(event))
        .map(|def| def.id)
}

/// Whether a rotate/flip step targets a layer (`Edit > Transform`) rather than
/// the canvas (`Image > Image Rotation`).
fn is_layer_transform(params: &CommandParams) -> bool {
    match params.get("layer") {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value.eq_ignore_ascii_case("layer") || value == "true",
        _ => params
            .get_str("family")
            .is_some_and(|family| family.eq_ignore_ascii_case("edit.transform")),
    }
}

fn angle_degrees(params: &CommandParams) -> Option<f64> {
    let value = params.get("angle")?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn rotate_command_id(params: &CommandParams) -> Option<&'static str> {
    let layer = is_layer_transform(params);
    let angle = angle_degrees(params)?.rem_euclid(360.0);
    let id = if (angle - 180.0).abs() < 1e-6 {
        if layer {
            "edit.transform.rotate180"
        } else {
            "image.rotate180"
        }
    } else if (angle - 90.0).abs() < 1e-6 {
        if layer {
            "edit.transform.rotate90cw"
        } else {
            "image.rotate90cw"
        }
    } else if (angle - 270.0).abs() < 1e-6 {
        if layer {
            "edit.transform.rotate90ccw"
        } else {
            "image.rotate90ccw"
        }
    } else if layer {
        "edit.transform.rotate"
    } else {
        "image.rotate.arbitrary"
    };
    Some(id)
}

fn flip_command_id(params: &CommandParams) -> Option<&'static str> {
    let layer = is_layer_transform(params);
    let axis = params.get_str("axis")?.to_ascii_lowercase();
    let vertical = matches!(axis.as_str(), "vertical" | "v" | "vrtc" | "vertical axis");
    let horizontal = matches!(
        axis.as_str(),
        "horizontal" | "h" | "hrzn" | "horizontal axis"
    );
    match (vertical, horizontal) {
        (true, _) => Some(if layer {
            "edit.transform.flipVertical"
        } else {
            "image.flipVertical"
        }),
        (false, true) => Some(if layer {
            "edit.transform.flipHorizontal"
        } else {
            "image.flipHorizontal"
        }),
        (false, false) => None,
    }
}

/// The definition selected by an Adobe event id and already-decoded params.
pub fn definition_for_event(event: &str, params: &CommandParams) -> Option<CommandDefinition> {
    let id = command_id_for_event(event, params)?;
    definition(id)
}

/// The labels shown for a command in the panel and Insert Menu Item dialog.
pub fn label_for(id: &str) -> &str {
    definition(id)
        .map(|def| def.label)
        .or_else(|| command_catalogue::find(id).map(|spec| spec.label))
        .unwrap_or(id)
}

/// Fail with the first id that has no classification.
pub fn coverage_error_for(ids: &[&str]) -> Result<(), CommandError> {
    for id in ids {
        if definition(id).is_none() {
            return Err(CommandError::Unclassified((*id).to_string()));
        }
    }
    Ok(())
}

/// Implemented catalogue ids with no definition. Empty is the release gate.
pub fn coverage_gaps() -> Vec<&'static str> {
    command_catalogue::command_catalogue()
        .iter()
        .filter(|spec| spec.implemented && definition(spec.id).is_none())
        .map(|spec| spec.id)
        .collect()
}

/// A command step's parameter values: a JSON object.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandParams(Value);

impl CommandParams {
    pub fn new() -> Self {
        Self(Value::Object(Map::new()))
    }

    pub fn from_value(value: Value) -> Self {
        Self(value)
    }

    pub fn from_json(text: &str) -> Result<Self, CommandError> {
        if text.trim().is_empty() {
            return Ok(Self::new());
        }
        let value: Value =
            serde_json::from_str(text).map_err(|err| CommandError::InvalidParams {
                command: String::new(),
                message: err.to_string(),
            })?;
        if !value.is_object() {
            return Err(CommandError::InvalidParams {
                command: String::new(),
                message: "params must be a JSON object".into(),
            });
        }
        Ok(Self(value))
    }

    pub fn to_json(&self) -> String {
        self.0.to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.0.as_object().is_none_or(Map::is_empty)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    pub fn object(&self) -> Option<&Map<String, Value>> {
        self.0.as_object()
    }

    pub fn value(&self) -> &Value {
        &self.0
    }

    pub fn with<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.set(key, value);
        self
    }

    pub fn set<T: Into<Value>>(&mut self, key: &str, value: T) {
        if !self.0.is_object() {
            self.0 = Value::Object(Map::new());
        }
        if let Some(map) = self.0.as_object_mut() {
            map.insert(key.to_string(), value.into());
        }
    }
}

impl Default for CommandParams {
    fn default() -> Self {
        Self::new()
    }
}

/// A command-layer failure that names the command.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("unknown command '{0}'")]
    Unknown(String),
    #[error("command '{0}' has no classification")]
    Unclassified(String),
    #[error("command '{0}' is not recordable")]
    NotRecordable(String),
    #[error("invalid params for '{command}': {message}")]
    InvalidParams { command: String, message: String },
    #[error("no executor for '{command}' at step {step}")]
    MissingExecutor { command: String, step: usize },
}
