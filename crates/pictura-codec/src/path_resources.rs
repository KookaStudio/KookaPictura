//! The document's Work Path (image resource 1025) and saved paths (2000-2997).
//!
//! A path resource's data is the same 26-byte path records a `vmsk` block
//! carries, without the version/flags header; a saved path's name is the
//! resource's Pascal name. The read derives `work_path` / `saved_paths`; the
//! write re-emits the section byte-for-byte while those still decode from it,
//! and otherwise replaces every path resource with the document's current paths.
//! The clipping-path resource (2999) names a path and stays verbatim.

use pictura_core::path::{NamedPath, Subpath, VectorPath};
use pictura_core::Document;

use crate::image_resources::{decode_section, encode_image_resources, frame_image_resource};
use crate::vector_mask::{decode_path_records, encode_path_records};

const WORK_PATH: u16 = 1025;
const SAVED_FIRST: u16 = 2000;
const SAVED_LAST: u16 = 2997;

fn is_path_resource(id: u16) -> bool {
    id == WORK_PATH || (SAVED_FIRST..=SAVED_LAST).contains(&id)
}

fn vector_path(subpaths: Vec<Subpath>) -> VectorPath {
    let mut path = VectorPath::default();
    for subpath in subpaths {
        path.add_subpath(subpath);
    }
    path
}

/// The Work Path and saved paths `section` decodes to at `width`×`height`. A
/// malformed path resource is skipped.
fn decode_paths(section: &[u8], width: u32, height: u32) -> (VectorPath, Vec<NamedPath>) {
    let mut work = VectorPath::default();
    let mut saved = Vec::new();
    for resource in decode_section(section).0 {
        if !is_path_resource(resource.id) {
            continue;
        }
        let Some(subpaths) = decode_path_records(&resource.data, width, height) else {
            continue;
        };
        if resource.id == WORK_PATH {
            work = vector_path(subpaths);
        } else {
            saved.push(NamedPath {
                name: resource.name,
                path: vector_path(subpaths),
            });
        }
    }
    (work, saved)
}

/// Derive the document's Work Path and saved paths from its preserved
/// image-resource section.
pub(crate) fn resolve_paths(doc: &mut Document) {
    let (work, saved) = decode_paths(&doc.image_resources, doc.width, doc.height);
    doc.work_path = work;
    doc.saved_paths = saved;
}

/// The image-resource section a save of `doc` under header mode `mode_code`
/// emits: the ICC-filtered section with the document's current paths.
pub(crate) fn output_resources(doc: &Document, mode_code: u16) -> Vec<u8> {
    with_document_paths(doc, crate::icc::resources_for_output(doc, mode_code))
}

/// `section` with its path resources standing for the document's current
/// Work Path and saved paths. Returned unchanged when it already decodes to
/// them, so an unedited file keeps its original path bytes.
pub(crate) fn with_document_paths(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    let (work, saved) = decode_paths(&section, doc.width, doc.height);
    let same_saved = saved.len() == doc.saved_paths.len()
        && saved
            .iter()
            .zip(&doc.saved_paths)
            .all(|(a, b)| a.name == b.name && a.path.subpaths == b.path.subpaths);
    if work.subpaths == doc.work_path.subpaths && same_saved {
        return section;
    }
    let (resources, consumed) = decode_section(&section);
    let mut kept: Vec<_> = resources
        .into_iter()
        .filter(|r| !is_path_resource(r.id))
        .collect();
    let records = |path: &VectorPath| {
        let mut out = Vec::new();
        encode_path_records(&path.subpaths, doc.width, doc.height, &mut out);
        out
    };
    if !doc.work_path.is_empty() {
        kept.push(frame_image_resource(
            WORK_PATH,
            "",
            &records(&doc.work_path),
        ));
    }
    // ponytail: past the 998 saved-path ids the rest are dropped on save.
    for (id, named) in (SAVED_FIRST..=SAVED_LAST).zip(&doc.saved_paths) {
        kept.push(frame_image_resource(id, &named.name, &records(&named.path)));
    }
    let mut out = encode_image_resources(&kept);
    out.extend_from_slice(&section[consumed..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::PathPoint;
    use pictura_core::{BitDepth, ColorMode};

    fn corner(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: (x, y),
            in_handle: None,
            out_handle: None,
            smooth: false,
        }
    }

    fn triangle() -> VectorPath {
        vector_path(vec![Subpath {
            points: vec![corner(10.0, 10.0), corner(90.0, 20.0), corner(50.0, 70.0)],
            closed: true,
        }])
    }

    fn curve() -> VectorPath {
        vector_path(vec![Subpath {
            points: vec![
                PathPoint {
                    anchor: (20.0, 50.0),
                    in_handle: None,
                    out_handle: Some((30.0, 25.0)),
                    smooth: false,
                },
                PathPoint {
                    anchor: (80.0, 50.0),
                    in_handle: Some((70.0, 25.0)),
                    out_handle: Some((90.0, 75.0)),
                    smooth: true,
                },
            ],
            closed: false,
        }])
    }

    #[test]
    fn work_and_saved_paths_survive_a_psd_round_trip() {
        let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
        doc.work_path = triangle();
        doc.saved_paths = vec![
            NamedPath {
                name: "Outline".into(),
                path: curve(),
            },
            NamedPath {
                name: "Path 2".into(),
                path: triangle(),
            },
        ];
        let bytes = crate::write_psd(&doc).expect("write");
        let back = crate::read_psd(&bytes).expect("read");
        assert_close(&back.work_path, &doc.work_path);
        let names: Vec<_> = back.saved_paths.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Outline", "Path 2"]);
        assert_close(&back.saved_paths[0].path, &curve());
        assert_close(&back.saved_paths[1].path, &triangle());
    }

    /// Equal up to the 8.24 fixed-point quantization of the path records.
    fn assert_close(a: &VectorPath, b: &VectorPath) {
        let near =
            |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-3 && (p.1 - q.1).abs() < 1e-3;
        assert_eq!(a.subpaths.len(), b.subpaths.len());
        for (x, y) in a.subpaths.iter().zip(&b.subpaths) {
            assert_eq!(x.closed, y.closed);
            assert_eq!(x.points.len(), y.points.len());
            for (p, q) in x.points.iter().zip(&y.points) {
                assert!(near(p.anchor, q.anchor), "{p:?} vs {q:?}");
                assert_eq!(p.smooth, q.smooth);
                assert_eq!(p.in_handle.is_some(), q.in_handle.is_some());
                assert_eq!(p.out_handle.is_some(), q.out_handle.is_some());
                for (h, k) in [(p.in_handle, q.in_handle), (p.out_handle, q.out_handle)] {
                    if let (Some(h), Some(k)) = (h, k) {
                        assert!(near(h, k), "{p:?} vs {q:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn unchanged_paths_keep_the_section_bytes() {
        let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
        doc.work_path = curve();
        let section = with_document_paths(&doc, Vec::new());
        doc.image_resources = section.clone().into();
        resolve_paths(&mut doc);
        assert_eq!(with_document_paths(&doc, section.clone()), section);
    }

    #[test]
    fn deleting_every_path_drops_the_resources_but_keeps_others() {
        let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
        doc.work_path = triangle();
        let other = frame_image_resource(1005, "", &[0u8; 16]);
        let section = with_document_paths(&doc, encode_image_resources(&[other]));
        doc.work_path = VectorPath::default();
        let ids: Vec<_> = decode_section(&with_document_paths(&doc, section))
            .0
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, [1005]);
    }
}
