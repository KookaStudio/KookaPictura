//! The action library: ordered sets, CRUD, the compiled-in default set, and
//! `.atn`/`.psp` load and replace with a keep-on-failure contract.

use crate::action::{
    Action, ActionColor, ActionSet, ActionStep, Condition, DialogOptions, DocumentFacts, StepFlags,
    StepKind,
};
use crate::atn;
use crate::command::{label_for, CommandId, CommandParams};
use crate::error::ActionError;
use std::path::Path;

/// How Reset Actions applies the compiled-in default set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetMode {
    Replace,
    Append,
}

/// The in-memory action library.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionLibrary {
    sets: Vec<ActionSet>,
}

impl ActionLibrary {
    pub fn empty() -> Self {
        Self { sets: Vec::new() }
    }

    pub fn with_default() -> Self {
        Self {
            sets: vec![Self::default_set()],
        }
    }

    pub fn from_sets(sets: Vec<ActionSet>) -> Self {
        Self { sets }
    }

    /// The project-authored reset set: four actions, each with its own color.
    pub fn default_set() -> ActionSet {
        let mut set = ActionSet::new("Default Actions");
        set.actions = vec![
            default_action(
                "Duplicate Layer",
                ActionColor::RED,
                ActionStep::command("layer.duplicate.layer", CommandParams::new()),
            ),
            default_action(
                "Flatten Image",
                ActionColor::ORANGE,
                ActionStep::command("layer.flatten.image", CommandParams::new()),
            ),
            default_action(
                "Grayscale",
                ActionColor::YELLOW,
                ActionStep::command(
                    "image.mode.grayscale",
                    CommandParams::new().with("mode", "grayscale"),
                ),
            ),
            default_action(
                "Resize to 1000 px",
                ActionColor::GREEN,
                ActionStep::command(
                    "image.image.size",
                    CommandParams::new()
                        .with("width", 1000)
                        .with("constrainProportions", true)
                        .with("resample", "bicubic"),
                ),
            ),
        ];
        set
    }

    pub fn sets(&self) -> &[ActionSet] {
        &self.sets
    }

    pub fn sets_mut(&mut self) -> &mut [ActionSet] {
        &mut self.sets
    }

    pub fn set(&self, set: usize) -> Option<&ActionSet> {
        self.sets.get(set)
    }

    pub fn set_mut(&mut self, set: usize) -> Option<&mut ActionSet> {
        self.sets.get_mut(set)
    }

    pub fn action(&self, set: usize, action: usize) -> Option<&Action> {
        self.sets.get(set)?.actions.get(action)
    }

    pub fn action_mut(&mut self, set: usize, action: usize) -> Option<&mut Action> {
        self.sets.get_mut(set)?.actions.get_mut(action)
    }

    pub fn create_set(&mut self, name: impl Into<String>) -> usize {
        self.sets.push(ActionSet::new(name));
        self.sets.len() - 1
    }

    pub fn rename_set(&mut self, set: usize, name: impl Into<String>) -> bool {
        let Some(set) = self.sets.get_mut(set) else {
            return false;
        };
        set.name = name.into();
        true
    }

    pub fn delete_set(&mut self, set: usize) -> bool {
        if set >= self.sets.len() {
            return false;
        }
        self.sets.remove(set);
        true
    }

    pub fn duplicate_set(&mut self, set: usize) -> Option<usize> {
        let source = self.sets.get(set)?.clone();
        let mut copy = source;
        copy.name = unique_name(&copy.name, self.sets.iter().map(|set| set.name.as_str()));
        copy.raw_name = None;
        for action in &mut copy.actions {
            action.function_key = crate::action::FunctionKey::NONE;
        }
        self.sets.insert(set + 1, copy);
        Some(set + 1)
    }

    pub fn move_set(&mut self, from: usize, to: usize) -> bool {
        if from >= self.sets.len() || to > self.sets.len() {
            return false;
        }
        if from == to {
            return true;
        }
        let set = self.sets.remove(from);
        let index = to.min(self.sets.len());
        self.sets.insert(index, set);
        true
    }

    pub fn create_action(
        &mut self,
        set: usize,
        name: impl Into<String>,
    ) -> Result<usize, ActionError> {
        let set = self.sets.get_mut(set).ok_or(ActionError::NoTarget)?;
        let name = name.into();
        if set.actions.iter().any(|action| action.name == name) {
            return Err(ActionError::DuplicateActionName { name });
        }
        set.actions.push(Action::new(name));
        Ok(set.actions.len() - 1)
    }

    pub fn rename_action(
        &mut self,
        set: usize,
        action: usize,
        name: impl Into<String>,
    ) -> Result<bool, ActionError> {
        let new_name = name.into();
        let Some(set) = self.sets.get_mut(set) else {
            return Ok(false);
        };
        let Some(target) = set.actions.get(action) else {
            return Ok(false);
        };
        let old_name = target.name.clone();
        if old_name == new_name {
            return Ok(true);
        }
        if set
            .actions
            .iter()
            .enumerate()
            .any(|(index, other)| index != action && other.name == new_name)
        {
            return Err(ActionError::DuplicateActionName { name: new_name });
        }
        for other in &mut set.actions {
            for step in &mut other.steps {
                match &mut step.kind {
                    StepKind::Conditional {
                        then_action,
                        else_action,
                        ..
                    } => {
                        if then_action.as_deref() == Some(old_name.as_str()) {
                            *then_action = Some(new_name.clone());
                        }
                        if else_action.as_deref() == Some(old_name.as_str()) {
                            *else_action = Some(new_name.clone());
                        }
                    }
                    StepKind::Play { action } if *action == old_name => {
                        *action = new_name.clone();
                    }
                    _ => {}
                }
            }
        }
        set.actions[action].name = new_name;
        Ok(true)
    }

    pub fn delete_action(&mut self, set: usize, action: usize) -> Result<bool, ActionError> {
        let Some(target) = self.sets.get(set) else {
            return Ok(false);
        };
        if action >= target.actions.len() {
            return Ok(false);
        }
        if target.removal_breaks_reference(action) {
            return Err(ActionError::ActionReferenced {
                action: target.actions[action].name.clone(),
            });
        }
        self.sets[set].actions.remove(action);
        Ok(true)
    }

    pub fn duplicate_action(&mut self, set: usize, action: usize) -> Option<usize> {
        let target = self.sets.get(set)?;
        let source = target.actions.get(action)?.clone();
        let mut copy = source;
        copy.name = unique_name(
            &copy.name,
            target.actions.iter().map(|action| action.name.as_str()),
        );
        copy.raw_name = None;
        copy.function_key = crate::action::FunctionKey::NONE;
        self.sets[set].actions.insert(action + 1, copy);
        Some(action + 1)
    }

    /// Move an action, refusing when leaving its set would dangle or break a
    /// conditional reference.
    pub fn move_action(
        &mut self,
        from_set: usize,
        action: usize,
        to_set: usize,
        to_index: usize,
    ) -> Result<bool, ActionError> {
        if from_set == to_set {
            let Some(set) = self.sets.get_mut(from_set) else {
                return Ok(false);
            };
            if action >= set.actions.len() || to_index > set.actions.len() {
                return Ok(false);
            }
            if action == to_index {
                return Ok(true);
            }
            let moved = set.actions.remove(action);
            let index = to_index.min(set.actions.len());
            set.actions.insert(index, moved);
            return Ok(true);
        }
        let Some(source) = self.sets.get(from_set) else {
            return Ok(false);
        };
        if action >= source.actions.len() {
            return Ok(false);
        }
        if source.removal_breaks_reference(action) {
            return Err(ActionError::ActionReferenced {
                action: source.actions[action].name.clone(),
            });
        }
        let Some(destination) = self.sets.get(to_set) else {
            return Ok(false);
        };
        if destination
            .actions
            .iter()
            .any(|existing| existing.name == source.actions[action].name)
        {
            return Err(ActionError::DuplicateActionName {
                name: source.actions[action].name.clone(),
            });
        }
        for step in &source.actions[action].steps {
            match &step.kind {
                StepKind::Conditional {
                    then_action,
                    else_action,
                    ..
                } => {
                    for branch in [then_action, else_action].into_iter().flatten() {
                        if destination.action_index(branch).is_none() {
                            return Err(ActionError::ConditionalCrossSet);
                        }
                    }
                }
                StepKind::Play { action } if destination.action_index(action).is_none() => {
                    return Err(ActionError::ConditionalCrossSet);
                }
                _ => {}
            }
        }
        let moved = self.sets[from_set].actions.remove(action);
        let destination = &mut self.sets[to_set];
        let index = to_index.min(destination.actions.len());
        destination.actions.insert(index, moved);
        Ok(true)
    }

    pub fn set_action_color(&mut self, set: usize, action: usize, color: ActionColor) -> bool {
        match self.action_mut(set, action) {
            Some(target) => {
                target.color = color;
                true
            }
            None => false,
        }
    }

    pub fn set_set_expanded(&mut self, set: usize, expanded: bool) -> bool {
        match self.sets.get_mut(set) {
            Some(target) => {
                target.expanded = expanded;
                true
            }
            None => false,
        }
    }

    pub fn set_action_expanded(&mut self, set: usize, action: usize, expanded: bool) -> bool {
        match self.action_mut(set, action) {
            Some(target) => {
                target.expanded = expanded;
                true
            }
            None => false,
        }
    }

    pub fn add_step(
        &mut self,
        set: usize,
        action: usize,
        step: ActionStep,
    ) -> Result<usize, ActionError> {
        let target = self.action_mut(set, action).ok_or(ActionError::NoTarget)?;
        target.steps.push(step);
        Ok(target.steps.len() - 1)
    }

    pub fn delete_step(&mut self, set: usize, action: usize, step: usize) -> bool {
        let Some(target) = self.action_mut(set, action) else {
            return false;
        };
        if step >= target.steps.len() {
            return false;
        }
        target.steps.remove(step);
        true
    }

    pub fn move_step(&mut self, set: usize, action: usize, from: usize, to: usize) -> bool {
        let Some(target) = self.action_mut(set, action) else {
            return false;
        };
        if from >= target.steps.len() || to > target.steps.len() {
            return false;
        }
        if from == to {
            return true;
        }
        let step = target.steps.remove(from);
        let index = to.min(target.steps.len());
        target.steps.insert(index, step);
        true
    }

    /// Move one step to `to_index` in another action or set. `to_index` is an
    /// insertion index; `to_index == destination.steps.len()` appends, and an
    /// index past the end is clamped after removal. Returns `false` for a bad
    /// step index and `Err(NoTarget)` when either action is missing.
    pub fn move_step_to(
        &mut self,
        from_set: usize,
        from_action: usize,
        from_step: usize,
        to_set: usize,
        to_action: usize,
        to_index: usize,
    ) -> Result<bool, ActionError> {
        let source_len = self
            .action(from_set, from_action)
            .ok_or(ActionError::NoTarget)?
            .steps
            .len();
        let destination_len = self
            .action(to_set, to_action)
            .ok_or(ActionError::NoTarget)?
            .steps
            .len();
        if from_step >= source_len || to_index > destination_len {
            return Ok(false);
        }
        if from_set == to_set && from_action == to_action {
            if from_step == to_index {
                return Ok(true);
            }
            let action = self
                .action_mut(from_set, from_action)
                .expect("validated source action");
            let step = action.steps.remove(from_step);
            let index = to_index.min(action.steps.len());
            action.steps.insert(index, step);
            return Ok(true);
        }
        if from_set != to_set {
            match &self.sets[from_set].actions[from_action].steps[from_step].kind {
                StepKind::Conditional {
                    then_action,
                    else_action,
                    ..
                } => {
                    for branch in [then_action, else_action].into_iter().flatten() {
                        if self.sets[to_set].action_index(branch).is_none() {
                            return Err(ActionError::ConditionalCrossSet);
                        }
                    }
                }
                StepKind::Play { action } if self.sets[to_set].action_index(action).is_none() => {
                    return Err(ActionError::ConditionalCrossSet);
                }
                _ => {}
            }
        }
        let step = self.sets[from_set].actions[from_action]
            .steps
            .remove(from_step);
        let destination = &mut self.sets[to_set].actions[to_action];
        let index = to_index.min(destination.steps.len());
        destination.steps.insert(index, step);
        Ok(true)
    }

    pub fn duplicate_step(&mut self, set: usize, action: usize, step: usize) -> bool {
        let Some(target) = self.action_mut(set, action) else {
            return false;
        };
        let Some(source) = target.steps.get(step).cloned() else {
            return false;
        };
        target.steps.insert(step + 1, source);
        true
    }

    pub fn set_step_enabled(
        &mut self,
        set: usize,
        action: usize,
        step: usize,
        enabled: bool,
    ) -> bool {
        match self.step_mut(set, action, step) {
            Some(target) => {
                target.flags.enabled = enabled;
                true
            }
            None => false,
        }
    }

    pub fn set_step_params(
        &mut self,
        set: usize,
        action: usize,
        step: usize,
        params: CommandParams,
    ) -> bool {
        match self.step_mut(set, action, step) {
            Some(target) => {
                let edited = match &mut target.kind {
                    StepKind::Command { params: value, .. }
                    | StepKind::MenuItem { params: value, .. } => {
                        *value = params;
                        true
                    }
                    _ => false,
                };
                if edited {
                    target.imported = None;
                }
                edited
            }
            None => false,
        }
    }

    /// Evaluate a conditional step against the supplied document facts and
    /// return the chosen action name (or none when that branch is empty).
    pub fn conditional_branch(
        &self,
        set: usize,
        action: usize,
        step: usize,
        facts: &DocumentFacts,
    ) -> Option<String> {
        let step = self.sets.get(set)?.actions.get(action)?.steps.get(step)?;
        let StepKind::Conditional {
            condition,
            then_action,
            else_action,
        } = &step.kind
        else {
            return None;
        };
        if !facts.present {
            return None;
        }
        if condition.evaluate(facts) {
            then_action.clone()
        } else {
            else_action.clone()
        }
    }

    pub fn set_step_with_dialog(
        &mut self,
        set: usize,
        action: usize,
        step: usize,
        with_dialog: bool,
    ) -> bool {
        match self.step_mut(set, action, step) {
            Some(target) => {
                if !target.dialog_options.modal_editable() {
                    return false;
                }
                target.flags.with_dialog = with_dialog;
                true
            }
            None => false,
        }
    }

    pub fn insert_stop(
        &mut self,
        set: usize,
        action: usize,
        message: impl Into<String>,
        allow_continue: bool,
    ) -> Result<usize, ActionError> {
        let step = ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Stop".into(),
            kind: StepKind::Stop {
                message: message.into(),
                allow_continue,
            },
            imported: None,
        };
        self.add_step(set, action, step)
    }

    pub fn insert_menu_item(
        &mut self,
        set: usize,
        action: usize,
        command: impl Into<CommandId>,
        params: CommandParams,
    ) -> Result<usize, ActionError> {
        let command = command.into();
        let step = ActionStep {
            flags: StepFlags {
                with_dialog: false,
                ..StepFlags::default()
            },
            dialog_options: DialogOptions::SPECIFIC,
            display_name: format!("Insert Menu Item: {}", label_for(command.as_str())),
            kind: StepKind::MenuItem { command, params },
            imported: None,
        };
        self.add_step(set, action, step)
    }

    /// Insert the work path. A later Insert Path replaces the earlier one in
    /// the same action unless a Save Path step sits between them.
    pub fn insert_path(
        &mut self,
        set: usize,
        action: usize,
        path: impl Into<String>,
    ) -> Result<usize, ActionError> {
        let path = path.into();
        let replaced = {
            let Some(target) = self.action_mut(set, action) else {
                return Err(ActionError::NoTarget);
            };
            let mut replaced = None;
            for (index, step) in target.steps.iter_mut().enumerate().rev() {
                if let StepKind::InsertPath { path: existing } = &mut step.kind {
                    *existing = path.clone();
                    step.imported = None;
                    replaced = Some(index);
                    break;
                }
                if step.display_name.to_ascii_lowercase().contains("save path") {
                    break;
                }
            }
            replaced
        };
        if let Some(index) = replaced {
            return Ok(index);
        }
        let step = ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Insert Path".into(),
            kind: StepKind::InsertPath { path },
            imported: None,
        };
        self.add_step(set, action, step)
    }

    pub fn insert_conditional(
        &mut self,
        set: usize,
        action: usize,
        condition: Condition,
        then_action: Option<String>,
        else_action: Option<String>,
    ) -> Result<usize, ActionError> {
        let Some(target_set) = self.sets.get(set) else {
            return Err(ActionError::NoTarget);
        };
        target_set.validate_conditional(action, then_action.as_deref(), else_action.as_deref())?;
        let step = ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Conditional".into(),
            kind: StepKind::Conditional {
                condition,
                then_action,
                else_action,
            },
            imported: None,
        };
        self.add_step(set, action, step)
    }

    fn step_mut(&mut self, set: usize, action: usize, step: usize) -> Option<&mut ActionStep> {
        self.action_mut(set, action)?.steps.get_mut(step)
    }

    /// The palette bytes for the whole library.
    pub fn save_bytes(&self) -> Vec<u8> {
        atn::write_psp(&self.sets)
    }

    /// One set as `.atn` bytes.
    pub fn set_bytes(&self, set: usize) -> Option<Vec<u8>> {
        self.sets.get(set).map(atn::write_atn)
    }

    /// Load a palette or action set, replacing the library only on success.
    pub fn load_bytes(&mut self, source: &str, bytes: &[u8]) -> Result<(), ActionError> {
        let sets = parse_sets(source, bytes)?;
        self.sets = sets;
        Ok(())
    }

    /// Load what a partially malformed palette decoded before the error. A
    /// wholly malformed file leaves the library unchanged.
    pub fn load_bytes_partial(
        &mut self,
        source: &str,
        bytes: &[u8],
    ) -> (usize, Option<ActionError>) {
        if source.to_ascii_lowercase().ends_with(".atn") {
            return match atn::read_atn(bytes) {
                Ok(set) => {
                    self.sets = vec![set];
                    (1, None)
                }
                Err(error) => (
                    0,
                    Some(ActionError::Load {
                        source_name: source.to_string(),
                        offset: error_offset(&error),
                        message: error.to_string(),
                    }),
                ),
            };
        }
        let (sets, error) = atn::read_psp_partial(bytes);
        if let (true, Some(error)) = (sets.is_empty(), error.as_ref()) {
            return (
                0,
                Some(ActionError::Load {
                    source_name: source.to_string(),
                    offset: error_offset(error),
                    message: error.to_string(),
                }),
            );
        }
        let count = sets.len();
        self.sets = sets;
        (
            count,
            error.map(|error| ActionError::Load {
                source_name: source.to_string(),
                offset: error_offset(&error),
                message: error.to_string(),
            }),
        )
    }

    /// Replace the library (Replace Actions).
    pub fn replace_bytes(&mut self, source: &str, bytes: &[u8]) -> Result<(), ActionError> {
        self.load_bytes(source, bytes)
    }

    /// Append the sets of a palette or action file, keeping existing sets.
    pub fn append_bytes(&mut self, source: &str, bytes: &[u8]) -> Result<(), ActionError> {
        let loaded = parse_sets(source, bytes)?;
        for mut set in loaded {
            set.name = unique_name(&set.name, self.sets.iter().map(|item| item.name.as_str()));
            self.sets.push(set);
        }
        Ok(())
    }

    pub fn save_path(&self, path: impl AsRef<Path>) -> Result<(), ActionError> {
        let path = path.as_ref();
        std::fs::write(path, self.save_bytes()).map_err(|err| ActionError::Save {
            source_name: path.display().to_string(),
            message: err.to_string(),
        })
    }

    pub fn load_path(&mut self, path: impl AsRef<Path>) -> Result<(), ActionError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|err| ActionError::Load {
            source_name: path.display().to_string(),
            offset: None,
            message: err.to_string(),
        })?;
        self.load_bytes(&path.display().to_string(), &bytes)
    }

    /// Reset Actions: replace with the default set, or append a uniquely named
    /// copy when one already exists.
    pub fn reset(&mut self, mode: ResetMode) {
        match mode {
            ResetMode::Replace => self.sets = vec![Self::default_set()],
            ResetMode::Append => {
                let mut set = Self::default_set();
                set.name = unique_name(&set.name, self.sets.iter().map(|item| item.name.as_str()));
                self.sets.push(set);
            }
        }
    }

    pub fn clear(&mut self) {
        self.sets.clear();
    }
}

fn default_action(name: &str, color: ActionColor, step: ActionStep) -> Action {
    Action {
        name: name.into(),
        function_key: crate::action::FunctionKey::NONE,
        color,
        expanded: true,
        steps: vec![step],
        raw_name: None,
    }
}

fn error_offset(error: &ActionError) -> Option<usize> {
    let text = error.to_string();
    let (_, tail) = text.rsplit_once("(offset ")?;
    tail.trim_end_matches(')').parse().ok()
}

fn parse_sets(source: &str, bytes: &[u8]) -> Result<Vec<ActionSet>, ActionError> {
    let prefer_atn = source.to_ascii_lowercase().ends_with(".atn");
    if prefer_atn {
        match atn::read_atn(bytes) {
            Ok(set) => return Ok(vec![set]),
            Err(atn_error) => {
                return match atn::read_psp(bytes) {
                    Ok(sets) => Ok(sets),
                    Err(_) => Err(ActionError::Load {
                        source_name: source.to_string(),
                        offset: error_offset(&atn_error),
                        message: atn_error.to_string(),
                    }),
                };
            }
        }
    }
    match atn::read_psp(bytes) {
        Ok(sets) => Ok(sets),
        Err(psp_error) => match atn::read_atn(bytes) {
            Ok(set) => Ok(vec![set]),
            Err(_) => Err(ActionError::Load {
                source_name: source.to_string(),
                offset: error_offset(&psp_error),
                message: psp_error.to_string(),
            }),
        },
    }
}

fn unique_name<'a>(base: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = existing.collect();
    if !names.contains(&base) {
        return base.to_string();
    }
    for suffix in 2.. {
        let candidate = format!("{base} {suffix}");
        if !names.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!()
}
