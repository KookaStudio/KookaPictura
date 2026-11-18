//! The installed semantic content and its unchanged-save representation.
use crate::*;
use std::sync::{Arc, OnceLock};
mod parts;
pub use parts::LayerParts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Pixel,
    Adjustment,
    Fill,
    Text,
    Shape,
    SmartObject,
    Group,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayerContent {
    Pixel,
    Adjustment(Option<Arc<Adjustment>>),
    Fill(Arc<FillContent>),
    Text(Option<Arc<TypeTool>>),
    Shape(Arc<FillContent>),
    SmartObject(Arc<SmartObject>),
    Group(LayerStack),
}
impl LayerContent {
    pub fn kind(&self) -> LayerKind {
        match self {
            Self::Pixel => LayerKind::Pixel,
            Self::Adjustment(_) => LayerKind::Adjustment,
            Self::Fill(_) => LayerKind::Fill,
            Self::Text(_) => LayerKind::Text,
            Self::Shape(_) => LayerKind::Shape,
            Self::SmartObject(_) => LayerKind::SmartObject,
            Self::Group(_) => LayerKind::Group,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Preservation {
    adjustment: Option<AdjustmentData>,
    extra_blocks: Vec<LayerBlock>,
    raw_channels: Vec<RawChannel>,
    fallback_text: Option<Arc<TypeTool>>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub rect: PsdRect,
    pub blend: BlendMode,
    pub opacity: u8,
    pub fill: u8,
    pub lock: LockFlags,
    pub color: ColorLabel,
    pub clipping: bool,
    pub visible: bool,
    pub pixel_data_irrelevant: bool,
    pub mask: Option<LayerMask>,
    pub background: bool,
    pub blend_key: Option<[u8; 4]>,
    pub blending_ranges: Plane<u8>,
    pub knockout: Knockout,
    pub blend_clipping: bool,
    pub blend_interior: bool,
    pub transparency_shapes: bool,
    pub layer_mask_hides_effects: bool,
    pub vector_mask_hides_effects: bool,
    pub restricted_channels: Vec<u32>,
    pub blend_if: Option<BlendIf>,
    pub vector_mask: Option<VectorMask>,
    pub applied_character_style: Option<String>,
    pub applied_paragraph_style: Option<String>,
    pub type_overrides: StyleOverrides,
    pub source_channels: Option<SourceChannels>,
    content: LayerContent,
    channels: Vec<Channel>,
    preservation: Preservation,
    edited_fill: bool,
    content_stamp: u64,
}
impl Default for Layer {
    fn default() -> Self {
        Self::new(LayerContent::Pixel)
    }
}
impl Layer {
    pub fn new(content: LayerContent) -> Self {
        Self::from_parts(LayerParts::default(), content)
    }
    /// Install a resolved value and its source data together. The codec resolves
    /// competing imported definitions before calling this constructor.
    pub fn from_parts(parts: LayerParts, content: LayerContent) -> Self {
        Self {
            name: parts.name,
            rect: parts.rect,
            blend: parts.blend,
            opacity: parts.opacity,
            fill: parts.fill,
            lock: parts.lock,
            color: parts.color,
            clipping: parts.clipping,
            visible: parts.visible,
            pixel_data_irrelevant: parts.pixel_data_irrelevant,
            mask: parts.mask,
            background: parts.background,
            blend_key: parts.blend_key,
            blending_ranges: parts.blending_ranges,
            knockout: parts.knockout,
            blend_clipping: parts.blend_clipping,
            blend_interior: parts.blend_interior,
            transparency_shapes: parts.transparency_shapes,
            layer_mask_hides_effects: parts.layer_mask_hides_effects,
            vector_mask_hides_effects: parts.vector_mask_hides_effects,
            restricted_channels: parts.restricted_channels,
            blend_if: parts.blend_if,
            vector_mask: parts.vector_mask,
            applied_character_style: parts.applied_character_style,
            applied_paragraph_style: parts.applied_paragraph_style,
            type_overrides: parts.type_overrides,
            source_channels: parts.source_channels,
            content,
            channels: parts.channels,
            preservation: Preservation {
                adjustment: parts.adjustment,
                extra_blocks: parts.extra_blocks,
                raw_channels: parts.raw_channels,
                fallback_text: parts.type_tool.map(Arc::new),
            },
            edited_fill: false,
            content_stamp: crate::fresh_stamp(),
        }
    }
    pub fn kind(&self) -> LayerKind {
        self.content.kind()
    }
    /// Cache identity for an installed content value; snapshots retain it.
    pub fn content_identity(&self) -> u64 {
        self.content_stamp
    }
    pub fn content(&self) -> &LayerContent {
        &self.content
    }
    pub fn is_group(&self) -> bool {
        matches!(self.content, LayerContent::Group(_))
    }
    pub fn has_adjustment_content(&self) -> bool {
        matches!(
            self.content,
            LayerContent::Adjustment(_) | LayerContent::Fill(_) | LayerContent::Shape(_)
        )
    }
    pub fn is_type(&self) -> bool {
        matches!(self.content, LayerContent::Text(_))
    }
    pub fn adjustment_parameters(&self) -> Option<&Adjustment> {
        match &self.content {
            LayerContent::Adjustment(value) => value.as_deref(),
            _ => None,
        }
    }
    pub fn fill_content(&self) -> Option<&FillContent> {
        match &self.content {
            LayerContent::Fill(value) | LayerContent::Shape(value) => Some(value),
            _ => None,
        }
    }
    pub fn smart_object(&self) -> Option<&SmartObject> {
        match &self.content {
            LayerContent::SmartObject(value) => Some(value),
            _ => None,
        }
    }
    pub fn type_tool(&self) -> Option<&TypeTool> {
        match &self.content {
            LayerContent::Text(value) => value.as_deref(),
            _ => None,
        }
    }
    pub fn fallback_type_tool(&self) -> Option<&TypeTool> {
        self.preservation.fallback_text.as_deref()
    }
    pub fn children(&self) -> &LayerStack {
        static EMPTY: OnceLock<LayerStack> = OnceLock::new();
        match &self.content {
            LayerContent::Group(children) => children,
            _ => EMPTY.get_or_init(LayerStack::default),
        }
    }
    pub fn children_mut(&mut self) -> Option<&mut LayerStack> {
        match &mut self.content {
            LayerContent::Group(children) => Some(children),
            _ => None,
        }
    }
    pub fn children_iter_mut(&mut self) -> std::slice::IterMut<'_, Layer> {
        match &mut self.content {
            LayerContent::Group(children) => children.iter_mut(),
            _ => Default::default(),
        }
    }
    /// Replace group structure explicitly; children can never belong to pixels.
    pub fn set_group_children(&mut self, children: LayerStack) {
        self.content = LayerContent::Group(children);
        self.content_stamp = crate::fresh_stamp();
    }
    pub fn channels(&self) -> &Vec<Channel> {
        &self.channels
    }
    pub fn channels_mut(&mut self) -> &mut Vec<Channel> {
        &mut self.channels
    }
    pub fn channels_and_source_mut(&mut self) -> (&mut Vec<Channel>, &mut Option<SourceChannels>) {
        (&mut self.channels, &mut self.source_channels)
    }
    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.channels = channels;
    }
    pub fn adjustment(&self) -> Option<&AdjustmentData> {
        self.preservation.adjustment.as_ref()
    }
    pub fn extra_blocks(&self) -> &Vec<LayerBlock> {
        &self.preservation.extra_blocks
    }
    pub fn discard_raw_channels(&mut self) {
        self.preservation.raw_channels.clear();
    }
    pub fn raw_channels(&self) -> &Vec<RawChannel> {
        &self.preservation.raw_channels
    }
    pub fn extra_block(&self, key: &[u8; 4]) -> Option<&LayerBlock> {
        self.extra_blocks().iter().find(|block| &block.key == key)
    }
    /// Edit passive layer metadata without exposing content-defining blocks.
    pub fn set_metadata_block(&mut self, key: [u8; 4], data: Option<Plane<u8>>) -> bool {
        if matches!(
            &key,
            b"TySh"
                | b"SoLd"
                | b"SoLE"
                | b"PlLd"
                | b"plLd"
                | b"vscg"
                | b"PiSt"
                | b"vmsk"
                | b"vsms"
                | b"lsct"
                | b"lsdk"
                | b"nvrt"
                | b"invr"
                | b"post"
                | b"thrs"
                | b"brit"
                | b"levl"
                | b"hue2"
                | b"hue "
                | b"expA"
                | b"phfl"
                | b"vibA"
                | b"shdH"
                | b"blwh"
                | b"gdrm"
                | b"grdm"
                | b"blnc"
                | b"mixr"
                | b"curv"
                | b"selc"
                | b"clrL"
                | b"SoCo"
                | b"GdFl"
                | b"PtFl"
        ) {
            return false;
        }
        let index = self
            .preservation
            .extra_blocks
            .iter()
            .position(|block| block.key == key);
        match (index, data) {
            (Some(index), Some(data)) => self.preservation.extra_blocks[index].data = data,
            (Some(index), None) => {
                self.preservation.extra_blocks.remove(index);
            }
            (None, Some(data)) => self
                .preservation
                .extra_blocks
                .push(LayerBlock { key, data }),
            (None, None) => return false,
        }
        true
    }
    pub fn fill_was_edited(&self) -> bool {
        self.edited_fill
    }
    pub fn set_fill_content(&mut self, value: FillContent) -> bool {
        match &mut self.content {
            LayerContent::Fill(fill) | LayerContent::Shape(fill) => {
                *fill = Arc::new(value);
                self.edited_fill = true;
                self.content_stamp = crate::fresh_stamp();
                true
            }
            _ => false,
        }
    }
}
impl From<Layer> for LayerParts {
    fn from(layer: Layer) -> Self {
        let Layer {
            name,
            rect,
            blend,
            opacity,
            fill,
            lock,
            color,
            clipping,
            visible,
            pixel_data_irrelevant,
            mask,
            background,
            blend_key,
            blending_ranges,
            knockout,
            blend_clipping,
            blend_interior,
            transparency_shapes,
            layer_mask_hides_effects,
            vector_mask_hides_effects,
            restricted_channels,
            blend_if,
            vector_mask,
            applied_character_style,
            applied_paragraph_style,
            type_overrides,
            source_channels,
            content,
            channels,
            preservation,
            ..
        } = layer;
        let mut parts = Self {
            name,
            rect,
            blend,
            opacity,
            fill,
            lock,
            color,
            clipping,
            visible,
            pixel_data_irrelevant,
            mask,
            background,
            blend_key,
            blending_ranges,
            knockout,
            blend_clipping,
            blend_interior,
            transparency_shapes,
            layer_mask_hides_effects,
            vector_mask_hides_effects,
            restricted_channels,
            blend_if,
            vector_mask,
            applied_character_style,
            applied_paragraph_style,
            type_overrides,
            source_channels,
            channels,
            adjustment: preservation.adjustment,
            extra_blocks: preservation.extra_blocks,
            raw_channels: preservation.raw_channels,
            type_tool: preservation.fallback_text.map(Arc::unwrap_or_clone),
            ..Default::default()
        };
        match content {
            LayerContent::Group(children) => {
                parts.is_group = true;
                parts.children = children;
            }
            LayerContent::SmartObject(value) => {
                parts.smart_object = Some(Arc::unwrap_or_clone(value))
            }
            LayerContent::Text(value) => parts.type_tool = value.map(Arc::unwrap_or_clone),
            _ => {}
        }
        parts
    }
}
#[cfg(test)]
mod tests;

impl AsMut<Layer> for Layer {
    fn as_mut(&mut self) -> &mut Layer {
        self
    }
}

impl PartialEq for Layer {
    fn eq(&self, other: &Self) -> bool {
        // Cache generations do not change document or history semantics.
        self.name == other.name
            && self.rect == other.rect
            && self.blend == other.blend
            && self.opacity == other.opacity
            && self.fill == other.fill
            && self.lock == other.lock
            && self.color == other.color
            && self.clipping == other.clipping
            && self.visible == other.visible
            && self.pixel_data_irrelevant == other.pixel_data_irrelevant
            && self.mask == other.mask
            && self.background == other.background
            && self.blend_key == other.blend_key
            && self.blending_ranges == other.blending_ranges
            && self.knockout == other.knockout
            && self.blend_clipping == other.blend_clipping
            && self.blend_interior == other.blend_interior
            && self.transparency_shapes == other.transparency_shapes
            && self.layer_mask_hides_effects == other.layer_mask_hides_effects
            && self.vector_mask_hides_effects == other.vector_mask_hides_effects
            && self.restricted_channels == other.restricted_channels
            && self.blend_if == other.blend_if
            && self.vector_mask == other.vector_mask
            && self.applied_character_style == other.applied_character_style
            && self.applied_paragraph_style == other.applied_paragraph_style
            && self.type_overrides == other.type_overrides
            && self.source_channels == other.source_channels
            && self.content == other.content
            && self.channels == other.channels
            && self.preservation == other.preservation
            && self.edited_fill == other.edited_fill
    }
}
