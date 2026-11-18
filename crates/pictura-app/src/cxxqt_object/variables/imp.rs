mod export;
mod import;

use super::super::{edit::Edit, helpers, qobject::PictureView, state::PictureViewRust};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use export::ExportBatch;
use import::{import_sets, Encoding, ImportOptions};
use pictura_core::layer_comp::{layer_id, layer_paths_by_id};
use pictura_core::variables::{Placement, ScaleMethod, Value, VariableKind};
use pictura_core::{Document, LayerStack, PsdRect};
use serde_json::{json, Value as Json};
use std::path::{Path, PathBuf};

#[cfg(test)]
use export::{export_sets, BatchResult};
#[cfg(test)]
mod io_tests;
#[cfg(test)]
mod tests;

pub(in crate::cxxqt_object) struct VariableSession {
    draft: Document,
    stamp: (u64, u64, Option<u64>),
}

pub(in crate::cxxqt_object) struct ExportSession {
    batch: ExportBatch,
    source_dir: Option<PathBuf>,
}

struct ReplacementImage {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

fn load_replacement(file: &str, folder: Option<&Path>) -> Result<ReplacementImage, String> {
    let reference = Path::new(file);
    let path = if reference.is_absolute() {
        reference.to_owned()
    } else if file.as_bytes().get(1) == Some(&b':') || file.starts_with("\\\\") {
        return Err(format!(
            "Choose a local file for the replacement image {file}"
        ));
    } else {
        folder.unwrap_or_else(|| Path::new(".")).join(reference)
    };
    let bytes = helpers::safe_io::read_capped(&path, helpers::safe_io::import_file_limit())
        .ok_or_else(|| format!("Cannot read replacement image {}", path.display()))?;
    if bytes.starts_with(b"8BPS") {
        let doc = pictura_codec::read_psd(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let size = u64::from(doc.width) * u64::from(doc.height) * 4;
        if size > helpers::safe_io::import_file_limit() {
            return Err("The replacement PSD exceeds the raster memory budget".into());
        }
        let frame = pictura_render::composite_rgba(&doc);
        let srgb = pictura_codec::buffer_to_srgb(&doc, &frame);
        return Ok(ReplacementImage {
            rgba: super::super::helpers_composite::buffer_to_rgba_bytes(&srgb),
            width: doc.width,
            height: doc.height,
        });
    }
    let (rgba, width, height) = helpers::decode_import(&bytes)
        .ok_or_else(|| format!("Cannot decode replacement image {}", path.display()))?;
    let rgba = super::super::open_jobs::convert_placed_rgba(
        &bytes,
        rgba,
        width,
        height,
        pictura_codec::Policy::Convert,
    );
    Ok(ReplacementImage {
        rgba,
        width,
        height,
    })
}

fn evaluate(
    doc: &Document,
    index: usize,
    load: impl Fn(&str) -> Result<ReplacementImage, String>,
) -> Result<Option<Document>, String> {
    if let Some(issue) = doc.variables.issues().first() {
        return Err(format!(
            "The data set contains unresolved Photoshop data: {issue}"
        ));
    }
    let data = doc.variables.sets().get(index).ok_or("Unknown data set")?;
    let paths = layer_paths_by_id(doc);
    let mut scratch = doc.clone();
    let mut changed = false;
    let unreadable = pictura_core::memory::eviction::unreadable_reads();
    for (name, variable) in doc.variables.definitions() {
        let value = data
            .values
            .get(&variable.id)
            .ok_or_else(|| format!("Variable {name} has no value"))?;
        if *value == Value::KeepPixels {
            continue;
        }
        for binding in &variable.bindings {
            changed |= (|| {
                let path = paths
                    .get(&binding.layer_id)
                    .ok_or("The bound layer no longer exists")?;
                let target = pictura_render::resolve_path(doc, path)
                    .filter(|target| variable.kind.accepts(target))
                    .ok_or("The bound layer does not support this edit")?;
                match value {
                    Value::Visibility(visible) if target.visible != *visible => {
                        pictura_render::resolve_path_mut(&mut scratch, path)
                            .ok_or("Missing target")?
                            .visible = *visible;
                        Ok(true)
                    }
                    Value::Visibility(_) | Value::KeepPixels => Ok(false),
                    Value::Text(text)
                        if target.type_tool().is_some_and(|tool| tool.text == *text) =>
                    {
                        Ok(false)
                    }
                    Value::Text(text) => {
                        if !pictura_render::replace_variable_text(&mut scratch, path, text) {
                            return Err(
                                "Text replacement is unsupported in this document mode or layer"
                                    .to_string(),
                            );
                        }
                        Ok(true)
                    }
                    Value::File(file) => {
                        let source = load(file)?;
                        let (rect, pixels) = pictura_render::place_pixels(
                            &source.rgba,
                            source.width,
                            source.height,
                            target.rect,
                            binding.placement,
                        )?;
                        pictura_render::replace_variable_pixels(&mut scratch, path, rect, &pixels)
                    }
                }
            })()
            .map_err(|message: String| format!("{name}: {message}"))?;
        }
    }
    if pictura_core::memory::eviction::unreadable_reads() != unreadable
        && crate::history::unreadable_tiles(&scratch) > 0
    {
        return Err("The document contains unreadable pixels".into());
    }
    if changed {
        scratch.layer_comps.mark_base_dirty();
    }
    Ok(changed.then_some(scratch))
}

fn install_evaluation(rust: &mut PictureViewRust, result: Option<Document>) -> Edit {
    match result {
        Some(doc) => {
            rust.session.doc.set(Some(doc));
            Edit::Full
        }
        None => Edit::None,
    }
}

fn install_template(rust: &mut PictureViewRust, draft: Document) -> Result<Edit, String> {
    let doc = rust.session.doc.as_mut().ok_or("No active document")?;
    if doc.variables == draft.variables {
        return Ok(Edit::None);
    }
    doc.variables = draft.variables;
    doc.layers = draft.layers;
    doc.layer_comps
        .set_next_layer_id(draft.layer_comps.next_layer_id());
    doc.layer_comps.ids_dirty |= draft.layer_comps.ids_dirty;
    Ok(Edit::Meta)
}

fn refresh_export_composite(doc: &mut Document) -> Result<(), String> {
    if crate::history::unreadable_tiles(doc) > 0 {
        return Err("The document contains unreadable pixels".into());
    }
    pictura_render::DocumentComposite::new(doc)
        .refresh(pictura_render::CompositeScope::Full, false);
    pictura_render::refresh_native_composite(doc);
    if let std::borrow::Cow::Owned(prepared) = pictura_render::save_view(doc) {
        *doc = prepared;
    }
    Ok(())
}

fn blocked(rust: &PictureViewRust) -> bool {
    rust.has_active_session()
        || rust.pending_previews.any()
        || rust.display_preview_active()
        || rust.mask_solo.is_some()
}

pub(in crate::cxxqt_object) fn variables_available(view: &PictureView, group: &QString) -> bool {
    let rust = view.rust();
    !rust.session.read_only()
        && !blocked(rust)
        && rust
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| match group.to_string().as_str() {
                "variables" => !doc.variables.definitions().is_empty(),
                "sets" => !doc.variables.sets().is_empty(),
                _ => true,
            })
}

fn stamp(rust: &PictureViewRust) -> (u64, u64, Option<u64>) {
    (
        rust.session.document_generation,
        rust.session.content_revision,
        rust.session.history.current_id(),
    )
}

fn draft(rust: &PictureViewRust, use_draft: bool) -> Option<&Document> {
    if use_draft {
        rust.variables_dialog
            .as_ref()
            .filter(|edit| edit.stamp == stamp(rust))
            .map(|edit| &edit.draft)
    } else {
        rust.session.doc.as_ref()
    }
}

fn kind_name(kind: VariableKind) -> &'static str {
    match kind {
        VariableKind::Visibility => "visibility",
        VariableKind::Text => "text",
        VariableKind::Pixels => "pixels",
    }
}

fn mode_name(method: ScaleMethod) -> &'static str {
    match method {
        ScaleMethod::Fit => "fit",
        ScaleMethod::Fill => "fill",
        ScaleMethod::AsIs => "asIs",
        ScaleMethod::Conform => "conform",
    }
}

fn state_layers(stack: &LayerStack, prefix: &str, out: &mut Vec<Json>) {
    for (index, layer) in stack.iter().enumerate().rev() {
        let path = if prefix.is_empty() {
            index.to_string()
        } else {
            format!("{prefix}/{index}")
        };
        let kinds = [
            VariableKind::Visibility,
            VariableKind::Text,
            VariableKind::Pixels,
        ];
        out.push(json!({"path": path, "id": layer_id(layer), "name": &layer.name,
            "allowed": kinds.into_iter().filter(|kind| kind.accepts(layer)).map(kind_name).collect::<Vec<_>>() }));
        state_layers(layer.children(), &path, out);
    }
}

pub(in crate::cxxqt_object) fn variables_state(view: &PictureView, use_draft: bool) -> QString {
    let rust = view.rust();
    let doc = draft(rust, use_draft);
    let mut layers = Vec::new();
    if let Some(doc) = doc {
        state_layers(&doc.layers, "", &mut layers);
    }
    let definitions: Vec<_> = doc.into_iter().flat_map(|doc| doc.variables.definitions()).map(|(name, variable)| {
        let bindings: Vec<_> = variable.bindings.iter().map(|binding| {
            let p = binding.placement;
            json!({"id": binding.layer_id, "method": mode_name(p.method), "horizontal": p.horizontal, "vertical": p.vertical, "clip": p.clip})
        }).collect();
        json!({"name": name, "kind": kind_name(variable.kind), "bindings": bindings})
    }).collect();
    let sets: Vec<_> = doc
        .into_iter()
        .flat_map(|doc| {
            doc.variables.sets().iter().map(|set| {
                let values: serde_json::Map<String, Json> = doc
                    .variables
                    .definitions()
                    .iter()
                    .filter_map(|(name, var)| {
                        let value = match set.values.get(&var.id)? {
                            Value::Visibility(value) => json!(value),
                            Value::Text(text) | Value::File(text) => json!(text),
                            Value::KeepPixels => Json::Null,
                        };
                        Some((name.clone(), value))
                    })
                    .collect();
                json!({"name": &set.name, "values": values})
            })
        })
        .collect();
    let issues = doc.map_or(&[][..], |doc| doc.variables.issues());
    QString::from(
        json!({"layers": layers, "variables": definitions, "sets": sets,
        "issues": issues, "ready": doc.is_some() && !blocked(rust)})
        .to_string()
        .as_str(),
    )
}

fn text<'a>(input: &'a Json, key: &str) -> &'a str {
    input[key].as_str().unwrap_or_default()
}

fn index(input: &Json, key: &str) -> Result<usize, String> {
    input[key]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("Invalid {key}"))
}

fn placement(input: &Json) -> Result<Placement, String> {
    let method = match text(input, "method") {
        "" | "fit" => ScaleMethod::Fit,
        "fill" => ScaleMethod::Fill,
        "asIs" => ScaleMethod::AsIs,
        "conform" => ScaleMethod::Conform,
        _ => return Err("Unknown placement method".into()),
    };
    let position = |key: &str| {
        input[key]
            .as_u64()
            .map_or(Some(1), |n| u8::try_from(n).ok())
            .ok_or("Invalid alignment")
    };
    Ok(Placement {
        method,
        horizontal: position("horizontal")?,
        vertical: position("vertical")?,
        clip: input["clip"].as_bool().unwrap_or(false),
    })
}

fn options(input: &Json) -> Result<ImportOptions, String> {
    let encoding = match text(input, "encoding") {
        "" | "auto" => Encoding::Automatic,
        "utf8" => Encoding::Utf8,
        "utf16le" => Encoding::Utf16Le,
        "utf16be" => Encoding::Utf16Be,
        "windows1252" => Encoding::Windows1252,
        _ => return Err("Unknown data set encoding".into()),
    };
    Ok(ImportOptions {
        encoding,
        first_names: input["firstNames"].as_bool().unwrap_or(false),
        replace: input["replace"].as_bool().unwrap_or(false),
    })
}

fn mutate_template(doc: &mut Document, action: &str, input: &Json) -> Result<(), String> {
    let name = text(input, "name");
    match action {
        "bind" | "unbind" => {
            let kind = match text(input, "kind") {
                "visibility" => VariableKind::Visibility,
                "text" => VariableKind::Text,
                "pixels" => VariableKind::Pixels,
                _ => return Err("Invalid variable type".into()),
            };
            let path = text(input, "path")
                .split('/')
                .map(str::parse::<usize>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "Invalid layer path")?;
            if action == "bind" {
                doc.bind_variable(&path, name, kind, placement(input)?)
            } else {
                doc.unbind_variable(&path, kind)
            }
        }
        "renameVariable" => doc.rename_variable(text(input, "from"), name),
        "newSet" => doc
            .new_data_set(
                input["copy"].as_u64().and_then(|n| usize::try_from(n).ok()),
                name,
            )
            .map(|_| ()),
        "renameSet" => doc.variables.rename_set(index(input, "set")?, name),
        "deleteSet" => doc.variables.delete_set(index(input, "set")?),
        "setValue" => {
            let variable = doc
                .variables
                .definitions()
                .get(name)
                .ok_or("Unknown variable")?;
            let value = match variable.kind {
                VariableKind::Visibility => Value::Visibility(
                    input["value"]
                        .as_bool()
                        .ok_or("Expected a visibility value")?,
                ),
                VariableKind::Text => {
                    Value::Text(input["value"].as_str().ok_or("Expected text")?.into())
                }
                VariableKind::Pixels if input["value"].is_null() => Value::KeepPixels,
                VariableKind::Pixels => Value::File(
                    input["value"]
                        .as_str()
                        .ok_or("Expected an image path")?
                        .into(),
                ),
            };
            doc.set_data_value(index(input, "set")?, name, value)
        }
        "import" => import_sets(doc, Path::new(text(input, "path")), options(input)?).map(|_| ()),
        _ => Err("Unknown template edit".into()),
    }
}

fn source_dir(rust: &PictureViewRust) -> Option<&Path> {
    rust.path
        .as_deref()
        .and_then(|path| Path::new(path).parent())
}

fn execute(mut view: Pin<&mut PictureView>, action: &str, input: &Json) -> Result<String, String> {
    if action == "cancel" {
        view.as_mut().rust_mut().variables_dialog = None;
        return Ok(String::new());
    }
    if action == "exportEnd" {
        view.as_mut().rust_mut().variable_export = None;
        return Ok(String::new());
    }
    if view.rust().session.read_only() {
        return Err("This document is read-only".into());
    }
    if blocked(view.rust()) {
        return Err("Finish the current document edit first".into());
    }
    match action {
        "begin" => {
            let rust = view.rust();
            let draft = rust.session.doc.snapshot().ok_or("No active document")?;
            let session = VariableSession {
                draft,
                stamp: stamp(rust),
            };
            view.as_mut().rust_mut().variables_dialog = Some(Box::new(session));
        }
        "commit" => {
            let session = view
                .as_mut()
                .rust_mut()
                .variables_dialog
                .take()
                .ok_or("No open Variables dialog")?;
            if session.stamp != stamp(view.rust()) {
                return Err("The document changed while the Variables dialog was open".into());
            }
            view.as_mut().edit("Variables", |rust| {
                install_template(rust, session.draft).unwrap_or(Edit::None)
            });
        }
        "importLive" => {
            let mut next = view
                .rust()
                .session
                .doc
                .snapshot()
                .ok_or("No active document")?;
            if import_sets(&mut next, Path::new(text(input, "path")), options(input)?)? {
                view.as_mut().edit("Import Variable Data Sets", |rust| {
                    install_template(rust, next).unwrap_or(Edit::None)
                });
            }
        }
        "apply" => {
            let rust = view.rust();
            let doc = rust.session.doc.as_ref().ok_or("No active document")?;
            let index = index(input, "set")?;
            let name = &doc
                .variables
                .sets()
                .get(index)
                .ok_or("Unknown data set")?
                .name;
            let label = format!("Apply Data Set: {name}");
            let next = evaluate(doc, index, |file| load_replacement(file, source_dir(rust)))?;
            if next.is_some() {
                view.as_mut()
                    .edit(&label, |rust| install_evaluation(rust, next));
            }
        }
        "exportBegin" => {
            let rust = view.rust();
            let doc = rust.session.doc.as_ref().ok_or("No active document")?;
            let selected = input["selected"]
                .as_array()
                .ok_or("Choose data sets")?
                .iter()
                .map(|n| {
                    n.as_u64()
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or("Invalid selected data set")
                })
                .collect::<Result<Vec<_>, _>>()?;
            let batch = ExportBatch::new(
                doc,
                Path::new(text(input, "folder")),
                &selected,
                text(input, "base"),
                text(input, "pattern"),
            )?;
            let session = ExportSession {
                batch,
                source_dir: source_dir(rust).map(Path::to_owned),
            };
            view.as_mut().rust_mut().variable_export = Some(Box::new(session));
        }
        "exportStep" => {
            let mut rust = view.as_mut().rust_mut();
            let session = rust.variable_export.as_mut().ok_or("No active export")?;
            let folder = session.source_dir.as_deref();
            session.batch.step(|file| load_replacement(file, folder));
            let batch = &session.batch;
            return Ok(json!({"progress": batch.cursor, "done": batch.done(), "skipped": batch.result.skipped,
                "failure": batch.result.failure, "written": batch.result.written.len()}).to_string());
        }
        _ => {
            let actual = stamp(view.rust());
            let mut rust = view.as_mut().rust_mut();
            let session = rust
                .variables_dialog
                .as_mut()
                .filter(|edit| edit.stamp == actual)
                .ok_or("No current Variables dialog")?;
            mutate_template(&mut session.draft, action, input)?;
        }
    }
    Ok(String::new())
}

pub(in crate::cxxqt_object) fn variables_command(
    view: Pin<&mut PictureView>,
    action: &QString,
    payload: &QString,
) -> QString {
    let result = serde_json::from_str::<Json>(&payload.to_string())
        .map_err(|e| e.to_string())
        .and_then(|input| execute(view, &action.to_string(), &input));
    QString::from(result.unwrap_or_else(|error| error).as_str())
}

pub(in crate::cxxqt_object) fn variables_preview(view: &PictureView, payload: &QString) -> Vec<u8> {
    let rust = view.rust();
    let make = || -> Result<Vec<u8>, String> {
        if blocked(rust) {
            return Err("Finish the current document edit first".into());
        }
        let input: Json =
            serde_json::from_str(&payload.to_string()).map_err(|error| error.to_string())?;
        let doc =
            draft(rust, input["draft"].as_bool().unwrap_or(false)).ok_or("No active template")?;
        let changed = evaluate(doc, index(&input, "set")?, |file| {
            load_replacement(file, source_dir(rust))
        })?;
        let doc = changed.as_ref().unwrap_or(doc);
        let (mut width, mut height, mut level) = (doc.width, doc.height, 0);
        while width.max(height) > 1536 {
            width = width.div_ceil(2);
            height = height.div_ceil(2);
            level += 1;
        }
        let rect = PsdRect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        let frame = pictura_render::composite_region_level(doc, rect, level, &|chunk| {
            pictura_codec::buffer_to_srgb(doc, &chunk).into_owned()
        });
        let mut bytes = Vec::with_capacity(8 + frame.data().len());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(frame.data());
        Ok(bytes)
    };
    make().unwrap_or_else(|error| {
        let mut result = vec![0; 8];
        result.extend_from_slice(error.as_bytes());
        result
    })
}
