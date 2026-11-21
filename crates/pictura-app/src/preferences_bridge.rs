//! The Qt shell passes resolved profile paths and field-level preferences edits.
mod imp;

use imp::*;

#[cxx_qt::bridge]
mod ffi {
    #[namespace = "pictura"]
    struct PreferencesReply {
        values: String,
        notice: String,
        can_clean_legacy: bool,
        agent_guard: bool,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        type ProfileLease;

        fn prefs_defaults() -> String;
        fn prefs_load(
            root: &str,
            systems: &str,
            legacy: &str,
            defer_legacy_write: bool,
        ) -> PreferencesReply;
        fn prefs_update(
            root: &str,
            systems: &str,
            patch: &str,
            grant: bool,
            require_existing: bool,
        ) -> Result<()>;
        fn prefs_legacy_keys() -> Vec<String>;
        fn prefs_migration_marker() -> String;
        fn prefs_export(root: &str) -> Result<String>;
        fn prefs_validate_portable(text: &str) -> Result<()>;
        fn prefs_apply_import(
            root: &str,
            legacy: &str,
            toml: &str,
            replace: bool,
            id: &str,
            require_existing: bool,
        ) -> Result<()>;
        fn prefs_retire_receipt(root: &str, id: &str) -> Result<()>;
        fn profile_open(config: &str, state: &str) -> Result<Box<ProfileLease>>;
        fn profile_reset(config: &str, state: &str) -> Result<()>;
        fn profile_write(path: &str, bytes: &[u8]) -> Result<()>;
        fn profile_retire(path: &str) -> Result<()>;
    }
}
