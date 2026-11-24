//! The serializable action model: sets, actions, steps, function keys, colors,
//! and the app-native step kinds (tool strokes and conditionals).

use crate::command::{CommandId, CommandParams};
use crate::error::ActionError;
use pictura_codec::action_descriptor::{
    read_action_descriptor, write_action_descriptor, ActionKey, ActionValue,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

const KEY_NAME: [u8; 4] = *b"Nm  ";
const KEY_JSON: [u8; 4] = *b"jsn ";
const JSON_ACTION: &str = "action";
const JSON_CONDITION: &str = "condition";
const JSON_THEN: &str = "then";
const JSON_ELSE: &str = "else";

type JsonReplacement = (usize, usize, String);

fn key_is(key: &ActionKey, expected: [u8; 4]) -> bool {
    match key {
        ActionKey::Code(code) => *code == expected,
        ActionKey::String(bytes) => bytes.as_slice() == expected,
    }
}

fn skip_json_whitespace(text: &str, mut index: usize) -> usize {
    while let Some(byte) = text.as_bytes().get(index) {
        if !byte.is_ascii_whitespace() {
            break;
        }
        index += 1;
    }
    index
}

fn json_string_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut escaped = false;
    for (offset, byte) in bytes.iter().enumerate().skip(start + 1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            return Some(offset + 1);
        }
    }
    None
}

fn json_value_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    match *bytes.get(start)? {
        b'"' => json_string_end(text, start),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut in_string = false;
            let mut escaped = false;
            for (offset, byte) in bytes.iter().enumerate().skip(start) {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if *byte == b'\\' {
                        escaped = true;
                    } else if *byte == b'"' {
                        in_string = false;
                    }
                } else {
                    match *byte {
                        b'"' => in_string = true,
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth = depth.checked_sub(1)?;
                            if depth == 0 {
                                return Some(offset + 1);
                            }
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        _ => {
            let mut end = start;
            while let Some(byte) = bytes.get(end) {
                if byte.is_ascii_whitespace() || matches!(*byte, b',' | b']' | b'}') {
                    break;
                }
                end += 1;
            }
            (end > start).then_some(end)
        }
    }
}

fn json_object_members(text: &str) -> Option<Vec<(String, usize, usize)>> {
    let bytes = text.as_bytes();
    let mut index = skip_json_whitespace(text, 0);
    if bytes.get(index) != Some(&b'{') {
        return None;
    }
    index = skip_json_whitespace(text, index + 1);
    let mut members = Vec::new();
    if bytes.get(index) == Some(&b'}') {
        return Some(members);
    }
    loop {
        let key_start = index;
        let key_end = json_string_end(text, key_start)?;
        let key = serde_json::from_str(&text[key_start..key_end]).ok()?;
        index = skip_json_whitespace(text, key_end);
        if bytes.get(index) != Some(&b':') {
            return None;
        }
        let value_start = skip_json_whitespace(text, index + 1);
        let value_end = json_value_end(text, value_start)?;
        members.push((key, value_start, value_end));
        index = skip_json_whitespace(text, value_end);
        match bytes.get(index) {
            Some(&b',') => index = skip_json_whitespace(text, index + 1),
            Some(&b'}') => return Some(members),
            _ => return None,
        }
    }
}

fn json_member_replacements(
    text: &str,
    members: &[(String, usize, usize)],
    key: &str,
    before: Option<&str>,
    after: Option<&str>,
) -> Option<(bool, Vec<JsonReplacement>)> {
    let mut found = false;
    let mut replacements = Vec::new();
    for (member_key, start, end) in members {
        if member_key != key {
            continue;
        }
        found = true;
        let raw = &text[*start..*end];
        match before {
            Some(old) => {
                let Ok(value) = serde_json::from_str::<String>(raw) else {
                    return None;
                };
                if value != old {
                    return None;
                }
                if let Some(new) = after {
                    replacements.push((*start, *end, serde_json::to_string(new).ok()?));
                }
            }
            None => {
                let Ok(value) = serde_json::from_str::<Value>(raw) else {
                    return None;
                };
                if value != Value::Null {
                    return None;
                }
            }
        }
    }
    Some((found, replacements))
}

fn replace_json_spans(text: &str, mut replacements: Vec<JsonReplacement>) -> Option<String> {
    replacements.sort_by_key(|(start, _, _)| *start);
    let mut patched = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, replacement) in replacements {
        if start < cursor || end > text.len() {
            return None;
        }
        patched.push_str(&text[cursor..start]);
        patched.push_str(&replacement);
        cursor = end;
    }
    patched.push_str(&text[cursor..]);
    Some(patched)
}

/// The recorded dialog-options byte (0–4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogOptions(u8);

impl DialogOptions {
    pub const OPTIONAL: Self = Self(0);
    pub const REQUIRED: Self = Self(1);
    pub const NONE: Self = Self(2);
    pub const SPECIFIC: Self = Self(3);
    pub const STOP_CONTINUE: Self = Self(4);

    pub fn from_raw(raw: u8) -> Result<Self, ActionError> {
        if raw <= 4 {
            Ok(Self(raw))
        } else {
            Err(ActionError::InvalidDialogOptions(raw))
        }
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    /// The value an `.atn` writes; stop/continue is written as optional.
    pub fn written(self) -> u8 {
        if self.0 == 4 {
            0
        } else {
            self.0
        }
    }

    /// Whether playback opens the command's dialog.
    pub fn opens_dialog(self, with_dialog: bool) -> bool {
        match self.0 {
            1 | 3 => true,
            2 => false,
            _ => with_dialog,
        }
    }

    /// Whether the UI's modal control is shown on.
    pub fn modal_shown(self, with_dialog: bool) -> bool {
        match self.0 {
            1 => true,
            2 | 3 => false,
            _ => with_dialog,
        }
    }

    /// Whether the user may toggle the modal control.
    pub fn modal_editable(self) -> bool {
        self.0 == 0 || self.0 == 4
    }

    pub fn modal_state(self, with_dialog: bool) -> ModalState {
        match self.0 {
            1 => ModalState::Required,
            2 => ModalState::None,
            3 => ModalState::Specific,
            _ if with_dialog => ModalState::On,
            _ => ModalState::Off,
        }
    }
}

/// The panel's modal cell state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalState {
    On,
    Off,
    Required,
    None,
    Specific,
}

impl ModalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
            Self::Required => "required",
            Self::None => "none",
            Self::Specific => "specific",
        }
    }
}

/// The per-step flags carried by every step kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepFlags {
    pub expanded: bool,
    pub enabled: bool,
    pub with_dialog: bool,
}

impl Default for StepFlags {
    fn default() -> Self {
        Self {
            expanded: true,
            enabled: true,
            with_dialog: false,
        }
    }
}

/// A function-key assignment. Imported values outside F1–F12 are preserved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FunctionKey {
    /// 0 means none, 1–12 mean F1–F12, other values are preserved.
    pub number: u16,
    pub shift: bool,
    pub ctrl: bool,
}

impl FunctionKey {
    pub const NONE: Self = Self {
        number: 0,
        shift: false,
        ctrl: false,
    };

    pub fn new(number: u16, shift: bool, ctrl: bool) -> Self {
        Self {
            number,
            shift,
            ctrl,
        }
    }

    pub fn is_assigned(self) -> bool {
        self.number != 0
    }

    /// Whether the assignment UI may offer this combination.
    pub fn is_standard(self) -> bool {
        (1..=12).contains(&self.number)
    }

    pub fn display(self) -> String {
        if !self.is_assigned() {
            return String::new();
        }
        let mut out = String::new();
        if self.shift {
            out.push_str("Shift+");
        }
        if self.ctrl {
            out.push_str("Ctrl+");
        }
        if self.number >= 1 {
            out.push('F');
            out.push_str(&self.number.to_string());
        }
        out
    }

    pub fn is_allowed_on(self, platform: Platform) -> bool {
        if platform != Platform::Windows {
            return true;
        }
        !(self.number == 1 || (self.ctrl && (self.number == 4 || self.number == 6)))
    }
}

/// The platform seam for function-key rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOs,
    Windows,
}

/// A button color: 0 none, 1 red, 2 orange, 3 yellow, 4 green, 5 blue,
/// 6 violet, 7 gray.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionColor(pub u16);

impl ActionColor {
    pub const NONE: Self = Self(0);
    pub const RED: Self = Self(1);
    pub const ORANGE: Self = Self(2);
    pub const YELLOW: Self = Self(3);
    pub const GREEN: Self = Self(4);
    pub const BLUE: Self = Self(5);
    pub const VIOLET: Self = Self(6);
    pub const GRAY: Self = Self(7);

    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "None",
            1 => "Red",
            2 => "Orange",
            3 => "Yellow",
            4 => "Green",
            5 => "Blue",
            6 => "Violet",
            7 => "Gray",
            _ => "Unknown",
        }
    }
}

/// An imported event id, preserving code vs byte-string form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKey {
    Text(Vec<u8>),
    Code([u8; 4]),
}

impl EventKey {
    pub fn text(value: impl AsRef<[u8]>) -> Self {
        Self::Text(value.as_ref().to_vec())
    }

    pub fn display(&self) -> String {
        match self {
            Self::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            Self::Code(code) => String::from_utf8_lossy(code).into_owned(),
        }
    }
}

/// One tool stroke captured while Allow Tool Recording is on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolStroke {
    pub version: u32,
    pub tool: String,
    pub settings: Value,
    pub samples: Vec<ToolSample>,
}

/// One ordered input sample of a tool stroke.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSample {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
    pub time: f64,
}

impl ToolStroke {
    pub const VERSION: u32 = 1;

    pub fn new(tool: impl Into<String>, settings: CommandParams, samples: Vec<ToolSample>) -> Self {
        Self {
            version: Self::VERSION,
            tool: tool.into(),
            settings: settings.value().clone(),
            samples,
        }
    }

    pub fn settings(&self) -> CommandParams {
        CommandParams::from_value(self.settings.clone())
    }

    pub fn payload_len(&self) -> usize {
        serde_json::to_vec(self).map_or(usize::MAX, |bytes| bytes.len())
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|err| err.to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// A conditional condition. This release ships three document checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    DocumentIsSquare,
    DocumentMode(String),
    DocumentHasSelection,
}

impl Condition {
    pub fn evaluate(&self, facts: &DocumentFacts) -> bool {
        if !facts.present {
            return false;
        }
        match self {
            Self::DocumentIsSquare => facts.square,
            Self::DocumentMode(mode) => facts.mode.eq_ignore_ascii_case(mode),
            Self::DocumentHasSelection => facts.has_selection,
        }
    }
}

/// The active document facts a conditional evaluates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentFacts {
    pub present: bool,
    pub square: bool,
    pub mode: String,
    pub has_selection: bool,
}

/// One step's executable payload.
#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    Command {
        command: CommandId,
        params: CommandParams,
    },
    Stop {
        message: String,
        allow_continue: bool,
    },
    MenuItem {
        command: CommandId,
        params: CommandParams,
    },
    InsertPath {
        path: String,
    },
    Play {
        action: String,
    },
    Conditional {
        condition: Condition,
        then_action: Option<String>,
        else_action: Option<String>,
    },
    ToolStroke {
        stroke: ToolStroke,
    },
    Unknown {
        event: EventKey,
        descriptor: Option<Vec<u8>>,
    },
}

/// An imported step's original event, descriptor bytes, and display-name bytes.
/// `ActionStep::imported` keeps them so an unedited step re-emits unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedStepForm {
    pub event: EventKey,
    pub descriptor: Option<Vec<u8>>,
    pub display_name: Vec<u8>,
}

/// A single action step.
#[derive(Clone, Debug)]
pub struct ActionStep {
    pub flags: StepFlags,
    pub dialog_options: DialogOptions,
    pub display_name: String,
    kind: StepKind,
    /// The form read from disk, invalidated when a payload edit cannot preserve it.
    imported: Option<ImportedStepForm>,
}

impl PartialEq for ActionStep {
    fn eq(&self, other: &Self) -> bool {
        self.flags == other.flags
            && self.dialog_options == other.dialog_options
            && self.display_name == other.display_name
            && self.kind == other.kind
    }
}

impl ActionStep {
    /// Build a step from its executable payload with no preserved form.
    pub fn new(kind: StepKind) -> Self {
        Self {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: String::new(),
            kind,
            imported: None,
        }
    }

    /// Attach the form read from disk. Only the ATN decoder calls this.
    pub(crate) fn with_imported(kind: StepKind, imported: ImportedStepForm) -> Self {
        let mut step = Self::new(kind);
        step.imported = Some(imported);
        step
    }

    pub fn kind(&self) -> &StepKind {
        &self.kind
    }

    pub(crate) fn into_kind(self) -> StepKind {
        self.kind
    }

    pub fn imported(&self) -> Option<&ImportedStepForm> {
        self.imported.as_ref()
    }

    /// Replace the payload, keeping the preserved form only when it is unchanged.
    pub fn set_kind(&mut self, kind: StepKind) {
        if self.kind != kind {
            self.kind = kind;
            self.imported = None;
        }
    }

    /// Edit a Command or MenuItem's parameters. Returns whether the step accepts
    /// parameter edits, whether or not the value changed. The preserved form is
    /// dropped only when the value actually differs.
    pub fn set_params(&mut self, params: CommandParams) -> bool {
        match &mut self.kind {
            StepKind::Command { params: value, .. } | StepKind::MenuItem { params: value, .. } => {
                if *value != params {
                    *value = params;
                    self.imported = None;
                }
                true
            }
            _ => false,
        }
    }

    /// Rewrite a Play or Conditional reference from `old` to `new`, keeping the
    /// preserved descriptor's unrelated content when it can be patched.
    pub fn rewrite_action_reference(&mut self, old: &str, new: &str) -> bool {
        if old == new {
            return false;
        }
        enum Edit {
            Play,
            Conditional {
                condition: Condition,
                before: (Option<String>, Option<String>),
                after: (Option<String>, Option<String>),
            },
        }
        let edit = match &mut self.kind {
            StepKind::Play { action } if action == old => {
                *action = new.to_owned();
                Some(Edit::Play)
            }
            StepKind::Conditional {
                condition,
                then_action,
                else_action,
                ..
            } => {
                let before = (then_action.clone(), else_action.clone());
                let mut changed = false;
                if then_action.as_deref() == Some(old) {
                    *then_action = Some(new.to_owned());
                    changed = true;
                }
                if else_action.as_deref() == Some(old) {
                    *else_action = Some(new.to_owned());
                    changed = true;
                }
                let after = (then_action.clone(), else_action.clone());
                changed.then_some(Edit::Conditional {
                    condition: condition.clone(),
                    before,
                    after,
                })
            }
            _ => None,
        };
        match edit {
            Some(Edit::Play) => {
                self.patch_imported_play(old, new);
                true
            }
            Some(Edit::Conditional {
                condition,
                before,
                after,
            }) => {
                self.patch_imported_conditional(&condition, &before, &after);
                true
            }
            None => false,
        }
    }

    fn patch_imported_play(&mut self, old: &str, new: &str) {
        let Some(imported) = self.imported.clone() else {
            return;
        };
        let Some(bytes) = imported.descriptor.as_deref() else {
            self.imported = None;
            return;
        };
        let Ok(mut descriptor) = read_action_descriptor(bytes, true) else {
            self.imported = None;
            return;
        };
        let mut has_reference = false;
        let mut jsn_patches = Vec::new();
        for (index, (key, value)) in descriptor.items.iter().enumerate() {
            if key_is(key, KEY_NAME) {
                if !matches!(value, ActionValue::Text(text) if text.as_str() == old) {
                    self.imported = None;
                    return;
                }
                has_reference = true;
                continue;
            }
            if !key_is(key, KEY_JSON) {
                continue;
            }
            let ActionValue::Text(text) = value else {
                continue;
            };
            let Ok(parsed) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if !parsed.is_object() {
                continue;
            }
            let Some(members) = json_object_members(text) else {
                self.imported = None;
                return;
            };
            let Some((found, replacements)) =
                json_member_replacements(text, &members, JSON_ACTION, Some(old), Some(new))
            else {
                self.imported = None;
                return;
            };
            if !found {
                continue;
            }
            let Some(patched) = replace_json_spans(text, replacements) else {
                self.imported = None;
                return;
            };
            has_reference = true;
            jsn_patches.push((index, patched));
        }
        if !has_reference {
            self.imported = None;
            return;
        }
        for (key, value) in &mut descriptor.items {
            if key_is(key, KEY_NAME) {
                *value = ActionValue::Text(new.to_owned());
            }
        }
        for (index, text) in jsn_patches {
            descriptor.items[index].1 = ActionValue::Text(text);
        }
        if let Some(form) = &mut self.imported {
            form.descriptor = Some(write_action_descriptor(&descriptor, true));
        }
    }

    fn patch_imported_conditional(
        &mut self,
        condition: &Condition,
        before: &(Option<String>, Option<String>),
        after: &(Option<String>, Option<String>),
    ) {
        let Some(imported) = self.imported.clone() else {
            return;
        };
        let Some(bytes) = imported.descriptor.as_deref() else {
            self.imported = None;
            return;
        };
        let Ok(mut descriptor) = read_action_descriptor(bytes, true) else {
            self.imported = None;
            return;
        };
        let mut found = false;
        let mut decoded_seen = false;
        for (key, value) in &mut descriptor.items {
            if !key_is(key, KEY_JSON) {
                continue;
            }
            let ActionValue::Text(text) = value else {
                continue;
            };
            let Ok(parsed) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            let decoded = !decoded_seen;
            decoded_seen = true;
            let Some(object) = parsed.as_object() else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            let Some(raw_condition) = object.get(JSON_CONDITION) else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            let Ok(sidecar_condition) = serde_json::from_value::<Condition>(raw_condition.clone())
            else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            if &sidecar_condition != condition {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            }
            let Some(members) = json_object_members(text) else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            let Some((_, then_replacements)) = json_member_replacements(
                text,
                &members,
                JSON_THEN,
                before.0.as_deref(),
                after.0.as_deref(),
            ) else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            let Some((_, else_replacements)) = json_member_replacements(
                text,
                &members,
                JSON_ELSE,
                before.1.as_deref(),
                after.1.as_deref(),
            ) else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            let mut replacements = then_replacements;
            replacements.extend(else_replacements);
            if replacements.is_empty() {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            }
            let Some(patched) = replace_json_spans(text, replacements) else {
                if decoded {
                    self.imported = None;
                    return;
                }
                continue;
            };
            *value = ActionValue::Text(patched);
            found = true;
        }
        if !decoded_seen || !found {
            self.imported = None;
            return;
        }
        if let Some(form) = &mut self.imported {
            form.descriptor = Some(write_action_descriptor(&descriptor, true));
        }
    }

    pub fn command(command: impl Into<CommandId>, params: CommandParams) -> Self {
        let command = command.into();
        let label = crate::command::label_for(command.as_str()).to_string();
        let mut step = Self::new(StepKind::Command { command, params });
        step.display_name = label;
        step
    }

    pub fn modal_state(&self) -> ModalState {
        self.dialog_options.modal_state(self.flags.with_dialog)
    }

    pub fn opens_dialog(&self) -> bool {
        self.dialog_options.opens_dialog(self.flags.with_dialog)
    }
}

/// One action: a named, colored, optionally keyed ordered list of steps.
#[derive(Clone, Debug)]
pub struct Action {
    pub name: String,
    pub function_key: FunctionKey,
    pub color: ActionColor,
    pub expanded: bool,
    pub steps: Vec<ActionStep>,
    /// The imported UTF-16 name units, re-emitted while `name` is unchanged.
    pub raw_name: Option<Vec<u16>>,
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.function_key == other.function_key
            && self.color == other.color
            && self.expanded == other.expanded
            && self.steps == other.steps
    }
}

impl Action {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            function_key: FunctionKey::NONE,
            color: ActionColor::NONE,
            expanded: true,
            steps: Vec::new(),
            raw_name: None,
        }
    }

    /// Conditional steps that name `action_name` in either branch.
    pub fn conditional_references(&self, action_name: &str) -> bool {
        self.steps.iter().any(|step| match &step.kind {
            StepKind::Conditional {
                then_action,
                else_action,
                ..
            } => {
                then_action.as_deref() == Some(action_name)
                    || else_action.as_deref() == Some(action_name)
            }
            _ => false,
        })
    }
}

/// One action set. `trailing` preserves bytes after a single-set `.atn`.
#[derive(Clone, Debug)]
pub struct ActionSet {
    pub name: String,
    pub expanded: bool,
    pub actions: Vec<Action>,
    pub trailing: Vec<u8>,
    /// The imported UTF-16 name units, re-emitted while `name` is unchanged.
    pub raw_name: Option<Vec<u16>>,
}

impl PartialEq for ActionSet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.expanded == other.expanded
            && self.actions == other.actions
            && self.trailing == other.trailing
    }
}

impl ActionSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            expanded: true,
            actions: Vec::new(),
            trailing: Vec::new(),
            raw_name: None,
        }
    }

    pub fn action_index(&self, name: &str) -> Option<usize> {
        self.actions.iter().position(|action| action.name == name)
    }

    pub fn action(&self, name: &str) -> Option<&Action> {
        self.action_index(name).map(|index| &self.actions[index])
    }

    /// Whether any conditional or Play step in the set names `action_name`.
    pub fn references(&self, action_name: &str) -> bool {
        self.actions.iter().any(|action| {
            action.steps.iter().any(|step| match &step.kind {
                StepKind::Conditional {
                    then_action,
                    else_action,
                    ..
                } => {
                    then_action.as_deref() == Some(action_name)
                        || else_action.as_deref() == Some(action_name)
                }
                StepKind::Play { action } => action.as_str() == action_name,
                _ => false,
            })
        })
    }

    /// Whether removing one action called `action_name` would leave a
    /// conditional dangling (another same-named action still satisfies it).
    pub fn removal_breaks_reference(&self, action_index: usize) -> bool {
        let Some(action) = self.actions.get(action_index) else {
            return false;
        };
        if !self.references(&action.name) {
            return false;
        }
        self.actions
            .iter()
            .enumerate()
            .filter(|(index, other)| *index != action_index && other.name == action.name)
            .count()
            == 0
    }

    /// Refuse a conditional whose branches are empty, missing from this set, or
    /// would close a cycle back to `containing`.
    pub fn validate_conditional(
        &self,
        containing: usize,
        then_action: Option<&str>,
        else_action: Option<&str>,
    ) -> Result<(), ActionError> {
        if then_action.is_none() && else_action.is_none() {
            return Err(ActionError::ConditionalEmpty);
        }
        for branch in [then_action, else_action].into_iter().flatten() {
            let Some(index) = self.action_index(branch) else {
                return Err(ActionError::Invalid(format!(
                    "conditional action '{branch}' is not in this set"
                )));
            };
            if self.reaches(index, containing, &mut HashSet::new()) {
                return Err(ActionError::ConditionalCycle(branch.to_string()));
            }
        }
        Ok(())
    }

    fn reaches(&self, from: usize, target: usize, visited: &mut HashSet<usize>) -> bool {
        if from == target {
            return true;
        }
        if !visited.insert(from) {
            return false;
        }
        let Some(action) = self.actions.get(from) else {
            return false;
        };
        for step in &action.steps {
            if let StepKind::Conditional {
                then_action,
                else_action,
                ..
            } = &step.kind
            {
                for name in [then_action, else_action].into_iter().flatten() {
                    if let Some(next) = self.action_index(name) {
                        if self.reaches(next, target, visited) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
