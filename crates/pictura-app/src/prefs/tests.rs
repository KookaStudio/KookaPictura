use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    store: Store,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "pictura-prefs-test-{}-{}-{}",
            std::process::id(),
            files::timestamp(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).unwrap();
        Self {
            store: Store::new(root.clone(), vec![root.join("system.toml")]),
            root,
        }
    }

    fn write(&self, text: &str) {
        fs::write(self.store.path(), text).unwrap();
    }

    fn load(&self) -> Load {
        self.store.load(&Map::new(), false)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fields(value: &serde_json::Value) -> &Map<String, serde_json::Value> {
    value.as_object().unwrap()
}

#[test]
fn migration_guard_checks_under_preference_lock() {
    let fixture = Fixture::new();
    let patch = fields(&json!({"gpuCompute": false})).clone();
    assert!(fixture.store.update_checked(&patch, false, true).is_err());
    assert!(!fixture.store.path().exists());
    assert!(fixture
        .store
        .apply_import_checked(&patch, None, "pending", true)
        .is_err());
    fixture.write("schema_version = 1\n[interface]\nbrightnessLevel = 3\n");
    fixture.store.update_checked(&patch, false, true).unwrap();
    assert_eq!(fixture.load().values["brightnessLevel"], 3);
    assert_eq!(fixture.load().values["gpuCompute"], false);
}

#[test]
fn preference_layers_follow_precedence() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("system.toml"),
        "schema_version = 1\n[interface]\nbrightnessLevel = 3\nuiScalePercent = 120\n",
    )
    .unwrap();
    fixture.write("schema_version = 1\n[interface]\nbrightnessLevel = 2\n");
    let result = fixture.load();
    assert_eq!(result.values["brightnessLevel"], 2);
    assert_eq!(result.values["uiScalePercent"], 120);
    assert_eq!(result.values["memoryUsagePercent"], 60);
}

#[test]
fn preference_edits_merge_without_losing_unknown_keys() {
    let fixture = Fixture::new();
    fixture.write("schema_version = 1\nnew_date = 2026-11-21\n[interface]\nunknown = 22\n");
    let first = fixture.store.clone();
    let second = fixture.store.clone();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            barrier.wait();
            first
                .update(fields(&json!({"brightnessLevel": 3})), false)
                .unwrap();
        });
        scope.spawn(|| {
            barrier.wait();
            second
                .update(fields(&json!({"gpuCompute": false})), false)
                .unwrap();
        });
    });
    let stored = fs::read_to_string(fixture.store.path()).unwrap();
    let table = stored.parse::<toml::Table>().unwrap();
    assert_eq!(table["interface"]["unknown"].as_integer(), Some(22));
    assert!(matches!(table["new_date"], toml::Value::Datetime(_)));
    assert_eq!(fixture.load().values["brightnessLevel"], 3);
    assert_eq!(fixture.load().values["gpuCompute"], false);
}

#[test]
fn unchanged_preferences_do_not_rewrite_files() {
    let fixture = Fixture::new();
    let original = "schema_version = 1\n# hand-edited\n[performance]\ngpuCompute = false\n";
    fixture.write(original);
    fs::write(
        fixture.store.backup_path(),
        "schema_version = 1\n# backup\n",
    )
    .unwrap();
    let before = fs::read(fixture.store.backup_path()).unwrap();
    let _ = fixture.load();
    fixture
        .store
        .update(fields(&json!({"gpuCompute": false})), false)
        .unwrap();
    assert_eq!(fs::read_to_string(fixture.store.path()).unwrap(), original);
    assert_eq!(fs::read(fixture.store.backup_path()).unwrap(), before);
}

#[test]
fn failed_preference_write_keeps_last_complete_file() {
    let fixture = Fixture::new();
    let original = "schema_version = 1\n[performance]\ngpuCompute = false\n";
    fixture.write(original);
    fs::create_dir(fixture.store.backup_path()).unwrap();
    assert!(fixture
        .store
        .update(fields(&json!({"gpuCompute": true})), false)
        .is_err());
    assert_eq!(fs::read_to_string(fixture.store.path()).unwrap(), original);
}

#[test]
fn future_preferences_remain_read_only() {
    let fixture = Fixture::new();
    let future = "schema_version = 999\n[performance]\ngpuCompute = false\nmemoryUsagePercent = 'new representation'\n";
    fixture.write(future);
    let result = fixture.load();
    assert!(result.notice.is_some());
    assert_eq!(result.values["gpuCompute"], false);
    assert!(fixture
        .store
        .update(fields(&json!({"gpuCompute": true})), false)
        .is_err());
    assert_eq!(fs::read_to_string(fixture.store.path()).unwrap(), future);
}

#[test]
fn blocked_quarantine_preserves_original() {
    let fixture = Fixture::new();
    let invalid = "schema_version = 1\n[bad";
    fixture.write(invalid);
    fs::write(
        fixture.store.backup_path(),
        "schema_version = 1\n[interface]\nbrightnessLevel = 3\n",
    )
    .unwrap();
    fs::create_dir(fixture.root.join("prefs.lock")).unwrap();
    let result = fixture.load();
    assert!(result.notice.is_some());
    assert_eq!(result.values["brightnessLevel"], 3);
    assert_eq!(fs::read_to_string(fixture.store.path()).unwrap(), invalid);
}

#[test]
fn gpu_preference_round_trips_through_toml() {
    let fixture = Fixture::new();
    assert_eq!(fixture.load().values["gpuCompute"], true);
    fixture
        .store
        .update(fields(&json!({"gpuCompute": false})), false)
        .unwrap();
    assert_eq!(fixture.load().values["gpuCompute"], false);
}
