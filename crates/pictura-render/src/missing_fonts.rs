//! `Type > Replace All Missing Fonts`: re-set the type layers whose font the
//! host does not have in the Type tool's default face. The host (Qt) decides
//! which names are missing; this module only substitutes.

use pictura_core::{Document, Layer};

/// The family missing fonts are replaced with: the Type tool's default, which
/// the engine bundles, so it is never itself missing.
pub const REPLACEMENT_FONT: &str = "Liberation Sans";
const REPLACEMENT_STYLE: &str = "Regular";

/// Re-set every type layer, in every group, whose font is one of `missing`, in
/// [`REPLACEMENT_FONT`], keeping its text and other attributes. Returns how
/// many layers were re-set; records no history.
pub fn replace_type_fonts(doc: &mut Document, missing: &[&str]) -> usize {
    fn walk(layers: &[Layer], prefix: &str, missing: &[&str], out: &mut Vec<String>) {
        for (i, layer) in layers.iter().enumerate() {
            let path = if prefix.is_empty() {
                i.to_string()
            } else {
                format!("{prefix}/{i}")
            };
            walk(&layer.children, &path, missing, out);
            let font = crate::type_layer_spec(layer).map(|s| s.character.font_family);
            if font.is_some_and(|f| missing.contains(&f.as_str())) {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    walk(&doc.layers, "", missing, &mut paths);
    paths
        .iter()
        .filter(|path| {
            let Some(mut spec) = crate::resolve_path(doc, path).and_then(crate::type_layer_spec)
            else {
                return false;
            };
            spec.character.font_family = REPLACEMENT_FONT.to_string();
            spec.character.font_style = REPLACEMENT_STYLE.to_string();
            // A manual font override would win over the character set when a
            // style re-resolves the run, so it follows the replacement.
            let overrides = &mut spec.overrides.character;
            if overrides.font_family.is_some() {
                overrides.font_family = Some(REPLACEMENT_FONT.to_string());
                overrides.font_style = Some(REPLACEMENT_STYLE.to_string());
            }
            crate::replace_type_layer(doc, path, &spec)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, TypeSpec};

    fn add(doc: &mut Document, text: &str, font: &str) -> String {
        let mut spec = TypeSpec::new(text, font, 24.0);
        spec.origin = (20.0, 40.0);
        crate::add_type_layer(doc, "", &spec)
    }

    fn font_and_text(doc: &Document, path: &str) -> (String, String) {
        let spec = crate::type_layer_spec(crate::resolve_path(doc, path).unwrap()).unwrap();
        (spec.character.font_family, spec.text)
    }

    #[test]
    fn replace_type_fonts_replaces_named_fonts() {
        let mut doc = Document::new(240, 120, ColorMode::Rgb, BitDepth::Eight);
        let missing = add(&mut doc, "Gone", "NoSuchFontPictura");
        add(&mut doc, "Kept", "OtherFontPictura");
        let group = crate::group_paths(&mut doc, &[missing.as_str()]).expect("grouped");
        let grouped = format!("{group}/0");
        let kept = doc
            .layers
            .iter()
            .position(Layer::is_type)
            .expect("the ungrouped layer")
            .to_string();
        let other_before = font_and_text(&doc, &kept);
        assert_eq!(font_and_text(&doc, &grouped).0, "NoSuchFontPictura");

        assert_eq!(replace_type_fonts(&mut doc, &["NoSuchFontPictura"]), 1);
        assert_eq!(
            font_and_text(&doc, &grouped),
            (REPLACEMENT_FONT.to_string(), "Gone".to_string())
        );
        assert_eq!(font_and_text(&doc, &kept), other_before);
        assert_eq!(replace_type_fonts(&mut doc, &["NoSuchFontPictura"]), 0);
    }
}
