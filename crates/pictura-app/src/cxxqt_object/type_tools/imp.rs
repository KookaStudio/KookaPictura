use super::*;

use super::super::helpers::combine_mode_from;
use crate::cxxqt_object::edit::{structural, structural_after, structural_paths, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{CharacterAttrs, ParagraphAttrs, StyleOverrides, TextPathKind, TypeSpec};
use pictura_select::Selection;

use ffi::{CharacterSetting, CharacterStyleRow, ParagraphSetting, ParagraphStyleRow, TypeSetting};

use spec::{placement_setting, spec};

use attrs::{
    character_setting, edited_overrides, paragraph_attrs, paragraph_overrides, paragraph_setting,
    style_character_overrides,
};

pub(super) fn type_default_character_setting() -> CharacterSetting {
    character_setting(&CharacterAttrs::default())
}

pub(super) fn type_default_paragraph_setting() -> ParagraphSetting {
    paragraph_setting(&ParagraphAttrs::default())
}

pub(super) fn type_preview_rect(
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

pub(super) fn type_register_font(family: &QString, style: &QString, bytes: &[u8]) -> bool {
    pictura_render::register_font(&family.to_string(), &style.to_string(), bytes.to_vec())
}

pub(super) fn type_font_opentype_features(family: &QString, style: &QString) -> u32 {
    pictura_render::opentype_features(&family.to_string(), &style.to_string())
}

pub(super) fn type_caret_stops(
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

pub(super) fn type_baselines(
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

pub(super) fn type_overflows(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> bool {
    pictura_render::type_overflows(&spec(text, font, setting, character, paragraph))
}

pub(super) fn type_local_bounds(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_placement(&spec(text, font, setting, character, paragraph))
        .map_or_else(Vec::new, |p| p.bounds.to_vec())
}

pub(super) fn type_layer_is_paragraph(view: &PictureView, path: &QString) -> bool {
    layer_spec(view, path).is_some_and(|s| s.box_bounds.is_some())
}

pub(super) fn type_convert(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    paragraph: bool,
) -> bool {
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
    let path = path.to_string();
    let before = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, &path));
    let replaced = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| pictura_render::replace_type_layer(doc, &path, &spec));
    if replaced {
        let after = view
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        view.as_mut().edit(
            if paragraph {
                "Convert To Paragraph Text"
            } else {
                "Convert To Point Text"
            },
            |_| structural(before, after),
        );
    }
    replaced
}

pub(super) fn type_preview_rgba(
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

pub(super) fn type_commit_layer(
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
        let after = view
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &created));
        view.as_mut().edit(
            if spec.vertical {
                "Vertical Type"
            } else {
                "Horizontal Type"
            },
            |_| structural_after(after.map(|rect| vec![rect])),
        );
    }
    QString::from(created.as_str())
}

pub(super) fn type_commit_mask(
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
            data: data.into(),
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

pub(in crate::cxxqt_object) fn layer_spec(view: &PictureView, path: &QString) -> Option<TypeSpec> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::type_layer_spec(pictura_render::resolve_path(doc, &path.to_string())?)
}

pub(super) fn type_layer_at(view: &PictureView, x: f64, y: f64) -> QString {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::type_layer_at(doc, x, y))
        .map_or_else(QString::default, |path| QString::from(path.as_str()))
}

pub(super) fn type_layer_text(view: &PictureView, path: &QString) -> QString {
    layer_spec(view, path).map_or_else(QString::default, |s| QString::from(s.text.as_str()))
}

pub(super) fn type_path_at(view: &PictureView, x: f64, y: f64, tolerance: f64) -> Vec<f64> {
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

pub(super) fn type_path_overflow_point(
    text: &QString,
    font: &QString,
    setting: &TypeSetting,
    character: &CharacterSetting,
    paragraph: &ParagraphSetting,
) -> Vec<f64> {
    pictura_render::type_path_overflow_point(&spec(text, font, setting, character, paragraph))
        .map_or_else(Vec::new, |(x, y)| vec![x, y])
}

pub(super) fn type_layer_font(view: &PictureView, path: &QString) -> QString {
    layer_spec(view, path).map_or_else(QString::default, |s| {
        QString::from(s.character.font_family.as_str())
    })
}

pub(super) fn type_layer_setting(view: &PictureView, path: &QString) -> TypeSetting {
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

pub(super) fn type_layer_character_setting(view: &PictureView, path: &QString) -> CharacterSetting {
    layer_spec(view, path).map_or_else(type_default_character_setting, |s| {
        character_setting(&s.character)
    })
}

pub(super) fn type_layer_paragraph_setting(view: &PictureView, path: &QString) -> ParagraphSetting {
    layer_spec(view, path).map_or_else(type_default_paragraph_setting, |s| {
        paragraph_setting(&s.paragraph)
    })
}

pub(super) fn set_visible(mut view: Pin<&mut PictureView>, path: &QString, visible: bool) -> bool {
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

pub(super) fn type_edit_begin(view: Pin<&mut PictureView>, path: &QString) -> bool {
    set_visible(view, path, false)
}

pub(super) fn type_edit_cancel(view: Pin<&mut PictureView>, path: &QString) {
    set_visible(view, path, true);
}

pub(super) fn type_commit_edit(
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

pub(super) fn type_update_layer(
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

pub(super) fn type_preview_layer(
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
        view.as_mut().recomposite();
        view.as_mut().changed();
    }
    replaced
}

pub(super) fn type_apply_style(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    name: &QString,
    paragraph: bool,
) -> bool {
    let path = path.to_string();
    let before = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, &path));
    let applied = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| {
            pictura_render::apply_type_style(doc, &path, &name.to_string(), paragraph)
        });
    if applied {
        let after = view
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        view.as_mut()
            .edit("Apply Type Style", |_| structural(before, after));
    }
    applied
}

pub(in crate::cxxqt_object) fn type_paths(doc: &pictura_core::Document) -> Vec<String> {
    pictura_render::flatten_rows(doc)
        .into_iter()
        .filter(|(path, _)| {
            pictura_render::resolve_path(doc, path).is_some_and(|layer| layer.is_type())
        })
        .map(|(path, _)| path)
        .collect()
}

/// Run a style operation on the document and record one history state when it
/// changed anything.
pub(super) fn commit_style(
    mut view: Pin<&mut PictureView>,
    label: &str,
    apply: impl FnOnce(&mut pictura_core::Document) -> bool,
) -> bool {
    let before = view.rust().session.doc.as_ref().and_then(|doc| {
        let paths = type_paths(doc);
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        pictura_render::damage::influence_paths(doc, &refs)
    });
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(apply);
    if changed {
        let after = view.rust().session.doc.as_ref().and_then(|doc| {
            let paths = type_paths(doc);
            let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
            pictura_render::damage::influence_paths(doc, &refs)
        });
        view.as_mut()
            .edit(label, |_| structural_paths(before, after));
    }
    changed
}

pub(super) fn type_create_character_style(
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

pub(super) fn type_edit_character_style(
    view: Pin<&mut PictureView>,
    name: &QString,
    new_name: &QString,
    font: &QString,
    character: &CharacterSetting,
) -> bool {
    let name = name.to_string();
    let new_name = new_name.to_string();
    let attrs = style_character_overrides(font, character);
    commit_style(view, "Edit Character Style", |doc| {
        if name != new_name
            && crate::history::rename_character_style(doc, &name, &new_name).is_err()
        {
            return false;
        }
        crate::history::edit_character_style(doc, &new_name, attrs).is_ok()
    })
}

pub(super) fn type_preview_character_style(
    mut view: Pin<&mut PictureView>,
    name: &QString,
    font: &QString,
    character: &CharacterSetting,
) -> bool {
    let name = name.to_string();
    let attrs = style_character_overrides(font, character);
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| crate::history::edit_character_style(doc, &name, attrs).is_ok());
    if changed {
        view.as_mut().clear_link_sets();
        view.as_mut().recomposite();
        view.as_mut().changed();
    }
    changed
}

pub(super) fn type_delete_character_style(view: Pin<&mut PictureView>, name: &QString) -> bool {
    let name = name.to_string();
    commit_style(view, "Delete Character Style", |doc| {
        crate::history::delete_character_style(doc, &name).is_ok()
    })
}

pub(super) fn type_character_styles(view: &PictureView) -> Vec<CharacterStyleRow> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let sheet = &doc.text_styles;
    sheet
        .character_styles
        .iter()
        .map(|style| {
            let resolved =
                sheet.resolve(&StyleOverrides::default(), Some(style.name.as_str()), None);
            CharacterStyleRow {
                name: QString::from(style.name.as_str()),
                font: QString::from(resolved.character.font_family.as_str()),
                character: character_setting(&resolved.character),
            }
        })
        .collect()
}

pub(super) fn type_create_paragraph_style(
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

pub(super) fn type_edit_paragraph_style(
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

pub(super) fn type_delete_paragraph_style(view: Pin<&mut PictureView>, name: &QString) -> bool {
    let name = name.to_string();
    commit_style(view, "Delete Paragraph Style", |doc| {
        crate::history::delete_paragraph_style(doc, &name).is_ok()
    })
}

/// A live Preview edit: the style sheet and every applying layer change, the
/// display refreshes, and nothing is recorded; Cancel restores by previewing
/// the style's pre-dialog values back.
pub(super) fn type_preview_paragraph_style(
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

pub(super) fn type_paragraph_styles(view: &PictureView) -> Vec<ParagraphStyleRow> {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return Vec::new();
    };
    let sheet = &doc.text_styles;
    sheet
        .paragraph_styles
        .iter()
        .map(|style| {
            let resolved =
                sheet.resolve(&StyleOverrides::default(), None, Some(style.name.as_str()));
            ParagraphStyleRow {
                name: QString::from(style.name.as_str()),
                font: QString::from(resolved.character.font_family.as_str()),
                character: character_setting(&resolved.character),
                paragraph: paragraph_setting(&resolved.paragraph),
            }
        })
        .collect()
}

pub(super) fn replace(
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

pub(super) fn commit_edit(mut view: Pin<&mut PictureView>) {
    view.as_mut()
        .edit("Edit Type Layer", |_| Edit::Structural { damage: None });
}

/// Show the layer at `path` again after `type_edit_begin` hid it; no composite.
pub(super) fn set_shown(mut view: Pin<&mut PictureView>, path: &QString) {
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

#[cfg(test)]
mod header_tests {
    use std::path::Path;

    fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "tests") {
                    sources(&path, out);
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext == "cpp" || ext == "h")
            {
                out.push(path);
            }
        }
    }

    /// The generated type bridge headers stay inside `type_api` (#403): no
    /// other production header or source, other adapters included, pulls them
    /// in directly.
    #[test]
    fn type_bridge_headers_stay_in_adapter() {
        let cpp = Path::new(env!("CARGO_MANIFEST_DIR")).join("cpp");
        let mut files = Vec::new();
        sources(&cpp, &mut files);
        assert!(files.len() > 100, "the shell sources were found");
        let offenders: Vec<String> = files
            .iter()
            .filter(|path| {
                let name = path.file_name().unwrap().to_string_lossy();
                !(path.parent().unwrap().ends_with("api")
                    && (name == "type_api.h" || name == "type_api.cpp"))
            })
            .filter(|path| {
                std::fs::read_to_string(path).unwrap().lines().any(|line| {
                    line.trim_start().starts_with("#include")
                        && (line.contains("cxxqt_object/type_tools.cxxqt.h")
                            || line.contains("cxxqt_object/type_tools/"))
                })
            })
            .map(|path| path.display().to_string())
            .collect();
        assert!(
            offenders.is_empty(),
            "generated type headers included by {offenders:?}"
        );
    }
}
