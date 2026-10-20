use pictura_core::{
    CharacterOverrides, Document, Layer, LayerStack, ParagraphOverrides, StyleError,
};

/// Create a named character style. No history; the caller records one state.
pub fn create_character_style(
    doc: &mut Document,
    name: &str,
    attrs: CharacterOverrides,
) -> Result<(), StyleError> {
    doc.text_styles.create_character_style(name, attrs)
}

/// Edit a named character style and re-resolve every type layer applying it.
pub fn edit_character_style(
    doc: &mut Document,
    name: &str,
    attrs: CharacterOverrides,
) -> Result<(), StyleError> {
    doc.text_styles.edit_character_style(name, attrs)?;
    re_resolve_layers(doc, name, false);
    Ok(())
}

/// Delete a named character style and unlink the type layers applying it.
pub fn delete_character_style(doc: &mut Document, name: &str) -> Result<(), StyleError> {
    doc.text_styles.delete_character_style(name)?;
    clear_applied(&mut doc.layers, name, false);
    Ok(())
}

/// Create a named paragraph style. No history; the caller records one state.
pub fn create_paragraph_style(
    doc: &mut Document,
    name: &str,
    character: CharacterOverrides,
    paragraph: ParagraphOverrides,
) -> Result<(), StyleError> {
    doc.text_styles
        .create_paragraph_style(name, character, paragraph)
}

/// Edit a named paragraph style and re-resolve every type layer applying it.
pub fn edit_paragraph_style(
    doc: &mut Document,
    name: &str,
    character: CharacterOverrides,
    paragraph: ParagraphOverrides,
) -> Result<(), StyleError> {
    doc.text_styles
        .edit_paragraph_style(name, character, paragraph)?;
    re_resolve_layers(doc, name, true);
    Ok(())
}

/// Delete a named paragraph style and unlink the type layers applying it.
pub fn delete_paragraph_style(doc: &mut Document, name: &str) -> Result<(), StyleError> {
    doc.text_styles.delete_paragraph_style(name)?;
    clear_applied(&mut doc.layers, name, true);
    Ok(())
}

/// Re-resolve every type layer that applies `name`, preserving its manual
/// overrides, after an edit to that style.
fn re_resolve_layers(doc: &mut Document, name: &str, paragraph: bool) -> bool {
    let styles = doc.text_styles.clone();
    let mut paths = Vec::new();
    styled_layer_paths(&doc.layers, "", name, paragraph, &mut paths);
    let mut changed = false;
    for path in paths {
        let Some(mut spec) =
            pictura_render::resolve_path(doc, &path).and_then(pictura_render::type_layer_spec)
        else {
            continue;
        };
        let resolved = styles.resolve(
            &spec.overrides,
            spec.applied_character_style.as_deref(),
            spec.applied_paragraph_style.as_deref(),
        );
        spec.character = resolved.character;
        spec.paragraph = resolved.paragraph;
        changed |= pictura_render::replace_type_layer(doc, &path, &spec);
    }
    changed
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
        if layer.is_group {
            styled_layer_paths(&layer.children, &path, name, paragraph, out);
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

fn clear_applied(layers: &mut LayerStack, name: &str, paragraph: bool) {
    for layer in layers {
        if layer.is_group {
            clear_applied(&mut layer.children, name, paragraph);
            continue;
        }
        if paragraph {
            if layer.applied_paragraph_style.as_deref() == Some(name) {
                layer.applied_paragraph_style = None;
            }
        } else if layer.applied_character_style.as_deref() == Some(name) {
            layer.applied_character_style = None;
        }
    }
}
