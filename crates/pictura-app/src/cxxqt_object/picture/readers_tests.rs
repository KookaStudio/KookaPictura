//! The picture's whole readers against the whole composite (#1274): streamed
//! export, Copy Merged's region, and the reduced-level previews.

use super::*;
use crate::cxxqt_object::helpers_composite::buffer_to_rgba_bytes;
use crate::cxxqt_object::picture::readers_fixture::{full_picture, layered, rect, view_of};

/// Export, Save for Web and the raster save formats encode the same RGBA,
/// sRGB-converted or not, as the whole composite gives.
#[test]
fn export_streams_into_one_rgba() {
    for (w, h) in [(613, 541), (37, 23)] {
        let view = view_of(layered(w, h, true));
        let doc = view.session.doc.as_ref().expect("document");
        let full = full_picture(&view);
        let srgb = buffer_to_rgba_bytes(&pictura_codec::buffer_to_srgb(doc, &full));
        assert!(picture_rgba(doc, &view, true) == srgb, "sRGB {w}x{h}");
        assert!(
            picture_rgba(doc, &view, false) == buffer_to_rgba_bytes(&full),
            "working space {w}x{h}"
        );
    }
}

/// Copy Merged composites the selection's bounds only and copies what the
/// whole composite gives; with no selection it reads the whole canvas.
#[test]
fn copy_merged_reads_the_selection_bounds() {
    let view = view_of(layered(613, 541, false));
    let doc = view.session.doc.as_ref().expect("document");
    let full = pictura_render::rgba_frame(&full_picture(&view));
    let (w, h) = (doc.width as usize, doc.height as usize);
    let mut coverage = vec![0u8; w * h];
    let bounds = rect(97, 211, 402, 380);
    for y in bounds.top..bounds.bottom {
        for x in bounds.left..bounds.right {
            coverage[y as usize * w + x as usize] = ((x + y) % 256) as u8;
        }
    }
    for selection in [Some(&coverage[..]), None] {
        let mut read = Vec::new();
        let clip = pictura_render::copy_merged_region(doc, selection, |region| {
            read.push(region);
            picture_region(doc, &view, region)
        });
        let want = pictura_render::copy_merged(doc, &full, selection);
        assert_eq!(
            clip.map(|c| (c.rect, c.rgba, c.mask)),
            want.map(|c| (c.rect, c.rgba, c.mask))
        );
        let expected = if selection.is_some() {
            bounds
        } else {
            whole(doc)
        };
        assert_eq!(read, vec![expected]);
    }
}

/// A preview of a large document reads the coarsest level whose longer side
/// still reaches the preview size.
#[test]
fn previews_read_a_reduced_level() {
    let view = view_of(layered(2101, 1301, false));
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(preview_level(2101, 1301, 256), 3);
    let preview = preview_picture(doc, &view, 256);
    assert_eq!((preview.width, preview.height), (263, 163));
    assert!(matches!(preview, Cow::Owned(_)));
    let level = preview_display(doc, false, 512).expect("a reduced level");
    assert_eq!((level.width(), level.height()), (526, 326));
}

/// A preview no smaller than the document is today's whole picture.
#[test]
fn small_previews_are_unchanged() {
    let view = view_of(layered(301, 211, false));
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(preview_level(301, 211, 512), 0);
    assert!(preview_picture(doc, &view, 512).into_owned() == full_picture(&view));
    assert!(preview_display(doc, false, 512).is_none());
}

/// `preview_image`'s pixels on a large document are a reduced display level,
/// equal to the same level of the view pyramid the canvas shows.
#[test]
fn preview_image_reads_a_reduced_level() {
    let view = view_of(layered(1033, 517, true));
    let doc = view.session.doc.as_ref().expect("document");
    let level = preview_display(doc, false, 300).expect("a reduced level");
    assert_eq!(preview_level(1033, 517, 300), 1);
    let (lw, lh) = (level.width(), level.height());
    let whole_level = pictura_render::composite_region_level(
        doc,
        rect(0, 0, lw as i32, lh as i32),
        1,
        &|chunk| {
            crate::cxxqt_object::helpers_composite::into_rgba_frame(
                pictura_codec::buffer_to_srgb(doc, &chunk).into_owned(),
            )
        },
    );
    assert!(level.data() == whole_level.data());
}
