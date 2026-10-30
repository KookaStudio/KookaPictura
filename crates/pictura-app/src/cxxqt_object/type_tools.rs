//! The Type tools' commands: Horizontal / Vertical Type and their Type Mask
//! twins. Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! While text is being typed the canvas shows `type_preview_*` — the engine's
//! own render, so the preview is the commit — and nothing touches the document
//! or history. `type_commit_layer` adds one type layer and records one
//! "Horizontal Type" / "Vertical Type" state; `type_commit_mask` merges the
//! type's coverage into the selection and records one "Horizontal Type Mask" /
//! "Vertical Type Mask" state. Clicking an existing type layer reopens it:
//! `type_edit_begin` hides it (no state) while its text is retyped over it,
//! `type_commit_edit` re-sets it in place as one "Edit Type Layer" state, and
//! `type_edit_cancel` shows it again unchanged.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    /// Where and how the Type tools place the text: the click `(x, y)` in
    /// document pixels — the first baseline's start, or the first column's top
    /// centre for vertical type — the linear part `xx, xy, yx, yy` a
    /// transformed type layer reopens with (identity for new type), and the
    /// orientation. Placement rides separately from the character and paragraph
    /// attribute sets. `boxed` makes it paragraph type, wrapping to the box
    /// `box_left, box_top, box_right, box_bottom` relative to `(x, y)` before
    /// the linear part (a dragged box has its top-left at `(x, y)`). The
    /// `warp_*` fields are `Type > Warp Text`: `warp_style` a
    /// `WarpStyle::id` (empty or `none` for no warp), the bend and the
    /// horizontal / vertical distortions in percent.
    #[namespace = "pictura"]
    struct TypeSetting {
        vertical: bool,
        x: f64,
        y: f64,
        xx: f64,
        xy: f64,
        yx: f64,
        yy: f64,
        boxed: bool,
        box_left: f64,
        box_top: f64,
        box_right: f64,
        box_bottom: f64,
        /// 0 none, 1 on-path, 2 area. `path_points` holds the cubic segments as
        /// eight space-separated numbers each, segments separated by `;`.
        path_kind: i32,
        path_closed: bool,
        path_start: f64,
        path_points: QString,
        warp_style: QString,
        warp_bend: f64,
        warp_h: f64,
        warp_v: f64,
        warp_vertical: bool,
    }

    /// The character attribute set as the bridge carries it: enums as `i32`
    /// (`leading_mode` 0 Auto / 1 Fixed, `kerning_mode` 0 Metrics / 1 Optical /
    /// 2 Manual, `anti_alias` 0 None … 4 Smooth), `color` as `0xAARRGGBB`, the toggles as
    /// `bool`, and the model's units for sizes, scales, and spacing. The font
    /// family and PostScript style ride separately: `font_style` here, the
    /// family on the bridge functions' dedicated `font` argument.
    #[namespace = "pictura"]
    struct CharacterSetting {
        font_style: QString,
        size: f64,
        leading_mode: i32,
        leading_value: f64,
        kerning_mode: i32,
        kerning_value: i32,
        tracking: f64,
        horizontal_scale: f64,
        vertical_scale: f64,
        baseline_shift: f64,
        tsume: f64,
        anti_alias: i32,
        color: u32,
        all_caps: bool,
        small_caps: bool,
        superscript: bool,
        subscript: bool,
        underline: bool,
        strikethrough: bool,
        fractional_widths: bool,
        faux_bold: bool,
        faux_italic: bool,
        standard_ligatures: bool,
        contextual_alternates: bool,
        discretionary_ligatures: bool,
        swash: bool,
        oldstyle: bool,
        stylistic_alternates: bool,
        titling_alternates: bool,
        ornaments: bool,
        ordinals: bool,
        fractions: bool,
        language: QString,
        vertical_roman_alignment: bool,
    }

    /// The paragraph attribute set as the bridge carries it: `justify` 0 Left,
    /// 1 Right, 2 Center, 3 Justify Last Left, 4 Justify Last Right, 5 Justify
    /// Last Center, 6 Justify All; word, letter, and glyph spacing as min /
    /// desired / max in the model's units; `direction` is inert (kept for the
    /// generated C++ header); `composer` 0 SingleLine / 1 EveryLine.
    #[namespace = "pictura"]
    struct ParagraphSetting {
        justify: i32,
        word_spacing_min: f64,
        word_spacing_desired: f64,
        word_spacing_max: f64,
        letter_spacing_min: f64,
        letter_spacing_desired: f64,
        letter_spacing_max: f64,
        glyph_spacing_min: f64,
        glyph_spacing_desired: f64,
        glyph_spacing_max: f64,
        start_indent: f64,
        end_indent: f64,
        first_line_indent: f64,
        space_before: f64,
        space_after: f64,
        hanging: bool,
        hyphenate: bool,
        direction: i32,
        composer: i32,
        auto_leading: f64,
        hyphenate_word_size: i32,
        hyphenate_pre: i32,
        hyphenate_post: i32,
        hyphen_limit: i32,
        hyphenation_zone: f64,
        hyphenate_caps: bool,
    }

    /// One character style as the Character Styles panel lists it.
    #[namespace = "pictura"]
    struct CharacterStyleRow {
        name: QString,
        font: QString,
        character: CharacterSetting,
    }

    /// One paragraph style as the Paragraph Styles panel lists it.
    #[namespace = "pictura"]
    struct ParagraphStyleRow {
        name: QString,
        font: QString,
        character: CharacterSetting,
        paragraph: ParagraphSetting,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        fn type_preview_rect(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<i32>;

        /// The default character attribute set for new type, matching the model.
        fn type_default_character_setting() -> CharacterSetting;

        /// The default paragraph attribute set for new type, matching the model.
        fn type_default_paragraph_setting() -> ParagraphSetting;

        /// Register `bytes` (an sfnt Qt resolved for `family` and `style`) so type set in that face renders in it; false for bytes that do not parse.
        fn type_register_font(family: &QString, style: &QString, bytes: &[u8]) -> bool;

        /// Which of the Character panel's eight OpenType features
        /// `family`/`style`'s face carries: bit 0 Standard Ligatures,
        /// 1 Contextual Alternates, 2 Discretionary Ligatures, 3 Swash,
        /// 4 Stylistic Alternates, 5 Titling Alternates, 6 Ordinals,
        /// 7 Fractions.
        fn type_font_opentype_features(family: &QString, style: &QString) -> u32;

        /// The caret at every UTF-16 boundary of `text` (`len + 1` segments), flat `[x0, y0, x1, y1, …]` in document pixels; empty for a non-positive size.
        fn type_caret_stops(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<f64>;

        /// The baseline guide under each non-blank line, flat `[x0, y0, x1, y1, …]` in document pixels.
        fn type_baselines(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<f64>;

        /// `text`'s box in its own frame (relative to the origin, before the matrix) as `[left, top, right, bottom]`: paragraph type's box, point type's text bounds; empty for blank text.
        fn type_local_bounds(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<f64>;

        /// Whether paragraph type has text past its box; false for point type.
        fn type_overflows(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Convert the type layer at `path` to paragraph type (`paragraph` true) or point type and record one "Convert To Paragraph Text" / "Convert To Point Text" state; false (no state) when it is not a type layer of the other kind.
        fn type_convert(view: Pin<&mut PictureView>, path: &QString, paragraph: bool) -> bool;

        /// Whether the type layer at `path` is paragraph type; false for point type or a non-type layer.
        fn type_layer_is_paragraph(view: &PictureView, path: &QString) -> bool;

        /// `text` rendered over its preview rect as packed straight RGBA; empty when nothing renders.
        fn type_preview_rgba(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<u8>;

        /// Add `text` as a type layer above the active layer, make it the active layer, and record one "Horizontal Type" / "Vertical Type" state. Returns the new path; empty (no state) for blank text or a document that is not 8-bit RGB.
        fn type_commit_layer(
            view: Pin<&mut PictureView>,
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> QString;

        /// The topmost visible type layer under `(x, y)`; empty when none.
        fn type_layer_at(view: &PictureView, x: f64, y: f64) -> QString;
        /// The path under `(x, y)`: `[kind, closed, start, segment_count,
        /// x0, y0, c1x, c1y, c2x, c2y, x1, y1, ...]`; empty when none.
        fn type_path_at(view: &PictureView, x: f64, y: f64, tolerance: f64) -> Vec<f64>;
        /// The overflow `+` point for on-path type, `[x, y]`; empty otherwise.
        fn type_path_overflow_point(
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> Vec<f64>;

        /// The text of the type layer at `path`, lines separated by `\r`; empty for a non-type layer.
        fn type_layer_text(view: &PictureView, path: &QString) -> QString;

        /// The font family the type layer at `path` names; empty for a non-type layer.
        fn type_layer_font(view: &PictureView, path: &QString) -> QString;

        /// Where the type layer at `path` sits, its origin following the layer; identity for a non-type layer.
        fn type_layer_setting(view: &PictureView, path: &QString) -> TypeSetting;

        /// The type layer at `path`'s character attribute set; the model defaults for a non-type layer.
        fn type_layer_character_setting(view: &PictureView, path: &QString) -> CharacterSetting;

        /// The type layer at `path`'s paragraph attribute set; the model defaults for a non-type layer.
        fn type_layer_paragraph_setting(view: &PictureView, path: &QString) -> ParagraphSetting;

        /// Hide the type layer at `path` while it is retyped. No history; false for a non-type layer.
        fn type_edit_begin(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Show the type layer at `path` again, unchanged. No history.
        fn type_edit_cancel(view: Pin<&mut PictureView>, path: &QString);

        /// Re-set the type layer at `path` from `text` and record one "Edit Type Layer" state. Blank text or a failed re-set shows the layer again unchanged and returns false (no state).
        fn type_commit_edit(
            view: Pin<&mut PictureView>,
            path: &QString,
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Re-set the type layer at `path` from `text` without reopening it (the options bar changed while it is selected) and record one "Edit Type Layer" state; false (no state) for blank text or a non-type layer.
        fn type_update_layer(
            view: Pin<&mut PictureView>,
            path: &QString,
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// As `type_update_layer` but only shows the result: no history state, so a live preview (the Text Color picker open) can be dropped by re-setting the original.
        fn type_preview_layer(
            view: Pin<&mut PictureView>,
            path: &QString,
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Apply the saved character (`paragraph` false) or paragraph (`paragraph` true) style named `name` to the type layer at `path`, resolve it, and record one "Apply Type Style" state; false (no state) for a missing style or non-type layer.
        fn type_apply_style(
            view: Pin<&mut PictureView>,
            path: &QString,
            name: &QString,
            paragraph: bool,
        ) -> bool;

        /// Create the named character style from `character` and record one "New Character Style" state; false (no state) for a duplicate name.
        fn type_create_character_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
        ) -> bool;

        /// Edit the named character style to `character` (`new_name` may rename it), re-resolve every applying type layer, and record one "Edit Character Style" state; false (no state) for a missing style.
        fn type_edit_character_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            new_name: &QString,
            font: &QString,
            character: &CharacterSetting,
        ) -> bool;

        /// Edit the named character style for a live Preview, re-resolving every applying type layer and recording no history state; false (unchanged) for a missing style.
        fn type_preview_character_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
        ) -> bool;

        /// Delete the named character style and record one "Delete Character Style" state; false (no state) for a missing style.
        fn type_delete_character_style(view: Pin<&mut PictureView>, name: &QString) -> bool;

        /// Every character style in creation order, each resolved against the
        /// style sheet (a sparse style reports the defaults it leaves unset).
        fn type_character_styles(view: &PictureView) -> Vec<CharacterStyleRow>;

        /// Create the named paragraph style and record one "New Paragraph Style" state; false (no state) for a duplicate name.
        fn type_create_paragraph_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Edit the named paragraph style, re-resolve every applying type layer, and record one "Edit Paragraph Style" state; false (no state) for a missing style.
        fn type_edit_paragraph_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Edit the named paragraph style for a live Preview, re-resolving every applying type layer and recording no history state; false (unchanged) for a missing style.
        fn type_preview_paragraph_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
        ) -> bool;

        /// Delete the named paragraph style and record one "Delete Paragraph Style" state; false (no state) for a missing or protected style.
        fn type_delete_paragraph_style(view: Pin<&mut PictureView>, name: &QString) -> bool;

        /// Every paragraph style in panel order (the default `Basic Paragraph`
        /// first), each resolved against the style sheet.
        fn type_paragraph_styles(view: &PictureView) -> Vec<ParagraphStyleRow>;

        /// Merge `text`'s coverage into the selection with `mode` ("new", "add", "subtract", "intersect") and record one "Horizontal Type Mask" / "Vertical Type Mask" state. False (no state) for blank text or without a document.
        fn type_commit_mask(
            view: Pin<&mut PictureView>,
            text: &QString,
            font: &QString,
            setting: &TypeSetting,
            character: &CharacterSetting,
            paragraph: &ParagraphSetting,
            mode: &QString,
        ) -> bool;
    }
}

mod attrs;
mod fonts;
mod spec;
mod warp;
