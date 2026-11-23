use pictura_core::{
    CharacterOverrides, Document, LayerStack, NamedStyleAttrs, ParagraphOverrides, StyleError,
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
    pictura_render::edit_named_style(
        doc,
        pictura_render::NamedStyleEdit {
            from: name,
            to: name,
            attrs: NamedStyleAttrs::Character(attrs),
        },
    )
}

/// Rename a character style and rewrite every type layer's reference.
#[cfg(test)]
pub fn rename_character_style(doc: &mut Document, from: &str, to: &str) -> Result<(), StyleError> {
    let attrs = doc
        .text_styles
        .character_style(from)
        .map(|style| style.attrs.clone())
        .ok_or_else(|| StyleError::NotFound(from.to_string()))?;
    pictura_render::edit_named_style(
        doc,
        pictura_render::NamedStyleEdit {
            from,
            to,
            attrs: NamedStyleAttrs::Character(attrs),
        },
    )
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

/// Edit a named paragraph style, renaming it when `from != to`, and re-resolve
/// every type layer applying it.
pub fn edit_paragraph_style(
    doc: &mut Document,
    from: &str,
    to: &str,
    character: CharacterOverrides,
    paragraph: ParagraphOverrides,
) -> Result<(), StyleError> {
    pictura_render::edit_named_style(
        doc,
        pictura_render::NamedStyleEdit {
            from,
            to,
            attrs: NamedStyleAttrs::Paragraph {
                character,
                paragraph,
            },
        },
    )
}

/// Delete a named paragraph style and unlink the type layers applying it.
pub fn delete_paragraph_style(doc: &mut Document, name: &str) -> Result<(), StyleError> {
    doc.text_styles.delete_paragraph_style(name)?;
    clear_applied(&mut doc.layers, name, true);
    Ok(())
}

fn clear_applied(layers: &mut LayerStack, name: &str, paragraph: bool) {
    for layer in layers {
        if layer.is_group() {
            clear_applied(layer.children_mut().expect("group"), name, paragraph);
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
