//! The document-level named-style edit: one validated style-sheet change plus
//! the applied-reference rewrite and the type re-resolution it owns.

use pictura_core::{Document, Layer, LayerStack, NamedStyleAttrs, StyleError};

/// One named character or paragraph style edit. `from == to` writes attributes
/// in place; a different `to` renames the style first.
pub struct NamedStyleEdit<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub attrs: NamedStyleAttrs,
}

/// Apply `edit` as one validated document operation. On `Err` nothing changed;
/// on success the sheet, every applying layer's reference (groups included), and
/// the re-resolved type content changed together. A document whose type layers
/// cannot be re-set keeps its rendered content.
pub fn edit_named_style(doc: &mut Document, edit: NamedStyleEdit<'_>) -> Result<(), StyleError> {
    let NamedStyleEdit { from, to, attrs } = edit;
    let paragraph = matches!(&attrs, NamedStyleAttrs::Paragraph { .. });
    doc.text_styles.edit_style(from, to, attrs)?;
    if from != to {
        rename_applied(&mut doc.layers, from, to, paragraph);
    }
    if crate::type_layers_editable(doc) {
        let mut paths = Vec::new();
        if paragraph && to == pictura_core::BASIC_PARAGRAPH {
            applied_layer_paths(&doc.layers, "", &mut paths);
        } else {
            styled_layer_paths(&doc.layers, "", to, paragraph, &mut paths);
        }
        re_resolve_layers(doc, &paths);
    }
    Ok(())
}

/// Re-resolve the type layers at `paths`, preserving their manual overrides,
/// after an edit to the style they inherit or apply.
fn re_resolve_layers(doc: &mut Document, paths: &[String]) {
    let styles = doc.text_styles.clone();
    for path in paths {
        let Some(mut spec) = crate::resolve_path(doc, path).and_then(crate::type_layer_spec) else {
            continue;
        };
        let resolved = styles.resolve(
            &spec.overrides,
            spec.applied_character_style.as_deref(),
            spec.applied_paragraph_style.as_deref(),
        );
        spec.character = resolved.character;
        spec.paragraph = resolved.paragraph;
        crate::replace_type_layer(doc, path, &spec);
    }
}

fn styled_layer_paths(
    layers: &[Layer],
    prefix: &str,
    name: &str,
    paragraph: bool,
    out: &mut Vec<String>,
) {
    for (i, layer) in layers.iter().enumerate() {
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{prefix}/{i}")
        };
        if layer.is_group() {
            styled_layer_paths(layer.children(), &path, name, paragraph, out);
            continue;
        }
        let matches = if paragraph {
            layer.applied_paragraph_style.as_deref() == Some(name)
        } else {
            layer.applied_character_style.as_deref() == Some(name)
        };
        if matches {
            out.push(path);
        }
    }
}

/// Every leaf type layer that applies a character or paragraph style, groups
/// included. Such a layer inherits `Basic Paragraph` as its base, so editing the
/// default style can change it. A layer with no applied style is deliberately
/// excluded: its authored attributes are local formatting, not a style
/// reference, so re-resolving it would clobber them.
fn applied_layer_paths(layers: &[Layer], prefix: &str, out: &mut Vec<String>) {
    for (i, layer) in layers.iter().enumerate() {
        let path = if prefix.is_empty() {
            i.to_string()
        } else {
            format!("{prefix}/{i}")
        };
        if layer.is_group() {
            applied_layer_paths(layer.children(), &path, out);
            continue;
        }
        if layer.applied_paragraph_style.is_some() || layer.applied_character_style.is_some() {
            out.push(path);
        }
    }
}

fn rename_applied(layers: &mut LayerStack, from: &str, to: &str, paragraph: bool) {
    for layer in layers {
        if layer.is_group() {
            rename_applied(layer.children_mut().expect("group"), from, to, paragraph);
            continue;
        }
        if paragraph {
            if layer.applied_paragraph_style.as_deref() == Some(from) {
                layer.applied_paragraph_style = Some(to.to_string());
            }
        } else if layer.applied_character_style.as_deref() == Some(from) {
            layer.applied_character_style = Some(to.to_string());
        }
    }
}

#[cfg(test)]
mod tests;
