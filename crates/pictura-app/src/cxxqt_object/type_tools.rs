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

use super::helpers::combine_mode_from;
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::path::PathPoint;
use pictura_core::{
    CharacterAttrs, ParagraphAttrs, StyleOverrides, TextPath, TextPathKind, TypeSpec,
};
use pictura_select::Selection;

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

        /// Register `bytes` (an sfnt Qt resolved for `family`) so type set in `family` renders in it; false for bytes that do not parse.
        fn type_register_font(family: &QString, bytes: &[u8]) -> bool;

        /// Which of the Character panel's eight OpenType features `family`'s
        /// face carries: bit 0 Standard Ligatures, 1 Contextual Alternates, 2
        /// Discretionary Ligatures, 3 Swash, 4 Stylistic Alternates, 5 Titling
        /// Alternates, 6 Ordinals, 7 Fractions.
        fn type_font_opentype_features(family: &QString) -> u32;

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

        /// Edit the named character style to `character`, re-resolve every applying type layer, and record one "Edit Character Style" state; false (no state) for a missing style.
        fn type_edit_character_style(
            view: Pin<&mut PictureView>,
            name: &QString,
            font: &QString,
            character: &CharacterSetting,
        ) -> bool;

        /// Delete the named character style and record one "Delete Character Style" state; false (no state) for a missing style.
        fn type_delete_character_style(view: Pin<&mut PictureView>, name: &QString) -> bool;

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

        /// The number of paragraph styles in the document (0 without one).
        fn type_paragraph_style_count(view: &PictureView) -> i32;

        /// The paragraph style name at `index` in panel order (the default `Basic Paragraph` first); empty out of range.
        fn type_paragraph_style_name(view: &PictureView, index: i32) -> QString;

        /// The named paragraph style's effective character attribute set, resolved against the style sheet; the default set for an unknown name.
        fn type_paragraph_style_character(view: &PictureView, name: &QString) -> CharacterSetting;

        /// The named paragraph style's effective paragraph attribute set, resolved against the style sheet; the default set for an unknown name.
        fn type_paragraph_style_paragraph(view: &PictureView, name: &QString) -> ParagraphSetting;

        /// The named paragraph style's font family (its effective character font), empty for an unknown name.
        fn type_paragraph_style_font(view: &PictureView, name: &QString) -> QString;

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

use ffi::{CharacterSetting, ParagraphSetting, TypeSetting};

mod attrs;
mod fonts;
mod warp;

use attrs::{
    character_attrs, character_setting, edited_overrides, paragraph_attrs, paragraph_overrides,
    paragraph_setting, style_character_overrides,
};

fn type_default_character_setting() -> CharacterSetting {
    character_setting(&CharacterAttrs::default())
}

fn type_default_paragraph_setting() -> ParagraphSetting {
    paragraph_setting(&ParagraphAttrs::default())
}

fn spec(
    text: &QString,
    font: &QString,
    placement: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> TypeSpec {
    let mut spec = TypeSpec::new(text.to_string(), font.to_string(), character.size);
    spec.vertical = placement.vertical;
    spec.origin = (placement.x, placement.y);
    spec.matrix = [placement.xx, placement.xy, placement.yx, placement.yy];
    spec.box_bounds = placement.boxed.then_some([
        placement.box_left,
        placement.box_top,
        placement.box_right,
        placement.box_bottom,
    ]);
    if let Some(path) = path_from_setting(placement) {
        if path.kind == TextPathKind::Area {
            let flattened = path.subpath.flattened(0.25);
            if let Some(&(first_x, first_y)) = flattened.points.first() {
                let (mut left, mut top, mut right, mut bottom) =
                    (first_x, first_y, first_x, first_y);
                for &(x, y) in &flattened.points {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
                spec.origin = (left, top);
                spec.box_bounds = Some([0.0, 0.0, right - left, bottom - top]);
            }
        } else {
            let flattened = path.subpath.flattened(0.25);
            if let Some((point, _)) = flattened.point_at(path.start) {
                spec.origin = point;
            }
        }
        spec.path = Some(path);
    }
    spec.character = character_attrs(character);
    spec.character.font_family = font.to_string();
    spec.paragraph = paragraph_attrs(paragraph);
    spec.warp = warp::text_warp(placement);
    spec
}

fn path_from_setting(setting: &TypeSetting) -> Option<TextPath> {
    let kind = match setting.path_kind {
        1 => TextPathKind::OnPath,
        2 => TextPathKind::Area,
        _ => return None,
    };
    let segments: Vec<[(f64, f64); 4]> = setting
        .path_points
        .to_string()
        .split(';')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let values: Vec<f64> = part
                .split_whitespace()
                .filter_map(|value| value.parse().ok())
                .collect();
            (values.len() == 8).then(|| {
                [
                    (values[0], values[1]),
                    (values[2], values[3]),
                    (values[4], values[5]),
                    (values[6], values[7]),
                ]
            })
        })
        .collect();
    if segments.is_empty() {
        return None;
    }
    let mut points = Vec::with_capacity(segments.len() + usize::from(!setting.path_closed));
    for (index, segment) in segments.iter().enumerate() {
        let in_handle = if index == 0 {
            setting
                .path_closed
                .then(|| segments.last().map(|last| last[2]))
                .flatten()
        } else {
            Some(segments[index - 1][2])
        };
        points.push(PathPoint {
            anchor: segment[0],
            in_handle,
            out_handle: Some(segment[1]),
            smooth: true,
        });
    }
    if !setting.path_closed {
        if let Some(last) = segments.last() {
            points.push(PathPoint {
                anchor: last[3],
                in_handle: Some(last[2]),
                out_handle: None,
                smooth: false,
            });
        }
    }
    Some(TextPath {
        kind,
        subpath: pictura_core::path::Subpath {
            points,
            closed: setting.path_closed,
        },
        start: setting.path_start,
    })
}

fn path_points(path: &TextPath) -> QString {
    let mut parts = Vec::new();
    for index in 0..path.subpath.segment_count() {
        let Some(segment) = path.subpath.segment(index) else {
            continue;
        };
        let values: Vec<String> = segment
            .iter()
            .flat_map(|(x, y)| [format!("{x}"), format!("{y}")])
            .collect();
        parts.push(values.join(" "));
    }
    QString::from(parts.join(";").as_str())
}

fn type_preview_rect(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<i32> {
    pictura_render::type_placement(&spec(text, font, setting, character, paragraph))
        .map_or_else(Vec::new, |p| {
            vec![p.rect.left, p.rect.top, p.rect.width(), p.rect.height()]
        })
}

fn type_register_font(family: &QString, bytes: &[u8]) -> bool {
    pictura_render::register_font(&family.to_string(), bytes.to_vec())
}

fn type_font_opentype_features(family: &QString) -> u32 {
    pictura_render::opentype_features(&family.to_string())
}

fn type_caret_stops(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_caret_stops(&spec(text, font, setting, character, paragraph))
        .into_iter()
        .flatten()
        .collect()
}

fn type_baselines(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_baselines(&spec(text, font, setting, character, paragraph))
        .into_iter()
        .flatten()
        .collect()
}

fn type_overflows(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    pictura_render::type_overflows(&spec(text, font, setting, character, paragraph))
}

fn type_local_bounds(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_placement(&spec(text, font, setting, character, paragraph))
        .map_or_else(Vec::new, |p| p.bounds.to_vec())
}

fn type_layer_is_paragraph(view: &PictureView, path: &QString) -> bool {
    layer_spec(view, path).is_some_and(|s| s.box_bounds.is_some())
}

fn type_convert(mut view: Pin<&mut PictureView>, path: &QString, paragraph: bool) -> bool {
    let Some(previous) = layer_spec(view.as_ref().get_ref(), path) else {
        return false;
    };
    let converted = if paragraph {
        pictura_render::type_to_paragraph(&previous)
    } else {
        pictura_render::type_to_point(&previous)
    };
    let Some(spec) = converted.filter(|s| !s.text.trim().is_empty()) else {
        return false;
    };
    let replaced = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| pictura_render::replace_type_layer(doc, &path.to_string(), &spec));
    if replaced {
        view.as_mut().edit(
            if paragraph {
                "Convert To Paragraph Text"
            } else {
                "Convert To Point Text"
            },
            |_| Edit::Structural,
        );
    }
    replaced
}

fn type_preview_rgba(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<u8> {
    let Some((_, buffer)) =
        pictura_render::render_type(&spec(text, font, setting, character, paragraph))
    else {
        return Vec::new();
    };
    let plane = buffer.pixel_count();
    (0..plane)
        .flat_map(|i| (0..4).map(move |c| c * plane + i))
        .map(|i| buffer.data[i])
        .collect()
}

fn type_commit_layer(
    mut view: Pin<&mut PictureView>,
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> QString {
    let spec = spec(text, font, setting, character, paragraph);
    let created = {
        let mut rust = view.as_mut().rust_mut();
        let above = rust
            .layer_target
            .anchor()
            .map(str::to_string)
            .unwrap_or_default();
        let created = match rust.session.doc.as_mut() {
            Some(doc) => pictura_render::add_type_layer(doc, &above, &spec),
            None => String::new(),
        };
        if !created.is_empty() {
            rust.select_only_layer(&created);
        }
        created
    };
    if !created.is_empty() {
        view.as_mut().edit(
            if spec.vertical {
                "Vertical Type"
            } else {
                "Horizontal Type"
            },
            |_| Edit::Structural,
        );
    }
    QString::from(created.as_str())
}

fn type_commit_mask(
    mut view: Pin<&mut PictureView>,
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
    mode: &QString,
) -> bool {
    let spec = spec(text, font, setting, character, paragraph);
    let shape = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let Some(data) = pictura_render::type_mask(doc.width, doc.height, &spec) else {
            return false;
        };
        Selection {
            width: doc.width,
            height: doc.height,
            data,
        }
    };
    let label = if spec.vertical {
        "Vertical Type Mask"
    } else {
        "Horizontal Type Mask"
    };
    view.as_mut()
        .apply_selection_labeled(shape, combine_mode_from(&mode.to_string()), label)
}

pub(super) fn layer_spec(view: &PictureView, path: &QString) -> Option<TypeSpec> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::type_layer_spec(pictura_render::resolve_path(doc, &path.to_string())?)
}

fn type_layer_at(view: &PictureView, x: f64, y: f64) -> QString {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::type_layer_at(doc, x, y))
        .map_or_else(QString::default, |path| QString::from(path.as_str()))
}

fn type_layer_text(view: &PictureView, path: &QString) -> QString {
    layer_spec(view, path).map_or_else(QString::default, |s| QString::from(s.text.as_str()))
}

fn type_path_at(view: &PictureView, x: f64, y: f64, tolerance: f64) -> Vec<f64> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let Some(hit) = pictura_render::path_hit_at(doc, x, y, tolerance) else {
        return Vec::new();
    };
    let points = hit.control_points();
    let mut out = vec![
        match hit.kind {
            TextPathKind::OnPath => 1.0,
            TextPathKind::Area => 2.0,
        },
        if hit.subpath.closed { 1.0 } else { 0.0 },
        hit.start,
        (points.len() / 8) as f64,
    ];
    out.extend(points);
    out
}

fn type_path_overflow_point(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_path_overflow_point(&spec(text, font, setting, character, paragraph))
        .map_or_else(Vec::new, |(x, y)| vec![x, y])
}

fn type_layer_font(view: &PictureView, path: &QString) -> QString {
    layer_spec(view, path).map_or_else(QString::default, |s| {
        QString::from(s.character.font_family.as_str())
    })
}

fn placement_setting(spec: &TypeSpec) -> TypeSetting {
    let b = spec.box_bounds.unwrap_or_default();
    TypeSetting {
        vertical: spec.vertical,
        x: spec.origin.0,
        y: spec.origin.1,
        xx: spec.matrix[0],
        xy: spec.matrix[1],
        yx: spec.matrix[2],
        yy: spec.matrix[3],
        boxed: spec.box_bounds.is_some(),
        box_left: b[0],
        box_top: b[1],
        box_right: b[2],
        box_bottom: b[3],
        path_kind: spec.path.as_ref().map_or(0, |path| match path.kind {
            TextPathKind::OnPath => 1,
            TextPathKind::Area => 2,
        }),
        path_closed: spec.path.as_ref().is_some_and(|path| path.subpath.closed),
        path_start: spec.path.as_ref().map_or(0.0, |path| path.start),
        path_points: spec
            .path
            .as_ref()
            .map_or_else(QString::default, path_points),
        warp_style: QString::from(if spec.warp.style.is_preset() {
            spec.warp.style.id()
        } else {
            ""
        }),
        warp_bend: spec.warp.bend,
        warp_h: spec.warp.h_distort,
        warp_v: spec.warp.v_distort,
        warp_vertical: spec.warp.vertical,
    }
}

fn type_layer_setting(view: &PictureView, path: &QString) -> TypeSetting {
    layer_spec(view, path).map_or(
        TypeSetting {
            vertical: false,
            x: 0.0,
            y: 0.0,
            xx: 1.0,
            xy: 0.0,
            yx: 0.0,
            yy: 1.0,
            boxed: false,
            box_left: 0.0,
            box_top: 0.0,
            box_right: 0.0,
            box_bottom: 0.0,
            path_kind: 0,
            path_closed: false,
            path_start: 0.0,
            path_points: QString::default(),
            warp_style: QString::default(),
            warp_bend: 0.0,
            warp_h: 0.0,
            warp_v: 0.0,
            warp_vertical: false,
        },
        |spec| placement_setting(&spec),
    )
}

fn type_layer_character_setting(view: &PictureView, path: &QString) -> CharacterSetting {
    layer_spec(view, path).map_or_else(type_default_character_setting, |s| {
        character_setting(&s.character)
    })
}

fn type_layer_paragraph_setting(view: &PictureView, path: &QString) -> ParagraphSetting {
    layer_spec(view, path).map_or_else(type_default_paragraph_setting, |s| {
        paragraph_setting(&s.paragraph)
    })
}

fn set_visible(mut view: Pin<&mut PictureView>, path: &QString, visible: bool) -> bool {
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .and_then(|doc| pictura_render::resolve_path_mut(doc, &path.to_string()))
        .filter(|layer| layer.type_tool.is_some())
        .map(|layer| layer.visible = visible)
        .is_some();
    if changed {
        view.as_mut().recomposite();
    }
    changed
}

fn type_edit_begin(view: Pin<&mut PictureView>, path: &QString) -> bool {
    set_visible(view, path, false)
}

fn type_edit_cancel(view: Pin<&mut PictureView>, path: &QString) {
    set_visible(view, path, true);
}

fn type_commit_edit(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    if !replace(
        view.as_mut(),
        path,
        text,
        font,
        setting,
        character,
        paragraph,
    ) {
        type_edit_cancel(view, path);
        return false;
    }
    set_shown(view.as_mut(), path);
    commit_edit(view);
    true
}

fn type_update_layer(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    let replaced = replace(
        view.as_mut(),
        path,
        text,
        font,
        setting,
        character,
        paragraph,
    );
    if replaced {
        commit_edit(view);
    }
    replaced
}

fn type_apply_style(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    name: &QString,
    paragraph: bool,
) -> bool {
    let applied = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| {
            pictura_render::apply_type_style(doc, &path.to_string(), &name.to_string(), paragraph)
        });
    if applied {
        view.as_mut().edit("Apply Type Style", |_| Edit::Structural);
    }
    applied
}

/// Run a style operation on the document and record one history state when it
/// changed anything.
fn commit_style(
    mut view: Pin<&mut PictureView>,
    label: &str,
    apply: impl FnOnce(&mut pictura_core::Document) -> bool,
) -> bool {
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(apply);
    if changed {
        view.as_mut().edit(label, |_| Edit::Structural);
    }
    changed
}

fn type_create_character_style(
    view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
) -> bool {
    let name = name.to_string();
    let attrs = style_character_overrides(font, character);
    commit_style(view, "New Character Style", |doc| {
        crate::history::create_character_style(doc, &name, attrs).is_ok()
    })
}

fn type_edit_character_style(
    view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
) -> bool {
    let name = name.to_string();
    let attrs = style_character_overrides(font, character);
    commit_style(view, "Edit Character Style", |doc| {
        crate::history::edit_character_style(doc, &name, attrs).is_ok()
    })
}

fn type_delete_character_style(view: Pin<&mut PictureView>, name: &QString) -> bool {
    let name = name.to_string();
    commit_style(view, "Delete Character Style", |doc| {
        crate::history::delete_character_style(doc, &name).is_ok()
    })
}

fn type_create_paragraph_style(
    view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    let name = name.to_string();
    let character = style_character_overrides(font, character);
    let paragraph = paragraph_overrides(&paragraph_attrs(paragraph));
    commit_style(view, "New Paragraph Style", |doc| {
        crate::history::create_paragraph_style(doc, &name, character, paragraph).is_ok()
    })
}

fn type_edit_paragraph_style(
    view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    let name = name.to_string();
    let character = style_character_overrides(font, character);
    let paragraph = paragraph_overrides(&paragraph_attrs(paragraph));
    commit_style(view, "Edit Paragraph Style", |doc| {
        crate::history::edit_paragraph_style(doc, &name, character, paragraph).is_ok()
    })
}

fn type_delete_paragraph_style(view: Pin<&mut PictureView>, name: &QString) -> bool {
    let name = name.to_string();
    commit_style(view, "Delete Paragraph Style", |doc| {
        crate::history::delete_paragraph_style(doc, &name).is_ok()
    })
}

/// A live Preview edit: the style sheet and every applying layer change, the
/// display refreshes, and nothing is recorded; Cancel restores by previewing
/// the style's pre-dialog values back.
fn type_preview_paragraph_style(
    mut view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    let name = name.to_string();
    let character = style_character_overrides(font, character);
    let paragraph = paragraph_overrides(&paragraph_attrs(paragraph));
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| {
            crate::history::edit_paragraph_style(doc, &name, character, paragraph).is_ok()
        });
    if changed {
        view.as_mut().clear_link_sets();
        view.as_mut().recomposite();
        view.as_mut().changed();
    }
    changed
}

fn type_paragraph_style_count(view: &PictureView) -> i32 {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(0, |doc| doc.text_styles.paragraph_styles.len() as i32)
}

fn type_paragraph_style_name(view: &PictureView, index: i32) -> QString {
    if index < 0 {
        return QString::default();
    }
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| doc.text_styles.paragraph_styles.get(index as usize))
        .map_or_else(QString::default, |style| QString::from(style.name.as_str()))
}

/// A paragraph style's effective attributes, resolved against the sheet so a
/// sparse style reports the document defaults for the fields it leaves unset.
fn resolved_paragraph_style(
    view: &PictureView,
    name: &QString,
) -> Option<pictura_core::ResolvedStyle> {
    let name = name.to_string();
    let doc = view.rust().session.doc.as_ref()?;
    doc.text_styles.paragraph_style(&name).map(|_| {
        doc.text_styles
            .resolve(&StyleOverrides::default(), None, Some(&name))
    })
}

fn type_paragraph_style_character(view: &PictureView, name: &QString) -> CharacterSetting {
    resolved_paragraph_style(view, name).map_or_else(type_default_character_setting, |r| {
        character_setting(&r.character)
    })
}

fn type_paragraph_style_paragraph(view: &PictureView, name: &QString) -> ParagraphSetting {
    resolved_paragraph_style(view, name).map_or_else(type_default_paragraph_setting, |r| {
        paragraph_setting(&r.paragraph)
    })
}

fn type_paragraph_style_font(view: &PictureView, name: &QString) -> QString {
    resolved_paragraph_style(view, name).map_or_else(QString::default, |r| {
        QString::from(r.character.font_family.as_str())
    })
}

fn replace(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    let previous = layer_spec(view.as_ref().get_ref(), path);
    let mut spec = spec(text, font, setting, character, paragraph);
    if let Some(previous) = previous {
        spec.overrides = edited_overrides(&previous, &spec);
    }
    !spec.text.trim().is_empty()
        && view
            .as_mut()
            .rust_mut()
            .session
            .doc
            .as_mut()
            .is_some_and(|doc| pictura_render::replace_type_layer(doc, &path.to_string(), &spec))
}

fn commit_edit(mut view: Pin<&mut PictureView>) {
    view.as_mut().edit("Edit Type Layer", |_| Edit::Structural);
}

/// Show the layer at `path` again after `type_edit_begin` hid it; no composite.
fn set_shown(mut view: Pin<&mut PictureView>, path: &QString) {
    if let Some(layer) = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .and_then(|doc| pictura_render::resolve_path_mut(doc, &path.to_string()))
    {
        layer.visible = true;
    }
}
