//! Library, action, step, and function-key operations.

use super::{index, q};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_script::{
    action::{
        ActionColor, ActionStep, Condition, DialogOptions, FunctionKey, Platform, StepFlags,
        StepKind,
    },
    command::{CommandId, CommandParams},
    ActionError,
};

impl super::ffi::ActionsEngine {
    pub fn create_set(mut self: Pin<&mut Self>, name: &QString) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            this.last_error.clear();
            this.library.create_set(name.to_string()) as i32
        };
        self.as_mut().changed();
        index
    }

    pub fn rename_set(mut self: Pin<&mut Self>, set: i32, name: &QString) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            index(set).is_some_and(|set| this.library.rename_set(set, name.to_string()))
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn delete_set(mut self: Pin<&mut Self>, set: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            index(set).is_some_and(|set| this.library.delete_set(set))
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn duplicate_set(mut self: Pin<&mut Self>, set: i32) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            index(set)
                .and_then(|set| this.library.duplicate_set(set))
                .map_or(-1, |index| index as i32)
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }

    pub fn move_set(mut self: Pin<&mut Self>, from: i32, to: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(from), index(to)) {
                (Some(from), Some(to)) => this.library.move_set(from, to),
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn create_action(mut self: Pin<&mut Self>, set: i32, name: &QString) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            let Some(set) = index(set) else {
                return -1;
            };
            match this.library.create_action(set, name.to_string()) {
                Ok(index) => index as i32,
                Err(err) => {
                    this.last_error = err.to_string();
                    -1
                }
            }
        };
        self.as_mut().changed();
        index
    }

    pub fn rename_action(mut self: Pin<&mut Self>, set: i32, action: i32, name: &QString) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => this
                    .library
                    .rename_action(set, action, name.to_string())
                    .unwrap_or(false),
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn delete_action(mut self: Pin<&mut Self>, set: i32, action: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            let Some(set) = index(set) else {
                return false;
            };
            let Some(action) = index(action) else {
                return false;
            };
            match this.library.delete_action(set, action) {
                Ok(changed) => changed,
                Err(err) => {
                    this.last_error = err.to_string();
                    false
                }
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn duplicate_action(mut self: Pin<&mut Self>, set: i32, action: i32) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => this
                    .library
                    .duplicate_action(set, action)
                    .map_or(-1, |index| index as i32),
                _ => -1,
            }
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }

    pub fn move_action(
        mut self: Pin<&mut Self>,
        from_set: i32,
        action: i32,
        to_set: i32,
        to_index: i32,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (
                index(from_set),
                index(action),
                index(to_set),
                index(to_index),
            ) {
                (Some(from_set), Some(action), Some(to_set), Some(to_index)) => {
                    match this.library.move_action(from_set, action, to_set, to_index) {
                        Ok(changed) => changed,
                        Err(err) => {
                            this.last_error = err.to_string();
                            false
                        }
                    }
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn set_action_color(mut self: Pin<&mut Self>, set: i32, action: i32, color: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), u16::try_from(color)) {
                (Some(set), Some(action), Ok(color)) => {
                    this.library
                        .set_action_color(set, action, ActionColor(color))
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn set_set_expanded(mut self: Pin<&mut Self>, set: i32, expanded: bool) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            index(set).is_some_and(|set| this.library.set_set_expanded(set, expanded))
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn set_action_expanded(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        expanded: bool,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => {
                    this.library.set_action_expanded(set, action, expanded)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn assign_function_key(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        number: i32,
        shift: bool,
        ctrl: bool,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            let (Some(set), Some(action), Ok(number)) =
                (index(set), index(action), u16::try_from(number))
            else {
                return false;
            };
            let key = FunctionKey::new(number, shift, ctrl);
            if key.is_assigned() && !key.is_allowed_on(this.platform) {
                this.last_error = ActionError::FunctionKeyUnavailable(key.display()).to_string();
                return false;
            }
            if key.is_assigned() {
                match owner_of(&this.library, key) {
                    Some(owner) if owner != (set, action) => {
                        this.last_error = ActionError::FunctionKeyTaken(key.display()).to_string();
                        return false;
                    }
                    _ => {}
                }
            }
            let Some(target) = this.library.action_mut(set, action) else {
                return false;
            };
            target.function_key = key;
            this.last_error.clear();
            true
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn clear_function_key(mut self: Pin<&mut Self>, set: i32, action: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => match this.library.action_mut(set, action) {
                    Some(target) => {
                        target.function_key = FunctionKey::NONE;
                        true
                    }
                    None => false,
                },
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn action_for_function_key(&self, number: i32, shift: bool, ctrl: bool) -> QString {
        let Ok(number) = u16::try_from(number) else {
            return q("");
        };
        owner_of(&self.rust().library, FunctionKey::new(number, shift, ctrl))
            .and_then(|(set, action)| self.rust().library.action(set, action))
            .map_or(q(""), |action| q(&action.name))
    }

    pub fn set_windows_platform(mut self: Pin<&mut Self>, windows: bool) {
        self.as_mut().rust_mut().get_mut().platform = if windows {
            Platform::Windows
        } else {
            Platform::Linux
        };
    }

    pub fn set_step_enabled(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        step: i32,
        enabled: bool,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), index(step)) {
                (Some(set), Some(action), Some(step)) => {
                    this.library.set_step_enabled(set, action, step, enabled)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn set_step_with_dialog(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        step: i32,
        with_dialog: bool,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), index(step)) {
                (Some(set), Some(action), Some(step)) => {
                    this.library
                        .set_step_with_dialog(set, action, step, with_dialog)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn edit_step(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        step: i32,
        params_json: &QString,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            let params = match CommandParams::from_json(&params_json.to_string()) {
                Ok(params) => params,
                Err(err) => {
                    this.last_error = err.to_string();
                    return false;
                }
            };
            match (index(set), index(action), index(step)) {
                (Some(set), Some(action), Some(step)) => {
                    this.library.set_step_params(set, action, step, params)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn delete_step(mut self: Pin<&mut Self>, set: i32, action: i32, step: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), index(step)) {
                (Some(set), Some(action), Some(step)) => {
                    this.library.delete_step(set, action, step)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn move_step(mut self: Pin<&mut Self>, set: i32, action: i32, from: i32, to: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), index(from), index(to)) {
                (Some(set), Some(action), Some(from), Some(to)) => {
                    this.library.move_step(set, action, from, to)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn duplicate_step(mut self: Pin<&mut Self>, set: i32, action: i32, step: i32) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action), index(step)) {
                (Some(set), Some(action), Some(step)) => {
                    this.library.duplicate_step(set, action, step)
                }
                _ => false,
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn insert_stop(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        at_index: i32,
        message: &QString,
        allow_continue: bool,
    ) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => {
                    let at = insert_index(&this.library, set, action, at_index);
                    let outcome =
                        this.library
                            .insert_stop(set, action, message.to_string(), allow_continue);
                    outcome
                        .map(|inserted| move_inserted(&mut this.library, set, action, inserted, at))
                        .map_or(-1, |index| index as i32)
                }
                _ => -1,
            }
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }

    pub fn insert_menu_item(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        at_index: i32,
        command: &QString,
        params_json: &QString,
    ) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            let params = match CommandParams::from_json(&params_json.to_string()) {
                Ok(params) => params,
                Err(err) => {
                    this.last_error = err.to_string();
                    return -1;
                }
            };
            match (index(set), index(action)) {
                (Some(set), Some(action)) => {
                    let at = insert_index(&this.library, set, action, at_index);
                    let outcome = this.library.insert_menu_item(
                        set,
                        action,
                        CommandId::new(command.to_string()),
                        params,
                    );
                    outcome
                        .map(|inserted| move_inserted(&mut this.library, set, action, inserted, at))
                        .map_or(-1, |index| index as i32)
                }
                _ => -1,
            }
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }

    pub fn insert_path(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        at_index: i32,
        path: &QString,
    ) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            match (index(set), index(action)) {
                (Some(set), Some(action)) => {
                    let at = insert_index(&this.library, set, action, at_index);
                    let steps = this
                        .library
                        .action(set, action)
                        .map_or(0, |action| action.steps.len());
                    if at >= steps {
                        this.library
                            .insert_path(set, action, path.to_string())
                            .map_or(-1, |index| index as i32)
                    } else {
                        let step = ActionStep {
                            flags: StepFlags::default(),
                            dialog_options: DialogOptions::OPTIONAL,
                            display_name: "Insert Path".into(),
                            kind: StepKind::InsertPath {
                                path: path.to_string(),
                            },
                            imported: None,
                        };
                        let outcome = this.library.add_step(set, action, step);
                        outcome
                            .map(|inserted| {
                                move_inserted(&mut this.library, set, action, inserted, at)
                            })
                            .map_or(-1, |index| index as i32)
                    }
                }
                _ => -1,
            }
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_conditional(
        mut self: Pin<&mut Self>,
        set: i32,
        action: i32,
        at_index: i32,
        condition: &QString,
        mode: &QString,
        then_action: &QString,
        else_action: &QString,
    ) -> i32 {
        let index = {
            let this = self.as_mut().rust_mut().get_mut();
            let condition = match condition.to_string().as_str() {
                "mode" => Condition::DocumentMode(mode.to_string()),
                "selection" => Condition::DocumentHasSelection,
                _ => Condition::DocumentIsSquare,
            };
            let then_action = optional_name(then_action);
            let else_action = optional_name(else_action);
            match (index(set), index(action)) {
                (Some(set), Some(action)) => {
                    let at = insert_index(&this.library, set, action, at_index);
                    let outcome = this.library.insert_conditional(
                        set,
                        action,
                        condition,
                        then_action,
                        else_action,
                    );
                    outcome
                        .map(|inserted| move_inserted(&mut this.library, set, action, inserted, at))
                        .map_or_else(
                            |err| {
                                this.last_error = err.to_string();
                                -1
                            },
                            |index| index as i32,
                        )
                }
                _ => -1,
            }
        };
        if index >= 0 {
            self.as_mut().changed();
        }
        index
    }
}

fn move_inserted(
    library: &mut pictura_script::ActionLibrary,
    set: usize,
    action: usize,
    inserted: usize,
    at: usize,
) -> usize {
    let target = at.min(inserted);
    if target < inserted && library.move_step(set, action, inserted, target) {
        target
    } else {
        inserted
    }
}

fn insert_index(
    library: &pictura_script::ActionLibrary,
    set: usize,
    action: usize,
    index: i32,
) -> usize {
    if index < 0 {
        library
            .action(set, action)
            .map_or(0, |action| action.steps.len())
    } else {
        index as usize
    }
}

fn optional_name(value: &QString) -> Option<String> {
    let text = value.to_string();
    (!text.is_empty()).then_some(text)
}

fn owner_of(library: &pictura_script::ActionLibrary, key: FunctionKey) -> Option<(usize, usize)> {
    if !key.is_assigned() {
        return None;
    }
    for (set_index, set) in library.sets().iter().enumerate() {
        for (action_index, action) in set.actions.iter().enumerate() {
            if action.function_key == key {
                return Some((set_index, action_index));
            }
        }
    }
    None
}
