//! The Type bridge's attribute plumbing: converting between the generated
//! `CharacterSetting`/`ParagraphSetting` transport structs and the model's
//! `CharacterAttrs`/`ParagraphAttrs`, and the override bookkeeping the style
//! and manual-edit paths share.

use super::ffi::{CharacterSetting, ParagraphSetting};
use cxx_qt_lib::QString;
use pictura_core::{
    AntiAlias, CharacterAttrs, CharacterOverrides, Composer, Justify, KerningMode, Leading,
    ParagraphAttrs, ParagraphOverrides, StyleOverrides, TypeSpec,
};

fn rgba_color(color: u32) -> [f64; 4] {
    let [a, r, g, b] = color.to_be_bytes();
    [
        f64::from(a) / 255.0,
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    ]
}

pub(super) fn character_attrs(setting: &CharacterSetting) -> CharacterAttrs {
    CharacterAttrs {
        font_style: setting.font_style.to_string(),
        size: setting.size,
        leading: if setting.leading_mode == 1 {
            Leading::Fixed(setting.leading_value)
        } else {
            Leading::Auto
        },
        kerning: match setting.kerning_mode {
            1 => KerningMode::Optical,
            2 => KerningMode::Manual(setting.kerning_value),
            _ => KerningMode::Metrics,
        },
        tracking: setting.tracking,
        horizontal_scale: setting.horizontal_scale,
        vertical_scale: setting.vertical_scale,
        baseline_shift: setting.baseline_shift,
        anti_alias: AntiAlias::from_index(setting.anti_alias),
        fill_color: rgba_color(setting.color),
        all_caps: setting.all_caps,
        small_caps: setting.small_caps,
        superscript: setting.superscript,
        subscript: setting.subscript,
        underline: setting.underline,
        strikethrough: setting.strikethrough,
        fractional_widths: setting.fractional_widths,
        faux_bold: setting.faux_bold,
        faux_italic: setting.faux_italic,
        standard_ligatures: setting.standard_ligatures,
        contextual_alternates: setting.contextual_alternates,
        discretionary_ligatures: setting.discretionary_ligatures,
        swash: setting.swash,
        oldstyle: setting.oldstyle,
        stylistic_alternates: setting.stylistic_alternates,
        titling_alternates: setting.titling_alternates,
        ornaments: setting.ornaments,
        ordinals: setting.ordinals,
        fractions: setting.fractions,
        language: setting.language.to_string(),
        vertical_roman_alignment: setting.vertical_roman_alignment,
        ..CharacterAttrs::default()
    }
}

pub(super) fn paragraph_attrs(setting: &ParagraphSetting) -> ParagraphAttrs {
    ParagraphAttrs {
        justify: Justify::from_index(setting.justify.clamp(0, 6) as u8).unwrap_or_default(),
        word_spacing: [
            setting.word_spacing_min,
            setting.word_spacing_desired,
            setting.word_spacing_max,
        ],
        letter_spacing: [
            setting.letter_spacing_min,
            setting.letter_spacing_desired,
            setting.letter_spacing_max,
        ],
        glyph_spacing: [
            setting.glyph_spacing_min,
            setting.glyph_spacing_desired,
            setting.glyph_spacing_max,
        ],
        start_indent: setting.start_indent,
        end_indent: setting.end_indent,
        first_line_indent: setting.first_line_indent,
        space_before: setting.space_before,
        space_after: setting.space_after,
        hanging: setting.hanging,
        hyphenate: setting.hyphenate,
        composer: if setting.composer == 1 {
            Composer::EveryLine
        } else {
            Composer::SingleLine
        },
        auto_leading: setting.auto_leading,
        hyphenate_word_size: setting.hyphenate_word_size,
        hyphenate_pre: setting.hyphenate_pre,
        hyphenate_post: setting.hyphenate_post,
        hyphen_limit: setting.hyphen_limit,
        hyphenation_zone: setting.hyphenation_zone,
        hyphenate_caps: setting.hyphenate_caps,
    }
}

pub(super) fn character_setting(attrs: &CharacterAttrs) -> CharacterSetting {
    let [r, g, b, a] = attrs.rgba();
    CharacterSetting {
        font_style: QString::from(attrs.font_style.as_str()),
        size: attrs.size,
        leading_mode: match attrs.leading {
            Leading::Auto => 0,
            Leading::Fixed(_) => 1,
        },
        leading_value: match attrs.leading {
            Leading::Fixed(value) => value,
            Leading::Auto => 0.0,
        },
        kerning_mode: match attrs.kerning {
            KerningMode::Metrics => 0,
            KerningMode::Optical => 1,
            KerningMode::Manual(_) => 2,
        },
        kerning_value: match attrs.kerning {
            KerningMode::Manual(value) => value,
            _ => 0,
        },
        tracking: attrs.tracking,
        horizontal_scale: attrs.horizontal_scale,
        vertical_scale: attrs.vertical_scale,
        baseline_shift: attrs.baseline_shift,
        anti_alias: attrs.anti_alias.index(),
        color: u32::from_be_bytes([a, r, g, b]),
        all_caps: attrs.all_caps,
        small_caps: attrs.small_caps,
        superscript: attrs.superscript,
        subscript: attrs.subscript,
        underline: attrs.underline,
        strikethrough: attrs.strikethrough,
        fractional_widths: attrs.fractional_widths,
        faux_bold: attrs.faux_bold,
        faux_italic: attrs.faux_italic,
        standard_ligatures: attrs.standard_ligatures,
        contextual_alternates: attrs.contextual_alternates,
        discretionary_ligatures: attrs.discretionary_ligatures,
        swash: attrs.swash,
        oldstyle: attrs.oldstyle,
        stylistic_alternates: attrs.stylistic_alternates,
        titling_alternates: attrs.titling_alternates,
        ornaments: attrs.ornaments,
        ordinals: attrs.ordinals,
        fractions: attrs.fractions,
        language: QString::from(attrs.language.as_str()),
        vertical_roman_alignment: attrs.vertical_roman_alignment,
    }
}

pub(super) fn paragraph_setting(attrs: &ParagraphAttrs) -> ParagraphSetting {
    ParagraphSetting {
        justify: i32::from(attrs.justify.index()),
        word_spacing_min: attrs.word_spacing[0],
        word_spacing_desired: attrs.word_spacing[1],
        word_spacing_max: attrs.word_spacing[2],
        letter_spacing_min: attrs.letter_spacing[0],
        letter_spacing_desired: attrs.letter_spacing[1],
        letter_spacing_max: attrs.letter_spacing[2],
        glyph_spacing_min: attrs.glyph_spacing[0],
        glyph_spacing_desired: attrs.glyph_spacing[1],
        glyph_spacing_max: attrs.glyph_spacing[2],
        start_indent: attrs.start_indent,
        end_indent: attrs.end_indent,
        first_line_indent: attrs.first_line_indent,
        space_before: attrs.space_before,
        space_after: attrs.space_after,
        hanging: attrs.hanging,
        hyphenate: attrs.hyphenate,
        // Kept on the bridge struct so the generated C++ header is unchanged;
        // the model no longer carries a paragraph direction.
        direction: 0,
        composer: match attrs.composer {
            Composer::SingleLine => 0,
            Composer::EveryLine => 1,
        },
        auto_leading: attrs.auto_leading,
        hyphenate_word_size: attrs.hyphenate_word_size,
        hyphenate_pre: attrs.hyphenate_pre,
        hyphenate_post: attrs.hyphenate_post,
        hyphen_limit: attrs.hyphen_limit,
        hyphenation_zone: attrs.hyphenation_zone,
        hyphenate_caps: attrs.hyphenate_caps,
    }
}

fn character_overrides(attrs: &CharacterAttrs) -> CharacterOverrides {
    CharacterOverrides {
        font_family: Some(attrs.font_family.clone()),
        font_style: Some(attrs.font_style.clone()),
        size: Some(attrs.size),
        leading: Some(attrs.leading),
        kerning: Some(attrs.kerning),
        tracking: Some(attrs.tracking),
        horizontal_scale: Some(attrs.horizontal_scale),
        vertical_scale: Some(attrs.vertical_scale),
        baseline_shift: Some(attrs.baseline_shift),
        anti_alias: Some(attrs.anti_alias),
        fill_color: Some(attrs.fill_color),
        all_caps: Some(attrs.all_caps),
        small_caps: Some(attrs.small_caps),
        superscript: Some(attrs.superscript),
        subscript: Some(attrs.subscript),
        underline: Some(attrs.underline),
        strikethrough: Some(attrs.strikethrough),
        fractional_widths: Some(attrs.fractional_widths),
        faux_bold: Some(attrs.faux_bold),
        faux_italic: Some(attrs.faux_italic),
        standard_ligatures: Some(attrs.standard_ligatures),
        contextual_alternates: Some(attrs.contextual_alternates),
        discretionary_ligatures: Some(attrs.discretionary_ligatures),
        swash: Some(attrs.swash),
        oldstyle: Some(attrs.oldstyle),
        stylistic_alternates: Some(attrs.stylistic_alternates),
        titling_alternates: Some(attrs.titling_alternates),
        ornaments: Some(attrs.ornaments),
        ordinals: Some(attrs.ordinals),
        fractions: Some(attrs.fractions),
        language: Some(attrs.language.clone()),
        vertical_roman_alignment: Some(attrs.vertical_roman_alignment),
    }
}

pub(super) fn paragraph_overrides(attrs: &ParagraphAttrs) -> ParagraphOverrides {
    ParagraphOverrides {
        justify: Some(attrs.justify),
        word_spacing: Some(attrs.word_spacing),
        letter_spacing: Some(attrs.letter_spacing),
        glyph_spacing: Some(attrs.glyph_spacing),
        start_indent: Some(attrs.start_indent),
        end_indent: Some(attrs.end_indent),
        first_line_indent: Some(attrs.first_line_indent),
        space_before: Some(attrs.space_before),
        space_after: Some(attrs.space_after),
        hanging: Some(attrs.hanging),
        hyphenate: Some(attrs.hyphenate),
        composer: Some(attrs.composer),
        auto_leading: Some(attrs.auto_leading),
        hyphenate_word_size: Some(attrs.hyphenate_word_size),
        hyphenate_pre: Some(attrs.hyphenate_pre),
        hyphenate_post: Some(attrs.hyphenate_post),
        hyphen_limit: Some(attrs.hyphen_limit),
        hyphenation_zone: Some(attrs.hyphenation_zone),
        hyphenate_caps: Some(attrs.hyphenate_caps),
    }
}

/// The character style's sparse overrides, with the bridge's separate family.
pub(super) fn style_character_overrides(
    font: &QString,
    character: &CharacterSetting,
) -> CharacterOverrides {
    let mut attrs = character_attrs(character);
    attrs.font_family = font.to_string();
    character_overrides(&attrs)
}

/// Record `after` as a manual override for one field only when it differs from
/// the layer's `before`; an unchanged field stays inherited from the styles.
fn changed<T: Clone + PartialEq>(current: &mut Option<T>, before: &T, after: &T) {
    if after != before {
        *current = Some(after.clone());
    }
}

/// The run's manual overrides after a bridge edit: the layer's existing
/// overrides plus every attribute that changed from its previous effective
/// value. Unchanged fields stay unset, so a later style application still
/// updates them.
pub(super) fn edited_overrides(previous: &TypeSpec, next: &TypeSpec) -> StyleOverrides {
    let mut overrides = previous.overrides.clone();
    let (before, after) = (&previous.character, &next.character);
    let c = &mut overrides.character;
    changed(&mut c.font_family, &before.font_family, &after.font_family);
    changed(&mut c.font_style, &before.font_style, &after.font_style);
    changed(&mut c.size, &before.size, &after.size);
    changed(&mut c.leading, &before.leading, &after.leading);
    changed(&mut c.kerning, &before.kerning, &after.kerning);
    changed(&mut c.tracking, &before.tracking, &after.tracking);
    changed(
        &mut c.horizontal_scale,
        &before.horizontal_scale,
        &after.horizontal_scale,
    );
    changed(
        &mut c.vertical_scale,
        &before.vertical_scale,
        &after.vertical_scale,
    );
    changed(
        &mut c.baseline_shift,
        &before.baseline_shift,
        &after.baseline_shift,
    );
    changed(&mut c.anti_alias, &before.anti_alias, &after.anti_alias);
    changed(&mut c.fill_color, &before.fill_color, &after.fill_color);
    changed(&mut c.all_caps, &before.all_caps, &after.all_caps);
    changed(&mut c.small_caps, &before.small_caps, &after.small_caps);
    changed(&mut c.superscript, &before.superscript, &after.superscript);
    changed(&mut c.subscript, &before.subscript, &after.subscript);
    changed(&mut c.underline, &before.underline, &after.underline);
    changed(
        &mut c.strikethrough,
        &before.strikethrough,
        &after.strikethrough,
    );
    changed(
        &mut c.fractional_widths,
        &before.fractional_widths,
        &after.fractional_widths,
    );
    changed(&mut c.faux_bold, &before.faux_bold, &after.faux_bold);
    changed(&mut c.faux_italic, &before.faux_italic, &after.faux_italic);
    changed(
        &mut c.standard_ligatures,
        &before.standard_ligatures,
        &after.standard_ligatures,
    );
    changed(
        &mut c.contextual_alternates,
        &before.contextual_alternates,
        &after.contextual_alternates,
    );
    changed(
        &mut c.discretionary_ligatures,
        &before.discretionary_ligatures,
        &after.discretionary_ligatures,
    );
    changed(&mut c.swash, &before.swash, &after.swash);
    changed(&mut c.oldstyle, &before.oldstyle, &after.oldstyle);
    changed(
        &mut c.stylistic_alternates,
        &before.stylistic_alternates,
        &after.stylistic_alternates,
    );
    changed(
        &mut c.titling_alternates,
        &before.titling_alternates,
        &after.titling_alternates,
    );
    changed(&mut c.ornaments, &before.ornaments, &after.ornaments);
    changed(&mut c.ordinals, &before.ordinals, &after.ordinals);
    changed(&mut c.fractions, &before.fractions, &after.fractions);
    changed(&mut c.language, &before.language, &after.language);
    changed(
        &mut c.vertical_roman_alignment,
        &before.vertical_roman_alignment,
        &after.vertical_roman_alignment,
    );

    let (before, after) = (&previous.paragraph, &next.paragraph);
    let p = &mut overrides.paragraph;
    changed(&mut p.justify, &before.justify, &after.justify);
    changed(
        &mut p.word_spacing,
        &before.word_spacing,
        &after.word_spacing,
    );
    changed(
        &mut p.letter_spacing,
        &before.letter_spacing,
        &after.letter_spacing,
    );
    changed(
        &mut p.glyph_spacing,
        &before.glyph_spacing,
        &after.glyph_spacing,
    );
    changed(
        &mut p.start_indent,
        &before.start_indent,
        &after.start_indent,
    );
    changed(&mut p.end_indent, &before.end_indent, &after.end_indent);
    changed(
        &mut p.first_line_indent,
        &before.first_line_indent,
        &after.first_line_indent,
    );
    changed(
        &mut p.space_before,
        &before.space_before,
        &after.space_before,
    );
    changed(&mut p.space_after, &before.space_after, &after.space_after);
    changed(&mut p.hanging, &before.hanging, &after.hanging);
    changed(&mut p.hyphenate, &before.hyphenate, &after.hyphenate);
    changed(&mut p.composer, &before.composer, &after.composer);
    changed(
        &mut p.auto_leading,
        &before.auto_leading,
        &after.auto_leading,
    );
    changed(
        &mut p.hyphenate_word_size,
        &before.hyphenate_word_size,
        &after.hyphenate_word_size,
    );
    changed(
        &mut p.hyphenate_pre,
        &before.hyphenate_pre,
        &after.hyphenate_pre,
    );
    changed(
        &mut p.hyphenate_post,
        &before.hyphenate_post,
        &after.hyphenate_post,
    );
    changed(
        &mut p.hyphen_limit,
        &before.hyphen_limit,
        &after.hyphen_limit,
    );
    changed(
        &mut p.hyphenation_zone,
        &before.hyphenation_zone,
        &after.hyphenation_zone,
    );
    changed(
        &mut p.hyphenate_caps,
        &before.hyphenate_caps,
        &after.hyphenate_caps,
    );
    overrides
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_default_setting() -> CharacterSetting {
        CharacterSetting {
            font_style: QString::from("Bold"),
            size: 48.0,
            leading_mode: 1,
            leading_value: 60.0,
            kerning_mode: 2,
            kerning_value: -25,
            tracking: 50.0,
            horizontal_scale: 120.0,
            vertical_scale: 80.0,
            baseline_shift: 3.0,
            anti_alias: 3,
            color: 0x80ff_4020,
            all_caps: true,
            small_caps: false,
            superscript: false,
            subscript: true,
            underline: true,
            strikethrough: true,
            fractional_widths: false,
            faux_bold: true,
            faux_italic: true,
            standard_ligatures: false,
            contextual_alternates: false,
            discretionary_ligatures: true,
            swash: true,
            oldstyle: true,
            stylistic_alternates: true,
            titling_alternates: true,
            ornaments: true,
            ordinals: true,
            fractions: true,
            language: QString::from("German: 2006 Reform"),
            vertical_roman_alignment: false,
        }
    }

    #[test]
    fn character_setting_round_trips_every_field() {
        let a = non_default_setting();
        let b = character_setting(&character_attrs(&a));
        assert_eq!(a.font_style.to_string(), b.font_style.to_string());
        assert_eq!(a.language.to_string(), b.language.to_string());
        assert_eq!(
            (a.size, a.leading_mode, a.leading_value, a.kerning_mode),
            (b.size, b.leading_mode, b.leading_value, b.kerning_mode)
        );
        assert_eq!(
            (a.kerning_value, a.tracking, a.anti_alias, a.color),
            (b.kerning_value, b.tracking, b.anti_alias, b.color)
        );
        assert_eq!(
            (a.horizontal_scale, a.vertical_scale, a.baseline_shift),
            (b.horizontal_scale, b.vertical_scale, b.baseline_shift)
        );
        let toggles = |s: &CharacterSetting| {
            [
                s.all_caps,
                s.small_caps,
                s.superscript,
                s.subscript,
                s.underline,
                s.strikethrough,
                s.fractional_widths,
                s.faux_bold,
                s.faux_italic,
                s.standard_ligatures,
                s.contextual_alternates,
                s.discretionary_ligatures,
                s.swash,
                s.oldstyle,
                s.stylistic_alternates,
                s.titling_alternates,
                s.ornaments,
                s.ordinals,
                s.fractions,
                s.vertical_roman_alignment,
            ]
        };
        assert_eq!(toggles(&a), toggles(&b));
    }

    #[test]
    fn optical_kerning_crosses_the_bridge() {
        let setting = CharacterSetting {
            kerning_mode: 1,
            kerning_value: 0,
            ..non_default_setting()
        };
        let attrs = character_attrs(&setting);
        assert_eq!(attrs.kerning, KerningMode::Optical);
        assert_eq!(character_setting(&attrs).kerning_mode, 1);
    }
}
