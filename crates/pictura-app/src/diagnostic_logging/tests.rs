use super::*;
use std::sync::mpsc;

fn directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pictura-{label}-{}", launch_id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn diagnostic_rotation_keeps_seven_dates() {
    let root = directory("log-rotation");
    let now = 20_000 * 86_400;
    let mut older = DailyFiles::new(root.clone(), "previous".into());
    for days in 1..=8 {
        older.write_at(now - days * 86_400, b"{}\n").unwrap();
    }
    std::fs::write(root.join("notes.txt"), "keep").unwrap();
    let mut first = DailyFiles::new(root.clone(), "first".into());
    let mut second = DailyFiles::new(root.clone(), "second".into());
    first.write_at(now - 1, b"{\"writer\":1}\n").unwrap();
    std::thread::scope(|threads| {
        threads.spawn(|| first.write_at(now, b"{\"writer\":1}\n").unwrap());
        threads.spawn(|| second.write_at(now, b"{\"writer\":2}\n").unwrap());
    });
    let oldest = day_prefix(now - 6 * 86_400);
    let current = day_prefix(now);
    let names: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(names.contains(&"notes.txt".into()));
    assert!(names.iter().any(|name| name.starts_with(&oldest)));
    assert!(names
        .iter()
        .filter(|name| name.ends_with(".jsonl"))
        .all(|name| name[..10] >= oldest[..]));
    for (id, value) in [("first", 1), ("second", 2)] {
        let path = root.join(format!("{current}-pictura-{id}.jsonl"));
        let line = std::fs::read_to_string(path).unwrap();
        let record: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(record["writer"], value);
    }
    drop((first, second, older));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostic_rotation_reopens_its_own_previous_date() {
    let root = directory("log-clock-rollback");
    let midnight = 20_000 * 86_400;
    let mut files = DailyFiles::new(root.clone(), "rollback".into());
    files.write_at(midnight - 1, b"before\n").unwrap();
    files.write_at(midnight, b"after\n").unwrap();
    files.write_at(midnight - 1, b"again\n").unwrap();
    let path = root.join(format!(
        "{}-pictura-rollback.jsonl",
        day_prefix(midnight - 1)
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "before\nagain\n");
    drop(files);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostic_writer_failure_and_saturation() {
    let root = directory("log-writer");
    let blocker = root.join("blocker");
    std::fs::write(&blocker, b"file").unwrap();
    assert!(prepare_directory(&blocker).is_err());

    let (entered, wait_entered) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let sink = FileQueue::spawn(1, move |_record| {
        entered.send(()).unwrap();
        wait_release.recv().unwrap();
        Err(std::io::Error::other("injected failure"))
    })
    .unwrap();
    assert!(sink.enqueue(b"first\n".to_vec()));
    wait_entered.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(sink.enqueue(b"second\n".to_vec()));
    assert!(!sink.enqueue(b"over capacity\n".to_vec()));
    release.send(()).unwrap();
    assert_eq!(sink.take_completion(Duration::from_secs(2)), Some(false));
    assert!(sink.failed());
    assert!(!sink.enqueue(b"after failure\n".to_vec()));
    std::fs::remove_dir_all(root).unwrap();
}
