//! `.atn`/`.psp` conversion between the exact codec `Raw*` model and the
//! engine's action model. Supported Adobe events map to their verified keys;
//! everything else carries a versioned app-native `TEXT` payload.

use crate::action::{
    Action, ActionSet, ActionStep, Condition, DialogOptions, EventKey, ImportedStepForm, StepFlags,
    StepKind, ToolStroke,
};
use crate::command::{self, CommandId, CommandParams};
use crate::error::ActionError;
use crate::recorder::{MAX_TOOL_PAYLOAD_BYTES, MAX_TOOL_SAMPLES};
use pictura_codec::action_descriptor::{
    read_action_descriptor, write_action_descriptor, ActionClass, ActionDescriptor, ActionKey,
    ActionValue,
};
use pictura_codec::action_file::{
    read_atn as codec_read_atn, read_psp as codec_read_psp, write_atn as codec_write_atn,
    write_psp as codec_write_psp, RawAction, RawActionSet, RawAtn, RawEventId, RawStep,
};
use serde_json::{json, Map, Value};

const CLASS_NULL: [u8; 4] = *b"null";
const KEY_JSON: [u8; 4] = *b"jsn ";
const KEY_MESSAGE: [u8; 4] = *b"Msge";
const KEY_CONTINUE: [u8; 4] = *b"Cntn";
const KEY_NAME: [u8; 4] = *b"Nm  ";
const KEY_COMMAND_ID: [u8; 4] = *b"cmid";
const KEY_COMMAND: [u8; 4] = *b"null";

const EVENT_INSERT_PATH: &str = "pictura.insertPath";
const EVENT_CONDITIONAL: &str = "pictura.conditional";
const EVENT_TOOL_STROKE: &str = "pictura.toolStroke";
const EVENT_MENU_ITEM: &str = "pictura.menuItem";
const EVENT_PLAY: &str = "Ply ";
const EVENT_MENU: &str = "slct";
const EVENT_STOP: &str = "Stop";

/// Read one action set from `.atn` bytes, preserving trailing bytes.
pub fn read_atn(bytes: &[u8]) -> Result<ActionSet, ActionError> {
    let raw = codec_read_atn(bytes).map_err(atn_error)?;
    Ok(decode_set(raw.set, raw.trailing))
}

/// Write one action set as `.atn` bytes, including preserved trailing bytes.
pub fn write_atn(set: &ActionSet) -> Vec<u8> {
    codec_write_atn(&RawAtn {
        set: encode_set(set),
        trailing: set.trailing.clone(),
    })
}

/// Read every set of an `.psp` palette.
pub fn read_psp(bytes: &[u8]) -> Result<Vec<ActionSet>, ActionError> {
    let raw = codec_read_psp(bytes).map_err(atn_error)?;
    Ok(raw
        .into_iter()
        .map(|set| decode_set(set, Vec::new()))
        .collect())
}

/// Read every set decoded before a malformed palette, plus the error. A wholly
/// malformed file returns no sets and the error.
pub fn read_psp_partial(bytes: &[u8]) -> (Vec<ActionSet>, Option<ActionError>) {
    match codec_read_psp(bytes) {
        Ok(raw) => (
            raw.into_iter()
                .map(|set| decode_set(set, Vec::new()))
                .collect(),
            None,
        ),
        Err(err) => {
            let sets = err
                .partial_sets
                .iter()
                .cloned()
                .map(|set| decode_set(set, Vec::new()))
                .collect();
            (sets, Some(ActionError::Atn(err.to_string())))
        }
    }
}

/// Write sets as an `.psp` palette.
pub fn write_psp(sets: &[ActionSet]) -> Vec<u8> {
    let raw: Vec<RawActionSet> = sets.iter().map(encode_set).collect();
    codec_write_psp(&raw)
}

fn atn_error(err: impl std::fmt::Display) -> ActionError {
    ActionError::Atn(err.to_string())
}

fn decode_set(raw: RawActionSet, trailing: Vec<u8>) -> ActionSet {
    ActionSet {
        name: name_from_units(&raw.name),
        expanded: raw.expanded,
        actions: raw.actions.into_iter().map(decode_action).collect(),
        trailing,
        raw_name: Some(raw.name),
    }
}

fn encode_set(set: &ActionSet) -> RawActionSet {
    RawActionSet {
        name: encode_name(&set.name, set.raw_name.as_deref()),
        expanded: set.expanded,
        actions: set.actions.iter().map(encode_action).collect(),
    }
}

fn decode_action(raw: RawAction) -> Action {
    Action {
        name: name_from_units(&raw.name),
        function_key: crate::action::FunctionKey::new(raw.function_key, raw.shift, raw.ctrl),
        color: crate::action::ActionColor(raw.color),
        expanded: raw.expanded,
        steps: raw.steps.into_iter().map(decode_step).collect(),
        raw_name: Some(raw.name),
    }
}

fn encode_action(action: &Action) -> RawAction {
    RawAction {
        function_key: action.function_key.number,
        shift: action.function_key.shift,
        ctrl: action.function_key.ctrl,
        color: action.color.0,
        name: encode_name(&action.name, action.raw_name.as_deref()),
        expanded: action.expanded,
        steps: action.steps.iter().map(encode_step).collect(),
    }
}

// Photoshop writes set/action names with a trailing NUL code unit; the count
// remains authoritative on read, and a name without one still round-trips.
fn name_units(name: &str) -> Vec<u16> {
    let mut units: Vec<u16> = name.encode_utf16().collect();
    if units.last() != Some(&0) {
        units.push(0);
    }
    units
}

/// Re-emit imported units exactly (including a missing trailing NUL and lone
/// surrogates) while the editable name still matches them.
fn encode_name(name: &str, raw: Option<&[u16]>) -> Vec<u16> {
    match raw {
        Some(units) if name_from_units(units).as_str() == name => units.to_vec(),
        _ => name_units(name),
    }
}

fn name_from_units(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

fn decode_step(raw: RawStep) -> ActionStep {
    let event = match &raw.event_id {
        RawEventId::Text(bytes) => EventKey::Text(bytes.clone()),
        RawEventId::Long(code) => EventKey::Code(*code),
    };
    let descriptor = raw
        .descriptor
        .as_ref()
        .map(|desc| write_action_descriptor(desc, true));
    let imported = ImportedStepForm {
        event: event.clone(),
        descriptor: descriptor.clone(),
        display_name: raw.display_name.clone(),
    };
    let mut flags = StepFlags {
        expanded: raw.expanded,
        enabled: raw.enabled,
        with_dialog: raw.with_dialog,
    };
    let display_name = String::from_utf8_lossy(&raw.display_name).into_owned();
    let Ok(mut dialog_options) = DialogOptions::from_raw(raw.dialog_options) else {
        return ActionStep {
            flags,
            dialog_options: DialogOptions::OPTIONAL,
            display_name,
            kind: StepKind::Unknown { event, descriptor },
            imported: Some(imported),
        };
    };
    let params = merged_params(&event, raw.descriptor.as_ref());
    let kind = decode_kind(&event, raw.descriptor.as_ref(), &params);
    if matches!(&kind, StepKind::MenuItem { .. }) {
        dialog_options = DialogOptions::SPECIFIC;
        flags.with_dialog = false;
    }
    ActionStep {
        flags,
        dialog_options,
        display_name,
        kind,
        imported: Some(imported),
    }
}

fn merged_params(event: &EventKey, descriptor: Option<&ActionDescriptor>) -> CommandParams {
    let Some(descriptor) = descriptor else {
        return CommandParams::new();
    };
    let mut params = params_for_event(&event.display(), descriptor);
    if let Some(json) = json_params(descriptor) {
        if let Some(map) = json.value().as_object() {
            for (key, value) in map {
                params.set(key, value.clone());
            }
        }
    }
    let code = event.display();
    if matches!(code.as_str(), "Rtte" | "Flp ")
        && params.get("layer").is_none()
        && descriptor_targets_layer(descriptor)
    {
        params.set("layer", true);
    }
    params
}

/// A layer transform's descriptor carries a layer reference; a canvas rotate
/// or flip does not. The marker lets `command_id_for_event` pick the family.
fn descriptor_targets_layer(descriptor: &ActionDescriptor) -> bool {
    descriptor.items.iter().any(|(_, value)| {
        let ActionValue::Reference(items) = value else {
            return false;
        };
        items
            .iter()
            .any(|item| key_bytes(&item.class.id).as_deref() == Some(&b"Lyr "[..]))
    })
}

fn decode_kind(
    event: &EventKey,
    descriptor: Option<&ActionDescriptor>,
    params: &CommandParams,
) -> StepKind {
    match event {
        EventKey::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes);
            match text.as_ref() {
                EVENT_INSERT_PATH => StepKind::InsertPath {
                    path: params
                        .get_str("path")
                        .map(ToString::to_string)
                        .unwrap_or_else(|| text.to_string()),
                },
                EVENT_CONDITIONAL => decode_conditional(params, descriptor),
                EVENT_TOOL_STROKE => decode_tool_stroke(params, descriptor),
                EVENT_MENU_ITEM => StepKind::MenuItem {
                    command: CommandId::new(
                        params.get_str("command").unwrap_or_default().to_string(),
                    ),
                    params: params
                        .get("params")
                        .cloned()
                        .map(CommandParams::from_value)
                        .unwrap_or_default(),
                },
                _ => {
                    if let Some(id) = text.strip_prefix("pictura.") {
                        if let Some(def) = command::definition(id) {
                            StepKind::Command {
                                command: def.command_id(),
                                params: params.clone(),
                            }
                        } else {
                            unknown(event, descriptor)
                        }
                    } else {
                        unknown(event, descriptor)
                    }
                }
            }
        }
        EventKey::Code(code) => {
            let code = String::from_utf8_lossy(code);
            match code.as_ref() {
                EVENT_STOP => StepKind::Stop {
                    message: descriptor_text(descriptor, KEY_MESSAGE).unwrap_or_default(),
                    allow_continue: descriptor_bool(descriptor, KEY_CONTINUE).unwrap_or(false),
                },
                EVENT_MENU => StepKind::MenuItem {
                    command: CommandId::new(
                        params
                            .get_str("command")
                            .map(ToString::to_string)
                            .or_else(|| descriptor_text(descriptor, KEY_COMMAND))
                            .unwrap_or_default(),
                    ),
                    params: params
                        .get("params")
                        .cloned()
                        .map(CommandParams::from_value)
                        .unwrap_or_default(),
                },
                EVENT_PLAY => StepKind::Play {
                    action: descriptor_text(descriptor, KEY_NAME)
                        .or_else(|| params.get_str("action").map(ToString::to_string))
                        .unwrap_or_default(),
                },
                _ => {
                    let stored = descriptor_text(descriptor, KEY_COMMAND_ID);
                    let def = stored
                        .as_deref()
                        .and_then(command::definition)
                        .or_else(|| command::definition_for_event(code.as_ref(), params));
                    match def {
                        Some(def) => StepKind::Command {
                            command: def.command_id(),
                            params: params.clone(),
                        },
                        None => unknown(event, descriptor),
                    }
                }
            }
        }
    }
}

fn decode_conditional(params: &CommandParams, descriptor: Option<&ActionDescriptor>) -> StepKind {
    let condition = params
        .get("condition")
        .cloned()
        .and_then(|value| serde_json::from_value::<Condition>(value).ok());
    match condition {
        Some(condition) => StepKind::Conditional {
            condition,
            then_action: params.get_str("then").map(ToString::to_string),
            else_action: params.get_str("else").map(ToString::to_string),
        },
        None => unknown(&EventKey::text(EVENT_CONDITIONAL), descriptor),
    }
}

fn decode_tool_stroke(params: &CommandParams, descriptor: Option<&ActionDescriptor>) -> StepKind {
    let Ok(stroke) = serde_json::from_value::<ToolStroke>(params.value().clone()) else {
        return unknown(&EventKey::text(EVENT_TOOL_STROKE), descriptor);
    };
    if stroke.version != ToolStroke::VERSION
        || stroke.samples.len() > MAX_TOOL_SAMPLES
        || stroke.payload_len() > MAX_TOOL_PAYLOAD_BYTES
    {
        return unknown(&EventKey::text(EVENT_TOOL_STROKE), descriptor);
    }
    StepKind::ToolStroke { stroke }
}

fn unknown(event: &EventKey, descriptor: Option<&ActionDescriptor>) -> StepKind {
    StepKind::Unknown {
        event: event.clone(),
        descriptor: descriptor.map(|desc| write_action_descriptor(desc, true)),
    }
}

fn encode_step(step: &ActionStep) -> RawStep {
    let (event_id, descriptor) = if let Some(imported) = &step.imported {
        (
            event_id(&imported.event),
            imported
                .descriptor
                .as_deref()
                .and_then(|bytes| read_action_descriptor(bytes, true).ok()),
        )
    } else {
        encode_step_kind(step)
    };
    let display_name = step
        .imported
        .as_ref()
        .filter(|imported| {
            String::from_utf8_lossy(&imported.display_name).as_bytes()
                == step.display_name.as_bytes()
        })
        .map(|imported| imported.display_name.clone())
        .unwrap_or_else(|| step.display_name.as_bytes().to_vec());
    RawStep {
        expanded: step.flags.expanded,
        enabled: step.flags.enabled,
        with_dialog: step.flags.with_dialog,
        dialog_options: step.dialog_options.written(),
        event_id,
        display_name,
        descriptor,
    }
}

fn event_id(event: &EventKey) -> RawEventId {
    match event {
        EventKey::Text(bytes) => RawEventId::Text(bytes.clone()),
        EventKey::Code(code) => RawEventId::Long(*code),
    }
}

fn encode_step_kind(step: &ActionStep) -> (RawEventId, Option<ActionDescriptor>) {
    match &step.kind {
        StepKind::Command { command, params } => command_event(command, params),
        StepKind::Stop {
            message,
            allow_continue,
        } => (
            RawEventId::Long(*b"Stop"),
            Some(descriptor(
                EVENT_STOP,
                vec![
                    (
                        ActionKey::Code(KEY_MESSAGE),
                        ActionValue::Text(message.clone()),
                    ),
                    (
                        ActionKey::Code(KEY_CONTINUE),
                        ActionValue::Boolean(*allow_continue),
                    ),
                ],
            )),
        ),
        StepKind::MenuItem { command, params } => (
            RawEventId::Long(*b"slct"),
            Some(descriptor(
                EVENT_MENU,
                vec![
                    (
                        ActionKey::Code(KEY_COMMAND),
                        ActionValue::Text(command.as_str().to_string()),
                    ),
                    json_item(&json!({
                        "command": command.as_str(),
                        "params": params.value(),
                    })),
                ],
            )),
        ),
        StepKind::InsertPath { path } => (
            RawEventId::Text(EVENT_INSERT_PATH.as_bytes().to_vec()),
            Some(descriptor(
                EVENT_INSERT_PATH,
                vec![json_item(&json!({ "path": path }))],
            )),
        ),
        StepKind::Play { action } => (
            RawEventId::Long(*b"Ply "),
            Some(descriptor(
                EVENT_PLAY,
                vec![(ActionKey::Code(KEY_NAME), ActionValue::Text(action.clone()))],
            )),
        ),
        StepKind::Conditional {
            condition,
            then_action,
            else_action,
        } => (
            RawEventId::Text(EVENT_CONDITIONAL.as_bytes().to_vec()),
            Some(descriptor(
                EVENT_CONDITIONAL,
                vec![json_item(&json!({
                    "v": 1,
                    "condition": condition,
                    "then": then_action,
                    "else": else_action,
                }))],
            )),
        ),
        StepKind::ToolStroke { stroke } => {
            let payload = serde_json::to_value(stroke).unwrap_or(Value::Null);
            (
                RawEventId::Text(EVENT_TOOL_STROKE.as_bytes().to_vec()),
                Some(descriptor(EVENT_TOOL_STROKE, vec![json_item(&payload)])),
            )
        }
        StepKind::Unknown { event, descriptor } => {
            let descriptor = descriptor
                .as_deref()
                .and_then(|bytes| read_action_descriptor(bytes, true).ok());
            (event_id(event), descriptor)
        }
    }
}

fn command_event(
    command: &CommandId,
    params: &CommandParams,
) -> (RawEventId, Option<ActionDescriptor>) {
    let Some(def) = command::definition(command.as_str()) else {
        return (
            RawEventId::Text(format!("pictura.{}", command.as_str()).into_bytes()),
            Some(descriptor(
                command.as_str(),
                vec![json_item(params.value())],
            )),
        );
    };
    match def.event {
        command::EventId::Adobe(_) | command::EventId::AdobeWhen { .. } => {
            let mut items = Vec::new();
            for spec in def.parameters {
                let Some(value) = params.get(spec.name) else {
                    continue;
                };
                let Some(key) = spec.key else {
                    continue;
                };
                if let Ok(code) = <[u8; 4]>::try_from(key.as_bytes()) {
                    items.push((ActionKey::Code(code), param_value(spec, value)));
                }
            }
            if let command::EventId::AdobeWhen {
                parameter, value, ..
            } = def.event
            {
                let spec = def.parameters.iter().find(|spec| spec.name == parameter);
                if let Some(code) = spec
                    .and_then(|spec| spec.key)
                    .and_then(|key| <[u8; 4]>::try_from(key.as_bytes()).ok())
                {
                    if !items
                        .iter()
                        .any(|(key, _)| key_bytes(key).as_deref() == Some(code.as_slice()))
                    {
                        let encoded = match spec {
                            Some(spec) => param_value(spec, &Value::String(value.to_string())),
                            None => ActionValue::Text(value.to_string()),
                        };
                        items.push((ActionKey::Code(code), encoded));
                    }
                }
            }
            items.push((
                ActionKey::Code(KEY_COMMAND_ID),
                ActionValue::Text(command.as_str().to_string()),
            ));
            items.push(json_item(params.value()));
            (
                RawEventId::Long(fourcc(def.event_id())),
                Some(descriptor(command.as_str(), items)),
            )
        }
        command::EventId::AppNative => (
            RawEventId::Text(def.event_id().into_bytes()),
            Some(descriptor(
                command.as_str(),
                vec![
                    (
                        ActionKey::Code(KEY_COMMAND_ID),
                        ActionValue::Text(command.as_str().to_string()),
                    ),
                    json_item(params.value()),
                ],
            )),
        ),
    }
}

fn fourcc(code: String) -> [u8; 4] {
    let bytes = code.as_bytes();
    let mut out = [b' '; 4];
    let len = bytes.len().min(4);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn param_value(spec: &crate::command::ParameterSpec, value: &Value) -> ActionValue {
    if let Some(unit) = spec.unit {
        if let Some(number) = value.as_f64() {
            return ActionValue::UnitFloat {
                unit: fourcc(unit.to_string()),
                value: number,
            };
        }
    }
    if spec.name == "mode" {
        if let Some(mode) = value.as_str() {
            let code = match mode {
                "bitmap" => *b"Btm ",
                "grayscale" => *b"Grys",
                "indexed" => *b"Indx",
                "rgb" => *b"RGB ",
                "cmyk" => *b"CMYK",
                "lab" => *b"Lab ",
                _ => *b"    ",
            };
            return ActionValue::Enumerated {
                kind: ActionKey::Code(*b"Md  "),
                value: ActionKey::Code(code),
            };
        }
    }
    match value {
        Value::Bool(value) => ActionValue::Boolean(*value),
        Value::Number(number) => match number.as_i64() {
            Some(value) => ActionValue::Integer(value as i32),
            None => ActionValue::Double(number.as_f64().unwrap_or(0.0)),
        },
        Value::String(value) => ActionValue::Text(value.clone()),
        Value::Null => ActionValue::Text(String::new()),
        other => ActionValue::Text(other.to_string()),
    }
}

fn descriptor(name: &str, items: Vec<(ActionKey, ActionValue)>) -> ActionDescriptor {
    ActionDescriptor {
        name: name.to_string(),
        class: ActionClass {
            name: "null".to_string(),
            id: ActionKey::Code(CLASS_NULL),
        },
        items,
    }
}

fn json_item(value: &Value) -> (ActionKey, ActionValue) {
    (
        ActionKey::Code(KEY_JSON),
        ActionValue::Text(value.to_string()),
    )
}

fn json_params(desc: &ActionDescriptor) -> Option<CommandParams> {
    for (key, value) in &desc.items {
        if key_bytes(key).as_deref() == Some(KEY_JSON.as_slice()) {
            let ActionValue::Text(text) = value else {
                continue;
            };
            if let Ok(value) = serde_json::from_str::<Value>(text) {
                return Some(CommandParams::from_value(value));
            }
        }
    }
    None
}

fn descriptor_text(desc: Option<&ActionDescriptor>, key: [u8; 4]) -> Option<String> {
    let desc = desc?;
    for (item_key, value) in &desc.items {
        if key_bytes(item_key).as_deref() == Some(key.as_slice()) {
            if let ActionValue::Text(text) = value {
                return Some(text.clone());
            }
        }
    }
    None
}

fn descriptor_bool(desc: Option<&ActionDescriptor>, key: [u8; 4]) -> Option<bool> {
    let desc = desc?;
    for (item_key, value) in &desc.items {
        if key_bytes(item_key).as_deref() == Some(key.as_slice()) {
            if let ActionValue::Boolean(value) = value {
                return Some(*value);
            }
        }
    }
    None
}

fn key_bytes(key: &ActionKey) -> Option<Vec<u8>> {
    match key {
        ActionKey::Code(code) => Some(code.to_vec()),
        ActionKey::String(bytes) => Some(bytes.clone()),
    }
}

/// Extract the mapped keys for imported Adobe steps that have no JSON sidecar.
fn params_for_event(event: &str, desc: &ActionDescriptor) -> CommandParams {
    for def in command::definitions()
        .iter()
        .filter(|def| def.event.code() == Some(event))
    {
        let params = params_from_descriptor(def, desc);
        if !params.is_empty() {
            return params;
        }
    }
    CommandParams::new()
}

fn params_from_descriptor(
    def: &command::CommandDefinition,
    desc: &ActionDescriptor,
) -> CommandParams {
    let mut map = Map::new();
    for spec in def.parameters {
        let Some(key) = spec
            .key
            .and_then(|key| <[u8; 4]>::try_from(key.as_bytes()).ok())
        else {
            continue;
        };
        let Some(value) = desc.items.iter().find_map(|(item_key, value)| {
            (key_bytes(item_key).as_deref() == Some(key.as_slice())).then_some(value)
        }) else {
            continue;
        };
        map.insert(spec.name.to_string(), action_value_json(value));
    }
    CommandParams::from_value(Value::Object(map))
}

fn action_value_json(value: &ActionValue) -> Value {
    match value {
        ActionValue::Double(value) => json!(value),
        ActionValue::Integer(value) => json!(value),
        ActionValue::LargeInteger(value) => json!(value),
        ActionValue::Boolean(value) => json!(value),
        ActionValue::Text(value) => json!(value),
        ActionValue::UnitFloat { value, .. } => json!(value),
        ActionValue::UnitFloats { values, .. } => json!(values),
        ActionValue::Enumerated { value, .. } => json!(enum_name(value)),
        ActionValue::List(values) => Value::Array(values.iter().map(action_value_json).collect()),
        _ => Value::Null,
    }
}

fn enum_name(key: &ActionKey) -> String {
    let text = match key {
        ActionKey::Code(code) => String::from_utf8_lossy(code).trim().to_string(),
        ActionKey::String(bytes) => String::from_utf8_lossy(bytes).trim().to_string(),
    };
    match text.as_str() {
        "Grys" => "grayscale".into(),
        "RGB" => "rgb".into(),
        "CMYK" => "cmyk".into(),
        "Lab" => "lab".into(),
        "Indx" => "indexed".into(),
        "Btm" => "bitmap".into(),
        _ => text,
    }
}
