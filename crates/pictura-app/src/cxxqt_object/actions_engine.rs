//! The Actions panel's Qt bridge: one `ActionsEngine` owns the library,
//! recording cursor, playback cursor, and function-key reservations. Every
//! structural change emits `changed`; the panel takes one snapshot per refresh.

mod imp;
mod library_ops;
mod session_ops;
mod snapshot;

pub use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    struct ScriptActionNodeRow {
        id: QString,
        parent_id: QString,
        kind: QString,
        name: QString,
        enabled: bool,
        modal: QString,
        color: i32,
        function_key: QString,
        step_index: i32,
        can_expand: bool,
        dialog_options: i32,
        step_kind: QString,
        command_id: QString,
        message: QString,
        allow_continue: bool,
        condition: QString,
        then_action: QString,
        else_action: QString,
        path: QString,
        params_json: QString,
        function_key_number: i32,
        function_shift: bool,
        function_ctrl: bool,
    }

    #[namespace = "pictura"]
    struct ScriptPlaybackRow {
        kind: QString,
        set: i32,
        action: i32,
        step: i32,
        command: QString,
        params_json: QString,
        modal: bool,
        message: QString,
        allow_continue: bool,
        path: QString,
        error: QString,
    }

    extern "RustQt" {
        #[qobject]
        #[namespace = "pictura"]
        type ActionsEngine = super::ActionsEngineRust;

        /// Emitted after any structural change, recording transition, playback
        /// step, or function-key change.
        #[qsignal]
        fn changed(self: Pin<&mut Self>);

        /// One row per set, action, and step.
        #[qinvokable]
        fn snapshot(&self) -> Vec<ScriptActionNodeRow>;

        #[qinvokable]
        fn last_error(&self) -> QString;

        #[qinvokable]
        fn recording(&self) -> bool;

        #[qinvokable]
        fn playback_active(&self) -> bool;

        #[qinvokable]
        fn allow_tool_recording(&self) -> bool;

        #[qinvokable]
        fn set_allow_tool_recording(self: Pin<&mut Self>, allow: bool);

        #[qinvokable]
        fn create_set(self: Pin<&mut Self>, name: &QString) -> i32;

        #[qinvokable]
        fn rename_set(self: Pin<&mut Self>, set: i32, name: &QString) -> bool;

        #[qinvokable]
        fn delete_set(self: Pin<&mut Self>, set: i32) -> bool;

        #[qinvokable]
        fn duplicate_set(self: Pin<&mut Self>, set: i32) -> i32;

        #[qinvokable]
        fn move_set(self: Pin<&mut Self>, from: i32, to: i32) -> bool;

        #[qinvokable]
        fn create_action(self: Pin<&mut Self>, set: i32, name: &QString) -> i32;

        #[qinvokable]
        fn rename_action(self: Pin<&mut Self>, set: i32, action: i32, name: &QString) -> bool;

        #[qinvokable]
        fn delete_action(self: Pin<&mut Self>, set: i32, action: i32) -> bool;

        #[qinvokable]
        fn duplicate_action(self: Pin<&mut Self>, set: i32, action: i32) -> i32;

        #[qinvokable]
        fn move_action(
            self: Pin<&mut Self>,
            from_set: i32,
            action: i32,
            to_set: i32,
            to_index: i32,
        ) -> bool;

        #[qinvokable]
        fn set_action_color(self: Pin<&mut Self>, set: i32, action: i32, color: i32) -> bool;

        #[qinvokable]
        fn set_set_expanded(self: Pin<&mut Self>, set: i32, expanded: bool) -> bool;

        #[qinvokable]
        fn set_action_expanded(self: Pin<&mut Self>, set: i32, action: i32, expanded: bool)
            -> bool;

        #[qinvokable]
        fn assign_function_key(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            number: i32,
            shift: bool,
            ctrl: bool,
        ) -> bool;

        #[qinvokable]
        fn clear_function_key(self: Pin<&mut Self>, set: i32, action: i32) -> bool;

        #[qinvokable]
        fn action_for_function_key(&self, number: i32, shift: bool, ctrl: bool) -> QString;

        /// Test seam for the Windows F1/Ctrl+F4/Ctrl+F6 rules.
        #[qinvokable]
        fn set_windows_platform(self: Pin<&mut Self>, windows: bool);

        #[qinvokable]
        fn set_step_enabled(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            step: i32,
            enabled: bool,
        ) -> bool;

        #[qinvokable]
        fn set_step_with_dialog(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            step: i32,
            with_dialog: bool,
        ) -> bool;

        #[qinvokable]
        fn edit_step(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            step: i32,
            params_json: &QString,
        ) -> bool;

        #[qinvokable]
        fn conditional_branch(&self, set: i32, action: i32, step: i32) -> QString;

        #[qinvokable]
        fn delete_step(self: Pin<&mut Self>, set: i32, action: i32, step: i32) -> bool;

        #[qinvokable]
        fn move_step(self: Pin<&mut Self>, set: i32, action: i32, from: i32, to: i32) -> bool;

        /// Move one step into another action or set at `to_index` (`-1` appends).
        #[qinvokable]
        fn move_step_to(
            self: Pin<&mut Self>,
            from_set: i32,
            from_action: i32,
            from_step: i32,
            to_set: i32,
            to_action: i32,
            to_index: i32,
        ) -> bool;

        #[qinvokable]
        fn duplicate_step(self: Pin<&mut Self>, set: i32, action: i32, step: i32) -> bool;

        #[qinvokable]
        fn insert_stop(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            index: i32,
            message: &QString,
            allow_continue: bool,
        ) -> i32;

        #[qinvokable]
        fn insert_menu_item(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            index: i32,
            command: &QString,
            params_json: &QString,
        ) -> i32;

        #[qinvokable]
        fn insert_path(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            index: i32,
            path: &QString,
        ) -> i32;

        #[qinvokable]
        fn insert_conditional(
            self: Pin<&mut Self>,
            set: i32,
            action: i32,
            index: i32,
            condition: &QString,
            mode: &QString,
            then_action: &QString,
            else_action: &QString,
        ) -> i32;

        #[qinvokable]
        fn record_start(self: Pin<&mut Self>, scope: i32, set: i32, action: i32, step: i32)
            -> bool;

        #[qinvokable]
        fn record_stop(self: Pin<&mut Self>);

        #[qinvokable]
        fn record_command(self: Pin<&mut Self>, id: &QString, params_json: &QString) -> bool;

        #[qinvokable]
        fn record_tool_stroke(self: Pin<&mut Self>, payload_json: &QString) -> bool;

        #[qinvokable]
        fn play_start(self: Pin<&mut Self>, scope: i32, set: i32, action: i32, step: i32) -> bool;

        #[qinvokable]
        fn play_next(self: Pin<&mut Self>) -> ScriptPlaybackRow;

        #[qinvokable]
        fn play_continue(self: Pin<&mut Self>);

        #[qinvokable]
        fn play_cancel(self: Pin<&mut Self>);

        #[qinvokable]
        fn set_document_facts(
            self: Pin<&mut Self>,
            present: bool,
            square: bool,
            mode: &QString,
            has_selection: bool,
        );

        #[qinvokable]
        fn set_playback_options(self: Pin<&mut Self>, speed: i32, pause_seconds: f64);

        #[qinvokable]
        fn playback_speed(&self) -> i32;

        #[qinvokable]
        fn playback_pause_seconds(&self) -> f64;

        #[qinvokable]
        fn palette_bytes(&self) -> Vec<u8>;

        #[qinvokable]
        fn set_bytes(&self, set: i32) -> Vec<u8>;

        #[qinvokable]
        fn load_palette(self: Pin<&mut Self>, path: &QString) -> bool;

        #[qinvokable]
        fn save_palette(self: Pin<&mut Self>, path: &QString) -> bool;

        #[qinvokable]
        fn load_palette_bytes(
            self: Pin<&mut Self>,
            bytes: &[u8],
            source: &QString,
            replace: bool,
        ) -> bool;

        #[qinvokable]
        fn save_set(self: Pin<&mut Self>, path: &QString, set: i32) -> bool;

        #[qinvokable]
        fn reset_actions(self: Pin<&mut Self>, append: bool);

        #[qinvokable]
        fn clear_actions(self: Pin<&mut Self>);
    }
}
