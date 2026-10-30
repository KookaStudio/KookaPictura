//! The shape tools (Rectangle, Rounded Rectangle, Ellipse, Polygon): the live
//! outline and the three ways a shape lands, CS6's Shape / Path / Fill Pixels
//! modes, plus the Layers panel's view of a shape layer. Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` declaration list
//! does not grow).
//!
//! Every mode draws one [`ShapeSpec`]: a drag, or (`boxed`) the box a Create
//! dialog placed. The preview makes the same call, and each landing records
//! one `"<Tool> Tool"` state (e.g. `"Rectangle Tool"`). A Rectangle, Rounded
//! Rectangle, or Ellipse drawn in Shape mode is a live shape (a CC feature):
//! the layer keeps its parametric origin until an edit turns it into a regular
//! path (`paths.rs`).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    /// One shape: tool `kind` (0 Rectangle, 1 Rounded Rectangle, 2 Ellipse,
    /// 3 Polygon) and either a drag from `(x0, y0)` to `(x1, y1)` (`shift`
    /// squares off or snaps the Polygon's turn to 15°, `alt` grows from the
    /// press) or, when `boxed`, the box with those corners. The Rounded
    /// Rectangle's corner radii `r_tl`, `r_tr`, `r_br`, `r_bl` (px); the
    /// Polygon's `sides`, `star` with `indent` (% of the radius), and smooth
    /// corners / indents; the Line's `weight` (px) and arrowheads (at the start
    /// / end, width and length in % of the weight, concavity %); the Custom
    /// Shape's index (kind 4 Line, 5 Custom Shape); `align_edges` snaps
    /// straight edges to the pixel grid. A Shape-mode layer is filled unless
    /// `no_fill`, and stroked when `stroke` (`stroke_color` `0xAARRGGBB`,
    /// `stroke_width` px, `stroke_align` 0 inside / 1 centre / 2 outside).
    #[namespace = "pictura"]
    struct ShapeSpec {
        kind: i32,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        shift: bool,
        alt: bool,
        boxed: bool,
        r_tl: f64,
        r_tr: f64,
        r_br: f64,
        r_bl: f64,
        sides: i32,
        star: bool,
        indent: f64,
        smooth_corners: bool,
        smooth_indents: bool,
        weight: f64,
        arrow_start: bool,
        arrow_end: bool,
        arrow_width: f64,
        arrow_length: f64,
        arrow_concavity: f64,
        custom: i32,
        align_edges: bool,
        no_fill: bool,
        stroke: bool,
        stroke_color: u32,
        stroke_width: f64,
        stroke_align: i32,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The closed outline `spec` draws. Six numbers per anchor `[ax, ay, ix, iy, ox, oy]`, a missing handle at its anchor; empty when the shape encloses nothing.
        fn shape_outline(spec: &ShapeSpec) -> Vec<f64>;

        /// Shape mode: a new shape layer above the active layer (a `foreground` (`0xAARRGGBB`) color fill cut to the outline by a vector mask), named after the tool, live unless a Polygon. Returns its path; empty (no state) without a document or for an empty outline.
        fn shape_add_layer(
            view: Pin<&mut PictureView>,
            spec: &ShapeSpec,
            foreground: u32,
        ) -> QString;

        /// Path mode: add the outline to the Work Path as a new closed component. False (no state) without a document or for an empty outline.
        fn shape_add_path(view: Pin<&mut PictureView>, spec: &ShapeSpec) -> bool;

        /// Fill Pixels mode: paint the anti-aliased outline in `foreground` (`0xAARRGGBB`) onto the active pixel layer, inside the selection when there is one. False (no state) as for `draw_gradient`, or for an empty outline.
        fn shape_fill_pixels(
            view: Pin<&mut PictureView>,
            spec: &ShapeSpec,
            foreground: u32,
        ) -> bool;

        /// The number of built-in custom shapes.
        fn shape_custom_count() -> i32;

        /// Built-in custom shape `index`'s name, or empty out of range.
        fn shape_custom_name(index: i32) -> QString;

        /// Custom shape `index` as a black silhouette on transparency in a `size` square, for the picker; null out of range.
        fn shape_custom_preview(index: i32, size: i32) -> QImage;

        /// The active shape layer as `[width, height, fill_on, fill_argb, stroke_on, stroke_argb, stroke_width, stroke_align]` (flags 0/1, colours `0xAARRGGBB`); empty when the active layer is not a shape layer.
        fn shape_active(view: &PictureView) -> Vec<f64>;

        /// Fill the active shape layer with `argb`, or leave it unfilled (`on` false); one "Change Shape Fill" state. False (no state) when there is no active shape layer or nothing changed.
        fn shape_set_active_fill(view: Pin<&mut PictureView>, on: bool, argb: u32) -> bool;

        /// Stroke the active shape layer (`width` px, `align` 0 inside / 1 centre / 2 outside), or remove its stroke; one "Change Shape Stroke" state. False (no state) as for `shape_set_active_fill`.
        fn shape_set_active_stroke(
            view: Pin<&mut PictureView>,
            on: bool,
            argb: u32,
            width: f64,
            align: i32,
        ) -> bool;

        /// Scale the active shape layer's outline about its top-left corner to `width` x `height` px; one "Resize Shape" state. False (no state) as for `shape_set_active_fill`.
        fn shape_resize_active(view: Pin<&mut PictureView>, width: f64, height: f64) -> bool;

        /// Whether the layer at `path` is a shape layer.
        fn shape_is_shape(view: &PictureView, path: &QString) -> bool;

        /// Copy the shape layer at `path`'s fill and stroke into the shape-attributes clipboard. False (clipboard kept) when `path` is not a shape layer; no history.
        fn shape_copy_attributes(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Paste the shape-attributes clipboard onto the shape layer at `path`, replacing its fill and stroke; one "Paste Shape Attributes" state when it changed. False (no state) when `path` is not a shape layer or nothing changed.
        fn shape_paste_attributes(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether the shape-attributes clipboard holds a copy.
        fn shape_can_paste_attributes(view: &PictureView) -> bool;

        /// Rasterize the shape layer at `path` to ordinary pixels, dropping its shape/vector definition; one "Rasterize Shape" state. False (no state) when `path` is not a shape layer.
        fn shape_rasterize(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}
