use pictura_adjust::{GradientKind, GradientStop};
use pictura_core::{BlendMode, Channel, ColorLabel, Document, Layer, LockFlags, PsdRect};

use super::clipping::{normalize_clipping, release_run_above};
use super::paths::{
    container_mut, container_of_mut, format_segments, parse_path, resolve_path, resolve_path_mut,
};

fn insertion_index(len: usize, above: i32) -> usize {
    if above < 0 {
        len
    } else {
        (above as usize + 1).min(len)
    }
}

/// A document-sized raster layer with fully transparent pixels.
///
/// ponytail: the reference stores no pixel data until the layer is painted; a
/// document-sized layer is the lazy stand-in and costs `w*h*4` bytes. Switch to
/// an empty-rect layer once the paint path can grow a layer on first dab.
pub(super) fn transparent_layer(width: u32, height: u32, name: &str) -> Layer {
    let pixels = width as usize * height as usize;
    Layer {
        name: name.to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: height as i32,
            right: width as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![0; pixels].into(),
            },
            Channel {
                id: 1,
                data: vec![0; pixels].into(),
            },
            Channel {
                id: 2,
                data: vec![0; pixels].into(),
            },
            Channel {
                id: -1,
                data: vec![0; pixels].into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// An empty group with a Normal blend and an empty bounds rectangle.
pub(super) fn empty_group(name: &str) -> Layer {
    Layer {
        name: name.to_string(),
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
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: Vec::new(),
        is_group: true,
        background: false,
        ..Default::default()
    }
}

/// The attributes of a New Layer / New Group dialog request (design D6).
#[derive(Debug, Clone)]
pub struct NewLayerSpec {
    pub name: String,
    pub color: ColorLabel,
    pub blend: BlendMode,
    pub opacity: u8,
    pub fill: u8,
    pub clipping: bool,
    pub neutral_fill: bool,
}

impl Default for NewLayerSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            color: ColorLabel::None,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            clipping: false,
            neutral_fill: false,
        }
    }
}

/// The mode-neutral fill as straight-alpha RGBA, or `None` for the seven modes
/// that lack the `Fill With (Mode)-Neutral Color` option.
///
/// See `docs/05-layers/blend-modes.md`: white for Darken/Multiply/Color
/// Burn/Linear Burn/Darker Color/Divide, black for Lighten/Screen/Color
/// Dodge/Linear Dodge (Add)/Lighter Color/Difference/Exclusion/Subtract, and
/// 50 % gray for Overlay/Soft Light/Hard Light/Vivid Light/Linear Light/Pin
/// Light. Normal, Dissolve, Hard Mix, Hue, Saturation, Color, and Luminosity
/// are unlisted and default to transparent.
pub fn neutral_color(mode: BlendMode) -> Option<[u8; 4]> {
    use BlendMode::*;
    Some(match mode {
        Darken | Multiply | ColorBurn | LinearBurn | DarkerColor | Divide => [255, 255, 255, 255],
        Lighten | Screen | ColorDodge | LinearDodge | LighterColor | Difference | Exclusion
        | Subtract => [0, 0, 0, 255],
        Overlay | SoftLight | HardLight | VividLight | LinearLight | PinLight => {
            [128, 128, 128, 255]
        }
        Normal | Dissolve | HardMix | Hue | Saturation | Color | Luminosity | PassThrough => {
            return None
        }
    })
}

/// Resolve a requested name, generating `Prefix N` when it is empty.
fn resolved_name(doc: &Document, name: &str, prefix: &str) -> String {
    if name.is_empty() {
        next_layer_name(doc, prefix)
    } else {
        name.to_string()
    }
}

/// Write a straight-alpha RGBA value into a full-rect layer's color channels.
fn fill_neutral(layer: &mut Layer, rgba: [u8; 4]) {
    for channel in &mut layer.channels {
        let value = match channel.id {
            0 => rgba[0],
            1 => rgba[1],
            2 => rgba[2],
            -1 => rgba[3],
            _ => continue,
        };
        channel.data.fill(value);
    }
}

/// `Prefix N` where N is one more than the highest existing `Prefix <number>`
/// name in the whole tree, falling back to 1.
pub fn next_layer_name(doc: &Document, prefix: &str) -> String {
    let mut highest = 0u32;
    fn visit(layers: &[Layer], prefix: &str, highest: &mut u32) {
        for layer in layers {
            let suffix = layer
                .name
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix(' '))
                .and_then(|n| n.parse::<u32>().ok());
            if let Some(n) = suffix {
                *highest = (*highest).max(n);
            }
            visit(&layer.children, prefix, highest);
        }
    }
    visit(&doc.layers, prefix, &mut highest);
    format!("{prefix} {}", highest + 1)
}

/// Insert a new transparent raster layer directly above `above`; returns its
/// index, or -1 for a document with a zero dimension.
pub fn add_layer(doc: &mut Document, above: i32, name: &str) -> i32 {
    if doc.width == 0 || doc.height == 0 {
        return -1;
    }
    let index = insertion_index(doc.layers.len(), above);
    doc.layers
        .insert(index, transparent_layer(doc.width, doc.height, name));
    index as i32
}

/// Insert an empty group directly above `above`; returns its index.
pub fn add_group(doc: &mut Document, above: i32, name: &str) -> i32 {
    let index = insertion_index(doc.layers.len(), above);
    doc.layers.insert(index, empty_group(name));
    index as i32
}

/// Deep-clone layer `index` (children, channels, mask, adjustment and all
/// attributes) and insert the copy directly above it. The copy is named
/// `"<name> copy"`. Returns the new index, or -1 when `index` is out of range.
pub fn duplicate_layer(doc: &mut Document, index: i32) -> i32 {
    if index < 0 || index as usize >= doc.layers.len() {
        return -1;
    }
    let source = index as usize;
    let mut copy = doc.layers[source].clone();
    copy.name = format!("{} copy", copy.name);
    if copy.background {
        release_background(&mut copy);
    }
    doc.layers.insert(source + 1, copy);
    (source + 1) as i32
}

/// Wrap layer `index` in a new group at the same stack position: the group
/// takes the layer's slot and the layer becomes its only child. Returns the
/// group's index, or -1 when `index` is out of range.
pub fn group_layer(doc: &mut Document, index: i32) -> i32 {
    if index < 0 || index as usize >= doc.layers.len() {
        return -1;
    }
    let source = index as usize;
    let child = doc.layers.remove(source);
    let name = next_layer_name(doc, "Group");
    let mut group = empty_group(&name);
    group.children.push(child);
    doc.layers.insert(source, group);
    source as i32
}

/// Splice a group's children into the parent at the group's position. Returns
/// false (state unchanged) when `index` is out of range or not a group.
pub fn ungroup_layer(doc: &mut Document, index: i32) -> bool {
    if index < 0 || index as usize >= doc.layers.len() {
        return false;
    }
    let at = index as usize;
    if !doc.layers[at].is_group {
        return false;
    }
    let was_clipped = doc.layers[at].clipping;
    release_run_above(doc.layers.as_mut_slice(), at, was_clipped);
    let group = doc.layers.remove(at);
    for (offset, child) in group.children.into_iter().enumerate() {
        doc.layers.insert(at + offset, child);
    }
    normalize_clipping(doc.layers.as_mut_slice(), None);
    true
}

/// Insert `node` inside `selection_path` when it is a group (as its top child),
/// otherwise directly above the selected node in its container, otherwise on
/// top of the document. Returns the new path, or empty on failure.
pub(crate) fn insert_node(doc: &mut Document, selection_path: &str, mut node: Layer) -> String {
    if let Some(segments) = parse_path(selection_path) {
        if resolve_path(doc, selection_path).is_some_and(|layer| layer.is_group) {
            let Some(container) = container_of_mut(doc, &segments) else {
                return String::new();
            };
            let index = container.len();
            if index == 0 {
                node.clipping = false;
            }
            container.push(node);
            let mut new_segments = segments;
            new_segments.push(index);
            return format_segments(&new_segments);
        }
        if let Some((container, index)) = container_mut(doc, &segments) {
            let at = insertion_index(container.len(), index as i32);
            if at == 0 {
                node.clipping = false;
            }
            container.insert(at, node);
            let mut new_segments = segments;
            if let Some(last) = new_segments.last_mut() {
                *last = at;
            }
            return format_segments(&new_segments);
        }
    }
    let at = insertion_index(doc.layers.len(), -1);
    if at == 0 {
        node.clipping = false;
    }
    doc.layers.insert(at, node);
    format_segments(&[at])
}

/// Insert a transparent raster layer for `name` (generated when empty) using
/// the tree-aware [`insert_node`] rule, applying `spec`'s attributes. Returns
/// the new path, or empty for a zero-dimension document.
pub fn add_layer_full(doc: &mut Document, selection_path: &str, spec: &NewLayerSpec) -> String {
    if doc.width == 0 || doc.height == 0 {
        return String::new();
    }
    let name = resolved_name(doc, &spec.name, "Layer");
    let mut layer = transparent_layer(doc.width, doc.height, &name);
    layer.color = spec.color;
    layer.blend = spec.blend;
    layer.opacity = spec.opacity;
    layer.fill = spec.fill;
    layer.clipping = spec.clipping;
    if spec.neutral_fill {
        if let Some(rgba) = neutral_color(spec.blend) {
            fill_neutral(&mut layer, rgba);
        }
    }
    insert_node(doc, selection_path, layer)
}

/// Insert a solid-color fill-content layer at the [`insert_node`] rule.
///
/// The node is document-sized with no pixel channels; its content lives in an
/// opaque `SoCo` block holding the standard PSD version-16 descriptor.
/// Named `"Color Fill N"`. Returns the new path, or empty for a zero-dimension
/// document. Fill layers carry an adjustment, so the merge check refuses them.
pub fn add_solid_fill(doc: &mut Document, selection_path: &str, rgba: [u8; 4]) -> String {
    if doc.width == 0 || doc.height == 0 {
        return String::new();
    }
    let name = next_layer_name(doc, "Color Fill");
    let layer = Layer {
        name,
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        // ponytail: SoCo descriptor is RGB-only, alpha is dropped (app callers
        // pass 0xFF); thread alpha through if a non-opaque fill is ever needed.
        adjustment: Some(crate::encode_solid_color_fill([rgba[0], rgba[1], rgba[2]])),
        channels: Vec::new(),
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    insert_node(doc, selection_path, layer)
}

/// Append a native-size raster pixel layer at the top of `doc.layers` from
/// packed RGBA8888 bytes, returning its path via [`format_segments`].
///
/// Mirrors [`add_solid_fill`]'s construction: the layer is named `name`, sized
/// `(0, 0, width, height)` (not the document), visible, Normal, opaque, with
/// planar channels `0`, `1`, `2`, `-1`. It changes no existing layer and no
/// document state, and returns an empty string for a zero dimension.
pub fn add_raster_layer_from_rgba(
    doc: &mut Document,
    name: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> String {
    if width == 0 || height == 0 {
        return String::new();
    }
    // ponytail: channels are always 0,1,2,-1, so placing a color image into a
    // Grayscale document renders red-as-gray; map to the document mode if needed.
    let plane = width as usize * height as usize;
    let mut channels = vec![
        Channel {
            id: 0,
            data: vec![0; plane].into(),
        },
        Channel {
            id: 1,
            data: vec![0; plane].into(),
        },
        Channel {
            id: 2,
            data: vec![0; plane].into(),
        },
        Channel {
            id: -1,
            data: vec![0; plane].into(),
        },
    ];
    for i in 0..plane {
        let at = i * 4;
        if at + 4 > rgba.len() {
            break;
        }
        channels[0].data[i] = rgba[at];
        channels[1].data[i] = rgba[at + 1];
        channels[2].data[i] = rgba[at + 2];
        channels[3].data[i] = rgba[at + 3];
    }
    let layer = Layer {
        name: name.to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: height as i32,
            right: width as i32,
        },
        channels,
        ..Default::default()
    };
    let index = doc.layers.len();
    doc.layers.push(layer);
    format_segments(&[index])
}

/// Insert a gradient fill-content layer at the [`insert_node`] rule.
///
/// The node is document-sized with no pixel channels; its content lives in an
/// opaque `GdFl` block holding a black-to-white Linear gradient at angle 0.
/// Named `"Gradient Fill N"`. Returns the new path, or empty for a
/// zero-dimension document.
pub fn add_gradient_fill(doc: &mut Document, selection_path: &str) -> String {
    if doc.width == 0 || doc.height == 0 {
        return String::new();
    }
    let name = next_layer_name(doc, "Gradient Fill");
    let stops = [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ];
    let layer = Layer {
        name,
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: Some(crate::encode_gradient_fill(
            GradientKind::Linear,
            &stops,
            0.0,
        )),
        channels: Vec::new(),
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    insert_node(doc, selection_path, layer)
}

/// Insert an empty group for `name` (generated when empty) using the
/// tree-aware [`insert_node`] rule. Groups ignore `fill`, `clipping`, and
/// `neutral_fill`.
pub fn add_group_full(doc: &mut Document, selection_path: &str, spec: &NewLayerSpec) -> String {
    let name = resolved_name(doc, &spec.name, "Group");
    let mut group = empty_group(&name);
    group.color = spec.color;
    group.blend = spec.blend;
    group.opacity = spec.opacity;
    insert_node(doc, selection_path, group)
}

/// Insert a transparent raster layer for `name` (generated when empty) using
/// the tree-aware [`insert_node`] rule. Returns the new path, or empty for a
/// zero-dimension document.
pub fn add_layer_in(doc: &mut Document, selection_path: &str, name: &str) -> String {
    let spec = NewLayerSpec {
        name: name.to_string(),
        ..NewLayerSpec::default()
    };
    add_layer_full(doc, selection_path, &spec)
}

/// Insert an empty group for `name` (generated when empty) using the
/// tree-aware [`insert_node`] rule. Returns the new path.
pub fn add_group_in(doc: &mut Document, selection_path: &str, name: &str) -> String {
    let spec = NewLayerSpec {
        name: name.to_string(),
        ..NewLayerSpec::default()
    };
    add_group_full(doc, selection_path, &spec)
}

/// `Layer from Background…`: clear the Background flag, unlock all four locks,
/// and rename to the next free `Layer N` on the layer at `path`. Refuses a
/// non-background path (returns false, leaving the document unchanged).
pub fn layer_from_background(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if !layer.background {
        return false;
    }
    let name = next_layer_name(doc, "Layer");
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    release_background(layer);
    layer.name = name;
    true
}

/// Make a Background (or a copy of one) an ordinary layer: unflagged,
/// unlocked, and given an opaque alpha channel so it can take transparency.
pub(super) fn release_background(layer: &mut Layer) {
    layer.background = false;
    layer.lock = LockFlags::default();
    if !layer.channels.iter().any(|c| c.id == -1) {
        let pixels = layer.rect.width().max(0) as usize * layer.rect.height().max(0) as usize;
        layer.channels.push(Channel {
            id: -1,
            data: vec![255; pixels].into(),
        });
    }
}

/// `Background From Layer`: flag the node at `path` as the Background, make its
/// fully transparent pixels opaque with the background color, and move it to
/// the bottom of the document. Refuses a group, an adjustment/fill-content
/// layer, or an already-background layer.
//
// ponytail: the background color is opaque white; plumb the toolbox's current
// background color through the bridge when that state is exposed.
pub fn background_from_layer(doc: &mut Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let was_clipped = match resolve_path(doc, path) {
        Some(layer) if !layer.is_group && layer.adjustment.is_none() && !layer.background => {
            layer.clipping
        }
        _ => return false,
    };
    if let Some(layer) = resolve_path_mut(doc, path) {
        opaque_transparency(layer, [255, 255, 255]);
        layer.background = true;
    }
    let node = {
        let Some((container, index)) = container_mut(doc, &segments) else {
            return false;
        };
        release_run_above(container.as_mut_slice(), index, was_clipped);
        let mut node = container.remove(index);
        node.clipping = false;
        normalize_clipping(container.as_mut_slice(), None);
        node
    };
    doc.layers.insert(0, node);
    normalize_clipping(doc.layers.as_mut_slice(), None);
    true
}

/// Set every fully transparent pixel (`alpha == 0`) to `rgb` at full opacity,
/// leaving partially transparent pixels alone.
fn opaque_transparency(layer: &mut Layer, rgb: [u8; 3]) {
    let Some(alpha_index) = layer.channels.iter().position(|channel| channel.id == -1) else {
        return;
    };
    let transparent: Vec<usize> = layer.channels[alpha_index]
        .data
        .iter()
        .enumerate()
        .filter_map(|(index, &alpha)| (alpha == 0).then_some(index))
        .collect();
    if transparent.is_empty() {
        return;
    }
    for channel in layer.channels.iter_mut() {
        let value = match channel.id {
            0 => rgb[0],
            1 => rgb[1],
            2 => rgb[2],
            -1 => 255,
            _ => continue,
        };
        for &index in &transparent {
            if index < channel.data.len() {
                channel.data[index] = value;
            }
        }
    }
}
