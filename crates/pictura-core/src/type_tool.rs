//! A derived view of a layer's `TySh` type-tool block.
//!
//! The raw `TySh` tagged block stays in [`crate::Layer::extra_blocks`] and is
//! re-emitted verbatim on an unmodified save; this view exposes the affine
//! transform, the `Txt ` string, and the bounds. `text_desc`/`warp_desc` keep
//! the full version-16 descriptor bytes from the input so a re-encode is
//! framing-only.

/// Anti-aliasing method, the `TySh` text descriptor's `AntA` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AntiAlias {
    None,
    #[default]
    Sharp,
    Crisp,
    Strong,
    Smooth,
}

impl AntiAlias {
    /// The index the app options bar and cxx-qt bridge carry: None 0 … Smooth 4.
    pub fn index(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Sharp => 1,
            Self::Crisp => 2,
            Self::Strong => 3,
            Self::Smooth => 4,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            0 => Self::None,
            2 => Self::Crisp,
            3 => Self::Strong,
            4 => Self::Smooth,
            _ => Self::Sharp,
        }
    }
}

/// Kerning mode; `Manual` is the pair value in 1/1000 em. `Optical` spaces
/// pairs from their outlines; it has no groundable PSD key, so it saves as
/// Metrics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum KerningMode {
    #[default]
    Metrics,
    Optical,
    Manual(i32),
}

/// Leading: the engine's auto value or a fixed baseline-to-baseline distance.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Leading {
    #[default]
    Auto,
    Fixed(f64),
}

/// Paragraph justification, the engine's `/Justification`: 0 Left, 1 Right,
/// 2 Center, 3 Justify Last Left, 4 Justify Last Right, 5 Justify Last Center,
/// 6 Justify All. The 3–6 order is psd-tools' `Justification` enum
/// (`JUSTIFY_LAST_LEFT`, `JUSTIFY_LAST_RIGHT`, `JUSTIFY_LAST_CENTER`,
/// `JUSTIFY_ALL`), confirmed against psd-tools 1.19 in-repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Left,
    Right,
    Center,
    JustifyLastLeft,
    JustifyLastRight,
    JustifyLastCenter,
    JustifyAll,
}

impl Justify {
    /// The engine's `/Justification` index.
    pub fn index(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Center => 2,
            Self::JustifyLastLeft => 3,
            Self::JustifyLastRight => 4,
            Self::JustifyLastCenter => 5,
            Self::JustifyAll => 6,
        }
    }

    /// The justification an engine index names, `None` when it names no
    /// modelled value (7 or more): a read must not silently coerce those to
    /// Left.
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Center),
            3 => Some(Self::JustifyLastLeft),
            4 => Some(Self::JustifyLastRight),
            5 => Some(Self::JustifyLastCenter),
            6 => Some(Self::JustifyAll),
            _ => None,
        }
    }
}

/// Composition method: single-line or every-line composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Composer {
    #[default]
    SingleLine,
    EveryLine,
}

/// The default spelling-dictionary language, as CS6 names it.
pub const DEFAULT_LANGUAGE: &str = "English: USA";

/// The character attribute set. `fill_color` keeps EngineData's `Values` order:
/// alpha, red, green, blue, as fractions. Scales are percentages, tracking and
/// baseline shift are 1/1000 em and pixels respectively; `leading` is auto or a
/// fixed distance.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterAttrs {
    pub font_family: String,
    pub font_style: String,
    pub size: f64,
    pub leading: Leading,
    pub kerning: KerningMode,
    pub tracking: f64,
    pub horizontal_scale: f64,
    pub vertical_scale: f64,
    pub baseline_shift: f64,
    pub anti_alias: AntiAlias,
    pub fill_color: [f64; 4],
    pub all_caps: bool,
    pub small_caps: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub fractional_widths: bool,
    pub faux_bold: bool,
    pub faux_italic: bool,
    pub standard_ligatures: bool,
    pub contextual_alternates: bool,
    pub discretionary_ligatures: bool,
    pub swash: bool,
    pub oldstyle: bool,
    pub stylistic_alternates: bool,
    pub titling_alternates: bool,
    pub ornaments: bool,
    pub ordinals: bool,
    pub fractions: bool,
    /// The spelling-dictionary language, by its CS6 display name
    /// (`English: USA`). The EngineData language index is not modelled.
    pub language: String,
    /// `Standard Vertical Roman Alignment`: upright Roman characters in
    /// vertical type.
    pub vertical_roman_alignment: bool,
}

impl Default for CharacterAttrs {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_style: String::new(),
            size: 0.0,
            leading: Leading::Auto,
            kerning: KerningMode::Metrics,
            tracking: 0.0,
            horizontal_scale: 100.0,
            vertical_scale: 100.0,
            baseline_shift: 0.0,
            anti_alias: AntiAlias::Sharp,
            fill_color: [0.0, 0.0, 0.0, 1.0],
            all_caps: false,
            small_caps: false,
            superscript: false,
            subscript: false,
            underline: false,
            strikethrough: false,
            fractional_widths: true,
            faux_bold: false,
            faux_italic: false,
            standard_ligatures: true,
            contextual_alternates: true,
            discretionary_ligatures: false,
            swash: false,
            oldstyle: false,
            stylistic_alternates: false,
            titling_alternates: false,
            ornaments: false,
            ordinals: false,
            fractions: false,
            language: DEFAULT_LANGUAGE.to_string(),
            vertical_roman_alignment: true,
        }
    }
}

impl CharacterAttrs {
    /// `fill_color` (alpha first, as EngineData stores it) as 8-bit straight
    /// RGBA.
    pub fn rgba(&self) -> [u8; 4] {
        let [a, r, g, b] = self
            .fill_color
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        [r, g, b, a]
    }
}

/// The paragraph attribute set. Word, letter, and glyph spacing are
/// `[min, desired, max]`; word and glyph are percentages (1.0 = 100%) and letter
/// spacing is a percentage offset. Indents and paragraph spacing are pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphAttrs {
    pub justify: Justify,
    pub word_spacing: [f64; 3],
    pub letter_spacing: [f64; 3],
    pub glyph_spacing: [f64; 3],
    pub start_indent: f64,
    pub end_indent: f64,
    pub first_line_indent: f64,
    pub space_before: f64,
    pub space_after: f64,
    pub hanging: bool,
    pub hyphenate: bool,
    pub composer: Composer,
    /// Auto-leading percentage (CS6's `120%`).
    pub auto_leading: f64,
    /// Hyphenation details, the Paragraph panel menu's dictionary values:
    /// the minimum word length, the letters required after the first and
    /// before the last, the consecutive-hyphen limit, and the zone in points.
    pub hyphenate_word_size: i32,
    pub hyphenate_pre: i32,
    pub hyphenate_post: i32,
    pub hyphen_limit: i32,
    pub hyphenation_zone: f64,
    pub hyphenate_caps: bool,
}

impl Default for ParagraphAttrs {
    fn default() -> Self {
        Self {
            justify: Justify::Left,
            word_spacing: [0.8, 1.0, 1.33],
            letter_spacing: [0.0, 0.0, 0.0],
            glyph_spacing: [1.0, 1.0, 1.0],
            start_indent: 0.0,
            end_indent: 0.0,
            first_line_indent: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            hanging: false,
            hyphenate: false,
            composer: Composer::SingleLine,
            auto_leading: 120.0,
            hyphenate_word_size: 5,
            hyphenate_pre: 2,
            hyphenate_post: 2,
            hyphen_limit: 2,
            hyphenation_zone: 36.0,
            hyphenate_caps: true,
        }
    }
}

/// A named character style: sparse per-field character overrides. A field the
/// style leaves `None` does not shadow the applied paragraph style's character
/// default for that field.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterStyle {
    pub name: String,
    pub attrs: CharacterOverrides,
}

/// A named paragraph style: sparse character and paragraph overrides.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphStyle {
    pub name: String,
    pub character: CharacterOverrides,
    pub paragraph: ParagraphOverrides,
}

/// Named character and paragraph styles; always carries the editable,
/// non-renamable default `Basic Paragraph` paragraph style.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyleSheet {
    pub character_styles: Vec<CharacterStyle>,
    pub paragraph_styles: Vec<ParagraphStyle>,
}

impl Default for TextStyleSheet {
    fn default() -> Self {
        Self {
            character_styles: Vec::new(),
            paragraph_styles: vec![ParagraphStyle {
                name: BASIC_PARAGRAPH.to_string(),
                character: CharacterOverrides::default(),
                paragraph: ParagraphOverrides::default(),
            }],
        }
    }
}

/// The document's default paragraph style. It is editable but not renamable or
/// deletable, and its attributes resolve below every applied style.
pub const BASIC_PARAGRAPH: &str = "Basic Paragraph";

/// A refused style-sheet operation: an unknown name, a duplicate name, or a
/// protected style (the default paragraph style).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleError {
    NotFound(String),
    Duplicate(String),
    Protected(String),
}

/// A run's locally-set character attributes; `None` inherits from the applied
/// styles. CS6 local formatting is per attribute, so editing tracking manually
/// leaves size, colour, and the rest to the hierarchy.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharacterOverrides {
    pub font_family: Option<String>,
    pub font_style: Option<String>,
    pub size: Option<f64>,
    pub leading: Option<Leading>,
    pub kerning: Option<KerningMode>,
    pub tracking: Option<f64>,
    pub horizontal_scale: Option<f64>,
    pub vertical_scale: Option<f64>,
    pub baseline_shift: Option<f64>,
    pub anti_alias: Option<AntiAlias>,
    pub fill_color: Option<[f64; 4]>,
    pub all_caps: Option<bool>,
    pub small_caps: Option<bool>,
    pub superscript: Option<bool>,
    pub subscript: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub fractional_widths: Option<bool>,
    pub faux_bold: Option<bool>,
    pub faux_italic: Option<bool>,
    pub standard_ligatures: Option<bool>,
    pub contextual_alternates: Option<bool>,
    pub discretionary_ligatures: Option<bool>,
    pub swash: Option<bool>,
    pub oldstyle: Option<bool>,
    pub stylistic_alternates: Option<bool>,
    pub titling_alternates: Option<bool>,
    pub ornaments: Option<bool>,
    pub ordinals: Option<bool>,
    pub fractions: Option<bool>,
    pub language: Option<String>,
    pub vertical_roman_alignment: Option<bool>,
}

/// A run's locally-set paragraph attributes; `None` inherits.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphOverrides {
    pub justify: Option<Justify>,
    pub word_spacing: Option<[f64; 3]>,
    pub letter_spacing: Option<[f64; 3]>,
    pub glyph_spacing: Option<[f64; 3]>,
    pub start_indent: Option<f64>,
    pub end_indent: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub space_before: Option<f64>,
    pub space_after: Option<f64>,
    pub hanging: Option<bool>,
    pub hyphenate: Option<bool>,
    pub composer: Option<Composer>,
    pub auto_leading: Option<f64>,
    pub hyphenate_word_size: Option<i32>,
    pub hyphenate_pre: Option<i32>,
    pub hyphenate_post: Option<i32>,
    pub hyphen_limit: Option<i32>,
    pub hyphenation_zone: Option<f64>,
    pub hyphenate_caps: Option<bool>,
}

/// A run's manual formatting, the top of the resolution order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleOverrides {
    pub character: CharacterOverrides,
    pub paragraph: ParagraphOverrides,
}

/// Effective formatting after the style hierarchy is resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStyle {
    pub character: CharacterAttrs,
    pub paragraph: ParagraphAttrs,
}

impl TextStyleSheet {
    pub fn character_style(&self, name: &str) -> Option<&CharacterStyle> {
        self.character_styles
            .iter()
            .find(|style| style.name == name)
    }

    pub fn paragraph_style(&self, name: &str) -> Option<&ParagraphStyle> {
        self.paragraph_styles
            .iter()
            .find(|style| style.name == name)
    }

    /// The effective formatting for a run, by CS6 precedence: manual override,
    /// then the applied character style, then the applied paragraph style's
    /// character attributes, then the document default (`Basic Paragraph`).
    ///
    /// ponytail: a style is a complete attribute set, so an applied character
    /// style overrides every character field of the paragraph style; sparse
    /// per-attribute style definitions arrive with the style panels if needed.
    pub fn resolve(
        &self,
        overrides: &StyleOverrides,
        character_style: Option<&str>,
        paragraph_style: Option<&str>,
    ) -> ResolvedStyle {
        let character = character_style.and_then(|name| self.character_style(name));
        let paragraph = paragraph_style.and_then(|name| self.paragraph_style(name));
        let basic = self.paragraph_style(BASIC_PARAGRAPH);

        let mut default_character = CharacterAttrs::default();
        let mut default_paragraph = ParagraphAttrs::default();
        if let Some(basic) = basic {
            merge_character(&mut default_character, &basic.character);
            merge_paragraph(&mut default_paragraph, &basic.paragraph);
        }

        let mut character_attrs = default_character;
        if let Some(paragraph) = paragraph {
            merge_character(&mut character_attrs, &paragraph.character);
        }
        if let Some(character) = character {
            merge_character(&mut character_attrs, &character.attrs);
        }
        merge_character(&mut character_attrs, &overrides.character);

        let mut paragraph_attrs = default_paragraph;
        if let Some(paragraph) = paragraph {
            merge_paragraph(&mut paragraph_attrs, &paragraph.paragraph);
        }
        merge_paragraph(&mut paragraph_attrs, &overrides.paragraph);

        ResolvedStyle {
            character: character_attrs,
            paragraph: paragraph_attrs,
        }
    }

    /// Apply a character style to a run: its effective character attributes.
    pub fn apply_character_style(
        &self,
        name: &str,
        overrides: &CharacterOverrides,
    ) -> Result<CharacterAttrs, StyleError> {
        if self.character_style(name).is_none() {
            return Err(StyleError::NotFound(name.to_string()));
        }
        Ok(self
            .resolve(
                &StyleOverrides {
                    character: overrides.clone(),
                    paragraph: ParagraphOverrides::default(),
                },
                Some(name),
                None,
            )
            .character)
    }

    /// Apply a paragraph style to a run or paragraph: its effective character
    /// and paragraph attributes.
    pub fn apply_paragraph_style(
        &self,
        name: &str,
        overrides: &StyleOverrides,
    ) -> Result<ResolvedStyle, StyleError> {
        if self.paragraph_style(name).is_none() {
            return Err(StyleError::NotFound(name.to_string()));
        }
        Ok(self.resolve(overrides, None, Some(name)))
    }

    pub fn create_character_style(
        &mut self,
        name: &str,
        attrs: CharacterOverrides,
    ) -> Result<(), StyleError> {
        if self.character_style(name).is_some() {
            return Err(StyleError::Duplicate(name.to_string()));
        }
        self.character_styles.push(CharacterStyle {
            name: name.to_string(),
            attrs,
        });
        Ok(())
    }

    pub fn create_paragraph_style(
        &mut self,
        name: &str,
        character: CharacterOverrides,
        paragraph: ParagraphOverrides,
    ) -> Result<(), StyleError> {
        if self.paragraph_style(name).is_some() {
            return Err(StyleError::Duplicate(name.to_string()));
        }
        self.paragraph_styles.push(ParagraphStyle {
            name: name.to_string(),
            character,
            paragraph,
        });
        Ok(())
    }

    pub fn edit_character_style(
        &mut self,
        name: &str,
        attrs: CharacterOverrides,
    ) -> Result<(), StyleError> {
        let style = self
            .character_styles
            .iter_mut()
            .find(|style| style.name == name)
            .ok_or_else(|| StyleError::NotFound(name.to_string()))?;
        style.attrs = attrs;
        Ok(())
    }

    pub fn edit_paragraph_style(
        &mut self,
        name: &str,
        character: CharacterOverrides,
        paragraph: ParagraphOverrides,
    ) -> Result<(), StyleError> {
        let style = self
            .paragraph_styles
            .iter_mut()
            .find(|style| style.name == name)
            .ok_or_else(|| StyleError::NotFound(name.to_string()))?;
        style.character = character;
        style.paragraph = paragraph;
        Ok(())
    }

    pub fn delete_character_style(&mut self, name: &str) -> Result<(), StyleError> {
        let before = self.character_styles.len();
        self.character_styles.retain(|style| style.name != name);
        if self.character_styles.len() == before {
            return Err(StyleError::NotFound(name.to_string()));
        }
        Ok(())
    }

    pub fn delete_paragraph_style(&mut self, name: &str) -> Result<(), StyleError> {
        if name == BASIC_PARAGRAPH {
            return Err(StyleError::Protected(name.to_string()));
        }
        let before = self.paragraph_styles.len();
        self.paragraph_styles.retain(|style| style.name != name);
        if self.paragraph_styles.len() == before {
            return Err(StyleError::NotFound(name.to_string()));
        }
        Ok(())
    }

    pub fn rename_character_style(&mut self, from: &str, to: &str) -> Result<(), StyleError> {
        if from == to {
            return self
                .character_style(from)
                .map(|_| ())
                .ok_or_else(|| StyleError::NotFound(from.to_string()));
        }
        if self.character_style(to).is_some() {
            return Err(StyleError::Duplicate(to.to_string()));
        }
        let style = self
            .character_styles
            .iter_mut()
            .find(|style| style.name == from)
            .ok_or_else(|| StyleError::NotFound(from.to_string()))?;
        style.name = to.to_string();
        Ok(())
    }

    pub fn rename_paragraph_style(&mut self, from: &str, to: &str) -> Result<(), StyleError> {
        if from == BASIC_PARAGRAPH {
            return Err(StyleError::Protected(from.to_string()));
        }
        if to == BASIC_PARAGRAPH {
            return Err(StyleError::Protected(to.to_string()));
        }
        if from == to {
            return self
                .paragraph_style(from)
                .map(|_| ())
                .ok_or_else(|| StyleError::NotFound(from.to_string()));
        }
        if self.paragraph_style(to).is_some() {
            return Err(StyleError::Duplicate(to.to_string()));
        }
        let style = self
            .paragraph_styles
            .iter_mut()
            .find(|style| style.name == from)
            .ok_or_else(|| StyleError::NotFound(from.to_string()))?;
        style.name = to.to_string();
        Ok(())
    }
}

/// Overwrite `attrs` with the fields `over` sets.
fn merge_character(attrs: &mut CharacterAttrs, over: &CharacterOverrides) {
    macro_rules! set {
        ($($field:ident),* $(,)?) => {
            $( if let Some(value) = &over.$field { attrs.$field = value.clone(); } )*
        };
    }
    set!(
        font_family,
        font_style,
        size,
        leading,
        kerning,
        tracking,
        horizontal_scale,
        vertical_scale,
        baseline_shift,
        anti_alias,
        fill_color,
        all_caps,
        small_caps,
        superscript,
        subscript,
        underline,
        strikethrough,
        fractional_widths,
        faux_bold,
        faux_italic,
        standard_ligatures,
        contextual_alternates,
        discretionary_ligatures,
        swash,
        oldstyle,
        stylistic_alternates,
        titling_alternates,
        ornaments,
        ordinals,
        fractions,
        language,
        vertical_roman_alignment,
    );
}

/// Overwrite `attrs` with the fields `over` sets.
fn merge_paragraph(attrs: &mut ParagraphAttrs, over: &ParagraphOverrides) {
    macro_rules! set {
        ($($field:ident),* $(,)?) => {
            $( if let Some(value) = &over.$field { attrs.$field = value.clone(); } )*
        };
    }
    set!(
        justify,
        word_spacing,
        letter_spacing,
        glyph_spacing,
        start_indent,
        end_indent,
        first_line_indent,
        space_before,
        space_after,
        hanging,
        hyphenate,
        composer,
        auto_leading,
        hyphenate_word_size,
        hyphenate_pre,
        hyphenate_post,
        hyphen_limit,
        hyphenation_zone,
        hyphenate_caps,
    );
}

/// Decoded type-tool framing. `transform` is `xx, xy, yx, yy, tx, ty`;
/// `bounds` is `left, top, right, bottom`. `vertical` is the text
/// descriptor's `Ornt` (`Vrtc`): the Vertical Type tool's columns.
/// `box_bounds` is paragraph type's box (EngineData `BoxBounds`: left, top,
/// right, bottom relative to the origin, before the transform's linear part);
/// `None` for point type.
#[derive(Debug, Clone)]
pub struct TypeTool {
    pub transform: [f64; 6],
    pub text: String,
    pub bounds: [i32; 4],
    pub text_desc: Vec<u8>,
    pub warp_desc: Vec<u8>,
    pub fonts: Vec<String>,
    pub style: Option<TextStyle>,
    pub vertical: bool,
    pub box_bounds: Option<[f64; 4]>,
}

/// Effective text style of a type layer's first style run, with the
/// paragraph/style defaults already applied. `applied_character_style` /
/// `applied_paragraph_style` name the saved styles resolved into `character` /
/// `paragraph`; they are in-memory only (style definitions are not written to
/// PSD), so a reopened file reports `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    pub font: Option<String>,
    pub character: CharacterAttrs,
    pub paragraph: ParagraphAttrs,
    pub applied_character_style: Option<String>,
    pub applied_paragraph_style: Option<String>,
}

impl TextStyle {
    /// `fill_color` as 8-bit straight RGBA.
    pub fn rgba(&self) -> [u8; 4] {
        self.character.rgba()
    }
}

/// What the Type tools commit: a point-type string and how it is set.
///
/// `text` separates lines with `\r`, as `TySh` does. `origin` is the click in
/// document pixels: the first baseline's start (horizontal) or the first
/// column's top centre (vertical), moved by `justification` (see [`Justify`]).
/// `character` and `paragraph` are the full attribute
/// model and the single source for the font family, size, colour,
/// justification, and anti-aliasing. `matrix` is the linear part of the `TySh`
/// transform (`xx, xy, yx, yy`: `x' = xx·x + yx·y`, `y' = xy·x + yy·y`) mapping
/// the laid-out text about the origin — Free Transform's scale and rotation;
/// identity for new type. `box_bounds` makes it paragraph type: the box the
/// text wraps to, left, top, right, bottom relative to the origin before
/// `matrix` (a dragged box has its top-left at the origin); `None` is point
/// type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeSpec {
    pub text: String,
    pub vertical: bool,
    pub origin: (f64, f64),
    pub matrix: [f64; 4],
    pub box_bounds: Option<[f64; 4]>,
    pub character: CharacterAttrs,
    pub paragraph: ParagraphAttrs,
    /// The named styles this run applies, in memory only (a style definition is
    /// not written to PSD, so a reopened file reports `None`).
    pub applied_character_style: Option<String>,
    pub applied_paragraph_style: Option<String>,
    /// The run's manual formatting, which outranks both applied styles.
    pub overrides: StyleOverrides,
}

impl TypeSpec {
    pub const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 1.0];

    /// A spec with `text`, the font family, and `size` set and every placement
    /// and attribute field at its default; callers fill the rest with
    /// struct-update syntax.
    pub fn new(text: impl Into<String>, font: impl Into<String>, size: f64) -> Self {
        Self {
            text: text.into(),
            vertical: false,
            origin: (0.0, 0.0),
            matrix: Self::IDENTITY,
            box_bounds: None,
            character: CharacterAttrs {
                font_family: font.into(),
                size,
                ..CharacterAttrs::default()
            },
            paragraph: ParagraphAttrs::default(),
            applied_character_style: None,
            applied_paragraph_style: None,
            overrides: StyleOverrides::default(),
        }
    }

    /// The text colour as 8-bit straight RGBA.
    pub fn rgba(&self) -> [u8; 4] {
        self.character.rgba()
    }

    /// The paragraph's justification as the engine index (see [`Justify`]).
    pub fn justification(&self) -> u8 {
        self.paragraph.justify.index()
    }
}

impl PartialEq for TypeTool {
    fn eq(&self, other: &Self) -> bool {
        self.transform.map(f64::to_bits) == other.transform.map(f64::to_bits)
            && self.text == other.text
            && self.bounds == other.bounds
            && self.text_desc == other.text_desc
            && self.warp_desc == other.warp_desc
            && self.fonts == other.fonts
            && self.style == other.style
            && self.vertical == other.vertical
            && self.box_bounds.map(|b| b.map(f64::to_bits))
                == other.box_bounds.map(|b| b.map(f64::to_bits))
    }
}

impl Eq for TypeTool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_type_defaults_are_cs6() {
        let character = CharacterAttrs::default();
        assert_eq!(character.horizontal_scale, 100.0);
        assert_eq!(character.vertical_scale, 100.0);
        assert_eq!(character.kerning, KerningMode::Metrics);
        assert_eq!(character.anti_alias, AntiAlias::Sharp);
        assert_eq!(character.tracking, 0.0);
        assert_eq!(character.baseline_shift, 0.0);
        assert!(!character.all_caps);
        assert!(!character.small_caps);
        assert!(!character.superscript);
        assert!(!character.subscript);
        assert!(!character.underline);
        assert!(!character.strikethrough);
        assert!(character.fractional_widths);
        assert!(!character.faux_bold);
        assert!(!character.faux_italic);
        assert!(character.standard_ligatures);
        assert!(character.contextual_alternates);
        assert!(!character.discretionary_ligatures);
        assert!(!character.swash);
        assert!(!character.oldstyle);
        assert!(!character.stylistic_alternates);
        assert!(!character.titling_alternates);
        assert!(!character.ornaments);
        assert!(!character.ordinals);
        assert!(!character.fractions);
        assert_eq!(character.language, DEFAULT_LANGUAGE);
        assert!(character.vertical_roman_alignment);

        let paragraph = ParagraphAttrs::default();
        assert_eq!(paragraph.justify, Justify::Left);
        assert_eq!(paragraph.word_spacing, [0.8, 1.0, 1.33]);
        assert_eq!(paragraph.letter_spacing, [0.0, 0.0, 0.0]);
        assert_eq!(paragraph.glyph_spacing, [1.0, 1.0, 1.0]);
        assert_eq!(paragraph.start_indent, 0.0);
        assert_eq!(paragraph.end_indent, 0.0);
        assert_eq!(paragraph.first_line_indent, 0.0);
        assert_eq!(paragraph.space_before, 0.0);
        assert_eq!(paragraph.space_after, 0.0);
        assert!(!paragraph.hanging);
        assert!(!paragraph.hyphenate);
        assert_eq!(paragraph.composer, Composer::SingleLine);
        assert_eq!(paragraph.auto_leading, 120.0);
        assert_eq!(paragraph.hyphenate_word_size, 5);
        assert_eq!(paragraph.hyphenate_pre, 2);
        assert_eq!(paragraph.hyphenate_post, 2);
        assert_eq!(paragraph.hyphen_limit, 2);
        assert_eq!(paragraph.hyphenation_zone, 36.0);
        assert!(paragraph.hyphenate_caps);

        let sheet = TextStyleSheet::default();
        assert!(sheet
            .paragraph_styles
            .iter()
            .any(|style| style.name == "Basic Paragraph"));
    }

    #[test]
    fn justify_index_round_trips_every_modelled_value() {
        for value in [
            Justify::Left,
            Justify::Right,
            Justify::Center,
            Justify::JustifyLastLeft,
            Justify::JustifyLastRight,
            Justify::JustifyLastCenter,
            Justify::JustifyAll,
        ] {
            assert_eq!(Justify::from_index(value.index()), Some(value));
        }
        assert_eq!(Justify::from_index(7), None);
        assert_eq!(Justify::from_index(255), None);
    }

    #[test]
    fn resolve_prefers_manual_then_character_then_paragraph_then_default() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .edit_paragraph_style(
                BASIC_PARAGRAPH,
                CharacterOverrides {
                    size: Some(11.0),
                    ..Default::default()
                },
                ParagraphOverrides {
                    justify: Some(Justify::Right),
                    ..Default::default()
                },
            )
            .unwrap();
        sheet
            .create_paragraph_style(
                "Para",
                CharacterOverrides {
                    size: Some(22.0),
                    ..Default::default()
                },
                ParagraphOverrides::default(),
            )
            .unwrap();
        sheet
            .create_character_style(
                "Char",
                CharacterOverrides {
                    size: Some(33.0),
                    ..Default::default()
                },
            )
            .unwrap();

        let manual = StyleOverrides {
            character: CharacterOverrides {
                size: Some(44.0),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            sheet
                .resolve(&manual, Some("Char"), Some("Para"))
                .character
                .size,
            44.0
        );
        assert_eq!(
            sheet
                .resolve(&StyleOverrides::default(), Some("Char"), Some("Para"))
                .character
                .size,
            33.0
        );
        assert_eq!(
            sheet
                .resolve(&StyleOverrides::default(), None, Some("Para"))
                .character
                .size,
            22.0
        );
        let default = sheet.resolve(&StyleOverrides::default(), None, None);
        assert_eq!(default.character.size, 11.0);
        assert_eq!(default.paragraph.justify, Justify::Right);
    }

    #[test]
    fn applying_a_style_keeps_a_manual_override_on_another_attribute() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .create_character_style(
                "Heading",
                CharacterOverrides {
                    size: Some(33.0),
                    ..Default::default()
                },
            )
            .unwrap();
        let manual = StyleOverrides {
            character: CharacterOverrides {
                tracking: Some(120.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let resolved = sheet.resolve(&manual, Some("Heading"), None);
        assert_eq!(resolved.character.tracking, 120.0);
        assert_eq!(resolved.character.size, 33.0);
    }

    #[test]
    fn a_paragraph_style_resolves_its_extended_attributes() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .create_paragraph_style(
                "Heading",
                CharacterOverrides {
                    faux_bold: Some(true),
                    fractions: Some(true),
                    language: Some("French".to_string()),
                    ..Default::default()
                },
                ParagraphOverrides {
                    auto_leading: Some(150.0),
                    hyphenate: Some(true),
                    hyphen_limit: Some(3),
                    hyphenate_caps: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
        let resolved = sheet
            .apply_paragraph_style("Heading", &StyleOverrides::default())
            .unwrap();
        assert!(resolved.character.faux_bold);
        assert!(resolved.character.fractions);
        assert_eq!(resolved.character.language, "French");
        assert!(
            resolved.character.standard_ligatures,
            "a field the style leaves unset keeps the default"
        );
        assert_eq!(resolved.paragraph.auto_leading, 150.0);
        assert!(resolved.paragraph.hyphenate);
        assert_eq!(resolved.paragraph.hyphen_limit, 3);
        assert!(!resolved.paragraph.hyphenate_caps);
    }

    #[test]
    fn applying_a_style_resolves_its_attributes_or_reports_a_missing_name() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .create_character_style(
                "Heading",
                CharacterOverrides {
                    size: Some(20.0),
                    ..Default::default()
                },
            )
            .unwrap();
        let attrs = sheet
            .apply_character_style("Heading", &CharacterOverrides::default())
            .unwrap();
        assert_eq!(attrs.size, 20.0);
        assert_eq!(
            sheet.apply_character_style("Ghost", &CharacterOverrides::default()),
            Err(StyleError::NotFound("Ghost".to_string()))
        );

        let resolved = sheet
            .apply_paragraph_style(BASIC_PARAGRAPH, &StyleOverrides::default())
            .unwrap();
        assert_eq!(resolved.paragraph.justify, Justify::Left);
        assert_eq!(
            sheet.apply_paragraph_style("Ghost", &StyleOverrides::default()),
            Err(StyleError::NotFound("Ghost".to_string()))
        );
    }

    #[test]
    fn basic_paragraph_refuses_rename_and_delete_but_accepts_edits() {
        let mut sheet = TextStyleSheet::default();
        assert_eq!(
            sheet.rename_paragraph_style(BASIC_PARAGRAPH, "Body"),
            Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
        );
        assert_eq!(
            sheet.delete_paragraph_style(BASIC_PARAGRAPH),
            Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
        );
        let paragraph = ParagraphOverrides {
            justify: Some(Justify::Center),
            ..Default::default()
        };
        sheet
            .edit_paragraph_style(
                BASIC_PARAGRAPH,
                CharacterOverrides::default(),
                paragraph.clone(),
            )
            .unwrap();
        assert_eq!(
            sheet.paragraph_style(BASIC_PARAGRAPH).unwrap().paragraph,
            paragraph
        );
        assert_eq!(sheet.paragraph_styles.len(), 1);
    }

    #[test]
    fn renaming_a_custom_style_onto_basic_paragraph_names_the_protected_target() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .create_paragraph_style(
                "Body",
                CharacterOverrides::default(),
                ParagraphOverrides::default(),
            )
            .unwrap();
        assert_eq!(
            sheet.rename_paragraph_style("Body", BASIC_PARAGRAPH),
            Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
        );
        assert!(sheet.paragraph_style("Body").is_some());
    }

    #[test]
    fn a_character_style_leaves_the_paragraph_styles_character_defaults_alone() {
        let mut sheet = TextStyleSheet::default();
        sheet
            .create_paragraph_style(
                "Body",
                CharacterOverrides {
                    size: Some(18.0),
                    tracking: Some(50.0),
                    ..Default::default()
                },
                ParagraphOverrides::default(),
            )
            .unwrap();
        sheet
            .create_character_style(
                "Bold",
                CharacterOverrides {
                    size: Some(33.0),
                    ..Default::default()
                },
            )
            .unwrap();
        let resolved = sheet.resolve(&StyleOverrides::default(), Some("Bold"), Some("Body"));
        assert_eq!(
            resolved.character.size, 33.0,
            "the character style wins for the field it sets"
        );
        assert_eq!(
            resolved.character.tracking, 50.0,
            "the paragraph default survives a field the character style leaves unset"
        );
    }

    #[test]
    fn create_delete_and_rename_reject_missing_and_duplicate_names() {
        let mut sheet = TextStyleSheet::default();
        assert_eq!(
            sheet.delete_character_style("Ghost"),
            Err(StyleError::NotFound("Ghost".to_string()))
        );
        sheet
            .create_character_style("One", CharacterOverrides::default())
            .unwrap();
        assert_eq!(
            sheet.create_character_style("One", CharacterOverrides::default()),
            Err(StyleError::Duplicate("One".to_string()))
        );
        sheet.rename_character_style("One", "Two").unwrap();
        assert!(sheet.character_style("One").is_none());
        assert!(sheet.character_style("Two").is_some());
        assert_eq!(
            sheet.rename_character_style("Ghost", "Three"),
            Err(StyleError::NotFound("Ghost".to_string()))
        );
        sheet.delete_character_style("Two").unwrap();
        assert!(sheet.character_styles.is_empty());
    }
}
