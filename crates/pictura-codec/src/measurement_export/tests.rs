use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "pictura-measurement-export-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create test directory: {error}"),
            }
        }
    }

    fn table(&self) -> PathBuf {
        self.0.join("measurements.tsv")
    }

    fn histograms(&self) -> PathBuf {
        self.0.join("measurements-histograms")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn measurement_export_writes_ordered_utf8_tsv() {
    use MeasurementCell::{Number, Text};
    let dir = TestDirectory::new();
    let rows: &[&[MeasurementCell<'_>]] = &[
        &[
            Number(2.5),
            Text("50 px = 1 µm"),
            Text("résumé\t\"a\"\r\nb"),
        ],
        &[Number(4.25), Text("1 px = 1 mm"), Text("second")],
    ];
    export_measurements(&dir.table(), &["Length", "Scale", "Label"], rows).unwrap();
    assert_eq!(
        fs::read_to_string(dir.table()).unwrap(),
        "Length\tScale\tLabel\n2.5\t50 px = 1 µm\t\"résumé\t\"\"a\"\"\r\nb\"\n4.25\t1 px = 1 mm\tsecond\n"
    );
}

#[test]
fn measurement_export_empty_selection_has_no_output() {
    let dir = TestDirectory::new();
    export_measurements(&dir.table(), &["Histogram"], &[]).unwrap();
    assert!(!dir.table().exists());
    assert!(!dir.histograms().exists());
    fs::write(dir.table(), "existing").unwrap();
    export_measurements(&dir.table(), &["Histogram"], &[]).unwrap();
    assert_eq!(fs::read_to_string(dir.table()).unwrap(), "existing");
}

#[test]
fn measurement_export_writes_numbered_histograms() {
    use MeasurementCell::{Empty, Histogram, Text};
    let dir = TestDirectory::new();
    let left = [2; 256];
    let right = std::array::from_fn(|bin| bin as u64);
    let summary = [
        HistogramChannel {
            name: "B,\"1\"",
            bins: &left,
        },
        HistogramChannel {
            name: "R",
            bins: &right,
        },
    ];
    let feature = [HistogramChannel {
        name: "R",
        bins: &right,
    }];
    let rows: &[&[MeasurementCell<'_>]] = &[
        &[Text("summary"), Histogram(&summary)],
        &[Text("ruler"), Empty],
        &[Text("feature 1"), Histogram(&feature)],
        &[Text("feature 2"), Histogram(&feature)],
    ];
    export_measurements(&dir.table(), &["Label", "Histogram"], rows).unwrap();
    assert_eq!(
        fs::read_to_string(dir.table()).unwrap(),
        "Label\tHistogram\nsummary\tmeasurements-histograms/0.csv\nruler\t\nfeature 1\tmeasurements-histograms/1.csv\nfeature 2\tmeasurements-histograms/2.csv\n"
    );
    assert_eq!(fs::read_dir(dir.histograms()).unwrap().count(), 3);
    let first = fs::read_to_string(dir.histograms().join("0.csv")).unwrap();
    let lines: Vec<_> = first.lines().collect();
    assert_eq!(lines.len(), 257);
    assert_eq!(lines[0], "Bin,\"B,\"\"1\"\"\",R");
    assert_eq!(lines[1], "0,2,0");
    assert_eq!(lines[256], "255,2,255");
}

#[test]
fn measurement_export_omits_unselected_histograms() {
    let dir = TestDirectory::new();
    let recorded = [
        MeasurementCell::Text("feature"),
        MeasurementCell::Histogram(&[]),
    ];
    export_measurements(&dir.table(), &["Label"], &[&recorded[..1]]).unwrap();
    assert_eq!(fs::read_to_string(dir.table()).unwrap(), "Label\nfeature\n");
    assert!(!dir.histograms().exists());
}

#[test]
fn measurement_export_preserves_destinations_on_failure() {
    let dir = TestDirectory::new();
    let bins = [0; 256];
    let channels = [HistogramChannel {
        name: "Gray",
        bins: &bins,
    }];
    let rows: &[&[MeasurementCell<'_>]] = &[&[MeasurementCell::Histogram(&channels)]];
    let header = &["Histogram"];
    fs::write(dir.table(), "existing table").unwrap();
    assert!(export_measurements(&dir.table(), header, rows).is_err());
    assert_eq!(fs::read_to_string(dir.table()).unwrap(), "existing table");
    fs::remove_file(dir.table()).unwrap();

    fs::create_dir(dir.histograms()).unwrap();
    let original = dir.histograms().join("0.csv");
    fs::write(&original, "existing histogram").unwrap();
    assert!(export_measurements(&dir.table(), header, rows).is_err());
    assert!(!dir.table().exists());
    assert_eq!(fs::read_to_string(original).unwrap(), "existing histogram");
    fs::remove_dir_all(dir.histograms()).unwrap();

    let error = export_with(&dir.table(), header, rows, |writer, _, _, _| {
        writer.write_all(b"partially written")?;
        Err(io::Error::other("injected write failure"))
    })
    .unwrap_err();
    assert!(error.to_string().contains("injected write failure"));
    assert!(!dir.table().exists());
    assert!(!dir.histograms().exists());
}
