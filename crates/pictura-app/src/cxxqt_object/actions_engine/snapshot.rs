//! Snapshot rows for the panel tree.

use super::ffi::ScriptActionNodeRow;
use super::q;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_script::StepKind;

impl super::ffi::ActionsEngine {
    pub fn snapshot(&self) -> Vec<ScriptActionNodeRow> {
        let rust = self.rust();
        let mut rows = Vec::new();
        for (set_index, set) in rust.library.sets().iter().enumerate() {
            let set_id = format!("s{set_index}");
            rows.push(ScriptActionNodeRow {
                id: q(&set_id),
                parent_id: q(""),
                kind: q("set"),
                name: q(&set.name),
                enabled: set.actions.iter().all(action_enabled),
                modal: q(modal_summary(
                    set.actions.iter().flat_map(|action| action.steps.iter()),
                )),
                color: 0,
                function_key: q(""),
                step_index: -1,
                can_expand: !set.actions.is_empty(),
                dialog_options: 0,
                step_kind: q(""),
                command_id: q(""),
                message: q(""),
                allow_continue: false,
                condition: q(""),
                then_action: q(""),
                else_action: q(""),
                path: q(""),
                params_json: q(""),
                function_key_number: 0,
                function_shift: false,
                function_ctrl: false,
            });
            for (action_index, action) in set.actions.iter().enumerate() {
                let action_id = format!("{set_id}a{action_index}");
                rows.push(ScriptActionNodeRow {
                    id: q(&action_id),
                    parent_id: q(&set_id),
                    kind: q("action"),
                    name: q(&action.name),
                    enabled: action_enabled(action),
                    modal: q(modal_summary(action.steps.iter())),
                    color: i32::from(action.color.0),
                    function_key: q(&action.function_key.display()),
                    step_index: -1,
                    can_expand: !action.steps.is_empty(),
                    dialog_options: 0,
                    step_kind: q(""),
                    command_id: q(""),
                    message: q(""),
                    allow_continue: false,
                    condition: q(""),
                    then_action: q(""),
                    else_action: q(""),
                    path: q(""),
                    params_json: q(""),
                    function_key_number: i32::from(action.function_key.number),
                    function_shift: action.function_key.shift,
                    function_ctrl: action.function_key.ctrl,
                });
                for (step_index, step) in action.steps.iter().enumerate() {
                    rows.push(ScriptActionNodeRow {
                        id: q(&format!("{action_id}t{step_index}")),
                        parent_id: q(&action_id),
                        kind: q("step"),
                        name: q(&step.display_name),
                        enabled: step.flags.enabled,
                        modal: q(step.modal_state().as_str()),
                        color: 0,
                        function_key: q(""),
                        step_index: step_index as i32,
                        can_expand: false,
                        dialog_options: i32::from(step.dialog_options.raw()),
                        step_kind: q(step_kind(step)),
                        command_id: q(&step_command(step)),
                        message: q(&step_message(step)),
                        allow_continue: matches!(
                            &step.kind,
                            StepKind::Stop {
                                allow_continue: true,
                                ..
                            }
                        ),
                        condition: q(&step_condition(step)),
                        then_action: q(&branch_node(set_index, set, step, true)),
                        else_action: q(&branch_node(set_index, set, step, false)),
                        path: q(&step_path(step)),
                        params_json: q(&step_params(step)),
                        function_key_number: 0,
                        function_shift: false,
                        function_ctrl: false,
                    });
                }
            }
        }
        rows
    }

    pub fn last_error(&self) -> QString {
        q(&self.rust().last_error)
    }

    pub fn recording(&self) -> bool {
        self.rust().recorder.is_recording()
    }

    pub fn playback_active(&self) -> bool {
        self.rust()
            .player
            .as_ref()
            .is_some_and(|player| !player.is_finished())
    }

    pub fn allow_tool_recording(&self) -> bool {
        self.rust().allow_tool_recording
    }
}

fn step_kind(step: &pictura_script::ActionStep) -> &'static str {
    match &step.kind {
        StepKind::Command { .. } => "command",
        StepKind::Stop { .. } => "stop",
        StepKind::MenuItem { .. } => "menuItem",
        StepKind::InsertPath { .. } => "path",
        StepKind::Play { .. } => "play",
        StepKind::Conditional { .. } => "conditional",
        StepKind::ToolStroke { .. } => "toolStroke",
        StepKind::Unknown { .. } => "unknown",
    }
}

fn step_command(step: &pictura_script::ActionStep) -> String {
    match &step.kind {
        StepKind::Command { command, .. } | StepKind::MenuItem { command, .. } => {
            command.to_string()
        }
        _ => String::new(),
    }
}

fn step_message(step: &pictura_script::ActionStep) -> String {
    match &step.kind {
        StepKind::Stop { message, .. } => message.clone(),
        StepKind::Unknown { event, .. } => event.display(),
        _ => String::new(),
    }
}

fn step_condition(step: &pictura_script::ActionStep) -> String {
    match &step.kind {
        StepKind::Conditional { condition, .. } => match condition {
            pictura_script::Condition::DocumentIsSquare => "square".into(),
            pictura_script::Condition::DocumentMode(_) => "mode".into(),
            pictura_script::Condition::DocumentHasSelection => "selection".into(),
        },
        _ => String::new(),
    }
}

fn branch_node(
    set_index: usize,
    set: &pictura_script::ActionSet,
    step: &pictura_script::ActionStep,
    then: bool,
) -> String {
    let StepKind::Conditional {
        then_action,
        else_action,
        ..
    } = &step.kind
    else {
        return String::new();
    };
    let name = if then { then_action } else { else_action };
    name.as_deref()
        .and_then(|name| set.action_index(name))
        .map_or_else(String::new, |index| format!("s{set_index}a{index}"))
}

fn step_path(step: &pictura_script::ActionStep) -> String {
    match &step.kind {
        StepKind::InsertPath { path } => path.clone(),
        _ => String::new(),
    }
}

fn step_params(step: &pictura_script::ActionStep) -> String {
    match &step.kind {
        StepKind::Command { params, .. } | StepKind::MenuItem { params, .. } => params.to_json(),
        StepKind::ToolStroke { stroke } => {
            // The tree shows settings only; the full sample array stays in the
            // step and travels through play_next.
            let mut params = pictura_script::CommandParams::new();
            params.set("version", stroke.version);
            params.set("tool", stroke.tool.clone());
            params.set("settings", stroke.settings.clone());
            params.to_json()
        }
        _ => String::new(),
    }
}

fn action_enabled(action: &pictura_script::Action) -> bool {
    action.steps.iter().all(|step| step.flags.enabled)
}

fn modal_summary<'a>(
    mut steps: impl Iterator<Item = &'a pictura_script::ActionStep>,
) -> &'static str {
    let Some(first) = steps.next() else {
        return "";
    };
    let first = first.opens_dialog();
    if steps.all(|step| step.opens_dialog() == first) {
        if first {
            "on"
        } else {
            "off"
        }
    } else {
        "partial"
    }
}
