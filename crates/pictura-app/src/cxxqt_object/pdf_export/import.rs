//! The generic PDF / EPS import bridge (`interop/pdf-eps-import`): sniffing,
//! the page and image facts the Import PDF dialog shows, thumbnails, and
//! opening a page, an embedded image, or an EPS into a view through
//! Ghostscript (`pictura_codec::pdf::import`, `pictura_codec::eps`).
//!
//! Results travel as text: a first line `ok` or `error <kind>` (`nogs`,
//! `password`, `wrongpassword`, `failed`), then one fact, warning, or the
//! error message per line.

mod imp;

use super::super::qobject::PictureView;
pub use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    /// What one import produces: pixel size, edge smoothing, and the new document's mode (`grayscale`, `rgb`, `cmyk`, `lab`), depth (8 or 16), resolution, and display name.
    #[namespace = "pictura"]
    struct ImportTarget {
        width: i32,
        height: i32,
        anti_alias: bool,
        mode: QString,
        depth: i32,
        ppi: f64,
        per_cm: bool,
        name: QString,
    }

    #[namespace = "pictura"]
    struct PreparedVectorPdf {
        error: QString,
        page: u32,
        page_count: u32,
        crop: u8,
        width_points: f64,
        height_points: f64,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    }

    #[namespace = "pictura"]
    #[repr(u8)]
    enum VectorPlaceResult {
        Pending,
        Committed,
        Cancelled,
        Refused,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        fn vector_pdf_page_count(bytes: &[u8]) -> i32;
        fn vector_pdf_import_limit() -> u64;
        fn prepare_vector_pdf(
            bytes: &[u8],
            page: u32,
            crop: u8,
            canvas_width: u32,
            canvas_height: u32,
        ) -> PreparedVectorPdf;
        fn place_prepared_pdf(
            view: Pin<&mut PictureView>,
            source: &PreparedVectorPdf,
            bytes: &[u8],
            filename: &QString,
            smart: bool,
            label: &QString,
        ) -> QString;
        fn vector_place_result(view: &PictureView) -> VectorPlaceResult;
        fn vector_smart_object_id(view: &PictureView, path: &QString) -> QString;
        fn vector_smart_object_suffix(view: &PictureView, path: &QString) -> QString;
        fn vector_edit_revision(view: &PictureView, id: &QString) -> u64;
        fn start_vector_edit(
            view: Pin<&mut PictureView>,
            id: &QString,
            expected: u64,
            file: &QString,
        ) -> bool;
        fn relink_source_id(view: &PictureView, path: &QString) -> QString;
        fn relink_source_location(view: &PictureView, id: &QString) -> QString;
        fn start_relink_source(
            view: Pin<&mut PictureView>,
            id: &QString,
            file: &QString,
            relative: &QString,
            timestamp: &[u8],
        ) -> bool;
        fn relink_fixture_for_test(path: &QString, vector: bool, filter: bool) -> bool;

        /// `pdf` or `eps` from the file's leading bytes; empty for anything else.
        fn vector_sniff(path: &QString) -> QString;

        /// The pages of a PDF: `ok`, then per page `page <view height> <x0 y0 x1 y1>×6` (Bounding, Media, Crop, Bleed, Trim, Art boxes in view-space points), then `images <n>` (`-1` when unavailable) and `image <width> <height> <page>` per image.
        fn pdf_import_probe(path: &QString, password: &QString) -> QString;

        /// Page thumbnails whose longest page side is `longest` px: per page a little-endian `u32` width and height, then RGBA8888. Empty on failure.
        fn pdf_import_thumbnails(path: &QString, password: &QString, longest: i32) -> Vec<u8>;

        /// Images-mode thumbnails at most `longest` px, in the same layout; `password` opens an encrypted PDF.
        fn pdf_import_image_thumbnails(path: &QString, password: &QString, longest: i32)
            -> Vec<u8>;

        /// Open page `page` (1-based), cropped to `area` (`x0 y0 x1 y1 view_height`), into the view as a new document.
        fn pdf_import_open_page(
            view: Pin<&mut PictureView>,
            path: &QString,
            password: &QString,
            page: i32,
            area: &QString,
            target: &ImportTarget,
        ) -> QString;

        /// Open embedded image `index` at its own size into the view; `target`'s size is ignored.
        fn pdf_import_open_image(
            view: Pin<&mut PictureView>,
            path: &QString,
            password: &QString,
            index: i32,
            target: &ImportTarget,
        ) -> QString;

        /// The EPS bounding box: `ok`, then `x0 y0 x1 y1` in points.
        fn eps_import_probe(path: &QString) -> QString;

        /// Rasterize an EPS's bounding box into the view as a new document.
        fn eps_import_open(
            view: Pin<&mut PictureView>,
            path: &QString,
            target: &ImportTarget,
        ) -> QString;
    }
}
