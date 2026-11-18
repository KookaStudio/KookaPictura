use crate::*;

/// Temporary source candidates and attributes. The codec normalizes these
/// before installing a Layer; they are never independent active payloads.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerParts {
    pub name: String,
    pub rect: PsdRect,
    pub blend: BlendMode,
    pub opacity: u8,
    pub fill: u8,
    pub lock: LockFlags,
    pub color: ColorLabel,
    pub clipping: bool,
    pub visible: bool,
    /// PSD record flag bit 4: the layer's pixel data is irrelevant to the
    /// document's appearance. Set on shape layers and preserved on save so a re-saved shape layer stays a shape.
    /// The writer emits bit 3 (`0x08`, "flags valid") with it.
    pub pixel_data_irrelevant: bool,
    pub mask: Option<LayerMask>,
    pub adjustment: Option<AdjustmentData>,
    pub channels: Vec<Channel>,
    pub children: LayerStack,
    pub is_group: bool,
    /// First-class Background flag. PSD has no background bit; the codec infers it from the bottom
    /// top-level non-group named `"Background"` and writes that name back for a flagged layer (design D5).
    pub background: bool,
    /// Raw blend-mode key, stored only when it does not map to a known [`BlendMode`], preserved for lossless re-save.
    pub blend_key: Option<[u8; 4]>,
    /// Raw blending-ranges block bytes, preserved verbatim for lossless re-save.
    pub blending_ranges: Plane<u8>,
    /// `knko` knockout mode (`None` when the block is absent).
    pub knockout: Knockout,
    /// `clbl` blend clipped elements (CS6 default `true` when absent).
    pub blend_clipping: bool,
    /// `infx` blend interior effects as a group (CS6 default `false` when
    /// absent; a stored byte `1` turns it on).
    pub blend_interior: bool,
    /// `tsly` transparency shapes layer (CS6 default `true` when absent).
    pub transparency_shapes: bool,
    /// `lmgm` layer mask hides effects (CS6 default `false` when absent).
    pub layer_mask_hides_effects: bool,
    /// `vmgm` vector mask hides effects (CS6 default `false` when absent).
    pub vector_mask_hides_effects: bool,
    /// `brst` channel numbers excluded from blending, in file order; the writer
    /// sorts and dedupes them.
    pub restricted_channels: Vec<u32>,
    /// Typed Blend If view of `blending_ranges`; `None` when empty or malformed.
    pub blend_if: Option<BlendIf>,
    /// Additional-layer-info tagged blocks the engine does not model, preserved verbatim for lossless re-save.
    pub extra_blocks: Vec<LayerBlock>,
    /// Layer channels the engine does not decode, kept as full on-disk streams
    /// for verbatim re-emit.
    pub raw_channels: Vec<RawChannel>,
    /// Derived embedded/linked smart-object view; `None` for ordinary layers.
    pub smart_object: Option<SmartObject>,
    /// Derived `vmsk`/`vsms` vector-mask view; `None` when absent or unparseable. The
    /// raw block remains in `extra_blocks` and is the serialization source.
    pub vector_mask: Option<VectorMask>,
    /// Derived `TySh` type-tool view; `None` when absent or unparseable. The
    /// raw block remains in `extra_blocks` and is the serialization source.
    pub type_tool: Option<TypeTool>,
    /// The named styles a type layer applies and the run's manual overrides, in
    /// memory only (a style definition is not written to PSD). Carried on the
    /// layer so a re-author within a session keeps them.
    pub applied_character_style: Option<String>,
    pub applied_paragraph_style: Option<String>,
    pub type_overrides: StyleOverrides,
    /// Retained source channel samples, re-emitted on save when the layer has
    /// not moved: native `16`/`32`-bit samples for a Grayscale/RGB read, or the
    /// 8-bit Lab color planes for a Lab read.
    pub source_channels: Option<SourceChannels>,
}

impl Default for LayerParts {
    fn default() -> Self {
        Self {
            name: String::new(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            pixel_data_irrelevant: false,
            mask: None,
            adjustment: None,
            channels: Vec::new(),
            children: LayerStack::default(),
            is_group: false,
            background: false,
            blend_key: None,
            blending_ranges: Plane::default(),
            knockout: Knockout::None,
            blend_clipping: true,
            blend_interior: false,
            transparency_shapes: true,
            layer_mask_hides_effects: false,
            vector_mask_hides_effects: false,
            restricted_channels: Vec::new(),
            blend_if: None,
            extra_blocks: Vec::new(),
            raw_channels: Vec::new(),
            smart_object: None,
            vector_mask: None,
            type_tool: None,
            applied_character_style: None,
            applied_paragraph_style: None,
            type_overrides: StyleOverrides::default(),
            source_channels: None,
        }
    }
}

impl LayerParts {
    pub fn is_group(&self) -> bool {
        self.is_group
    }
    pub fn adjustment(&self) -> Option<&AdjustmentData> {
        self.adjustment.as_ref()
    }
    pub fn smart_object(&self) -> Option<&SmartObject> {
        self.smart_object.as_ref()
    }
    pub fn type_tool(&self) -> Option<&TypeTool> {
        self.type_tool.as_ref()
    }
    pub fn extra_blocks(&self) -> &[LayerBlock] {
        &self.extra_blocks
    }
    pub fn raw_channels(&self) -> &[RawChannel] {
        &self.raw_channels
    }
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }
    pub fn extra_block(&self, key: &[u8; 4]) -> Option<&LayerBlock> {
        self.extra_blocks.iter().find(|b| &b.key == key)
    }
}
