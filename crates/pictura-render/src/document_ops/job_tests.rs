//! Progress and cancellation of whole-document engines run as jobs (#1278).

use super::*;
use pictura_core::job::{self, Cancelled, JobProgress};
use pictura_core::{Document, LayerStack};

/// Three full-canvas layers of `side`², each a different flat colour.
fn layered(side: u32) -> Document {
    let layer = |value: u8| {
        let rgba = vec![value; 4 * (side * side) as usize];
        Document::from_rgba("layer", side, side, &rgba).layers[0].clone()
    };
    let mut doc = Document::from_rgba("base", side, side, &vec![200; 4 * (side * side) as usize]);
    doc.layers = LayerStack::from_layers(vec![layer(40), layer(90), layer(160)]);
    doc
}

/// Run `op` under a fresh job and return its final progress before `finish`.
fn progress_of(op: impl FnOnce(&mut Document)) -> u32 {
    let job = JobProgress::new();
    let mut doc = layered(48);
    job::scope(&job, || op(&mut doc));
    job.permille()
}

/// Run `op` under a job cancelled before it starts.
fn cancelled(op: impl FnOnce(&mut Document)) -> (Result<(), Cancelled>, u32, Document) {
    let job = JobProgress::new();
    job.cancel();
    let mut doc = layered(48);
    let result = job::run(&job, || op(&mut doc));
    (result, job.permille(), doc)
}

// [lop_counted_progress]
#[test]
fn image_size_counts_every_resampled_row() {
    let shown = progress_of(|doc| {
        resize_document(doc, 30, 20, pictura_ops::Resample::Bilinear).unwrap();
    });
    assert_eq!(shown, 999, "every counted row was reported");
}

// [lop_counted_progress]
#[test]
fn rotate_and_flip_count_layers() {
    assert_eq!(progress_of(|doc| rotate_document(doc, 1).unwrap()), 999);
    assert_eq!(progress_of(|doc| flip_document(doc, true)), 999);
}

// [lop_counted_progress]
#[test]
fn merge_and_flatten_count_composited_rows() {
    let merged = progress_of(|doc| {
        crate::merge_scope(doc, crate::MergeScope::Down("1")).unwrap();
    });
    assert_eq!(merged, 999);
    let flattened = progress_of(|doc| {
        crate::flatten(doc).unwrap();
    });
    assert_eq!(flattened, 999);
}

// [lop_whole_image_cancel]
#[test]
fn a_cancelled_engine_stops_and_reports_cancelled() {
    let before = layered(48);
    let (result, shown, _) = cancelled(|doc| {
        resize_document(doc, 30, 20, pictura_ops::Resample::Bilinear).unwrap();
    });
    assert_eq!(result, Err(Cancelled));
    assert_eq!(shown, 0, "no row was resampled");

    let (result, _, rotated) = cancelled(|doc| rotate_document(doc, 1).unwrap());
    assert_eq!(result, Err(Cancelled));
    let rects = |doc: &Document| doc.layers.iter().map(|l| l.rect).collect::<Vec<_>>();
    assert_eq!(rects(&rotated), rects(&before), "no layer was rotated");

    let (result, shown, _) = cancelled(|doc| {
        crate::flatten(doc).unwrap();
    });
    assert_eq!(result, Err(Cancelled));
    assert_eq!(shown, 0, "no band was composited");
}

#[test]
fn engines_without_a_job_are_unchanged() {
    let mut doc = layered(48);
    flip_document(&mut doc, false);
    assert!(pictura_core::job::current().is_none());
    assert_eq!(doc.layers.len(), 3);
}
