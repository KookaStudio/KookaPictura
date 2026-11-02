//! Whether a just-inserted node leaves the document composite unchanged, so the
//! app can skip the recomposite for New Layer / New Group.

use pictura_core::{BitDepth, BlendMode, ColorMode, Document, Knockout, Layer};
use rayon::prelude::*;

use super::paths::{container_of, parse_path};

/// Whether the node just inserted at `path` is content-neutral: the stored
/// composite of the document before the insertion still equals
/// [`crate::composite_rgba`] of the document after it.
///
/// Clipped layers attach to the nearest unclipped sibling below them, and a
/// base that gains its first clipped layer composites through an isolated
/// group, so the clipping rules below are about not moving or starting a run.
pub fn insertion_is_content_neutral(doc: &Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let Some((&at, parent)) = segments.split_last() else {
        return false;
    };
    let Some(container) = container_of(doc, parent) else {
        return false;
    };
    let Some(node) = container.get(at) else {
        return false;
    };
    // An empty document's stored composite is not a composite of its layers
    // (it is the opened image), so its first layer must recomposite.
    let had_layers = !parent.is_empty() || doc.layers.len() > 1;
    let above_clipped = container.get(at + 1).is_some_and(|l| l.clipping);
    let joins_run = at
        .checked_sub(1)
        .and_then(|i| container.get(i))
        .is_some_and(|l| l.clipping);
    had_layers && is_inert(node) && !above_clipped && (!node.clipping || joins_run)
}

/// Tagged blocks that carry layer metadata and draw nothing.
const METADATA_BLOCKS: [&[u8; 4]; 5] = [b"luni", b"lyid", b"lclr", b"lspf", b"shmd"];

/// Whether the copy just inserted at `path`, directly above its source, leaves
/// the stored composite valid: an opaque copy of an opaque layer shows what the
/// source already showed, so the backdrop of everything above is unchanged.
pub fn duplicate_is_content_neutral(doc: &Document, path: &str) -> bool {
    // ponytail: 8-bit RGB only; other modes and depths recomposite, as before.
    if doc.mode != ColorMode::Rgb || doc.depth != BitDepth::Eight {
        return false;
    }
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let Some((&at, parent)) = segments.split_last() else {
        return false;
    };
    let Some(container) = container_of(doc, parent) else {
        return false;
    };
    let source = at.checked_sub(1).and_then(|i| container.get(i));
    let above_clipped = container.get(at + 1).is_some_and(|l| l.clipping);
    !above_clipped
        && !doc
            .layers
            .iter()
            .any(crate::composite_knockout::has_knockout)
        && source.is_some_and(is_plain_opaque)
        && container.get(at).is_some_and(is_plain_opaque)
}

/// A pixel layer that replaces its backdrop outright across its rect.
fn is_plain_opaque(node: &Layer) -> bool {
    let plain = !node.is_group
        && !node.clipping
        && node.visible
        && node.blend == BlendMode::Normal
        && node.blend_key.is_none()
        && node.opacity == 255
        && node.fill == 255
        && node.mask.is_none()
        && node.vector_mask.is_none()
        && node.adjustment.is_none()
        && node.smart_object.is_none()
        && node.type_tool.is_none()
        && node.raw_channels.is_empty()
        && node.blend_if.is_none()
        && node.knockout == Knockout::None
        && node
            .extra_blocks
            .iter()
            .all(|block| METADATA_BLOCKS.contains(&&block.key));
    if !plain {
        return false;
    }
    match node.channels.iter().find(|c| c.id == -1) {
        Some(alpha) => {
            let bands: Vec<&[u8]> = alpha.data.bands().collect();
            bands.iter().all(|band| {
                band.par_chunks(1 << 16)
                    .all(|chunk| chunk.iter().all(|&a| a == 255))
            })
        }
        None => node.background,
    }
}

/// A node that draws nothing in any blend mode: an empty group, or a pixel
/// layer whose alpha is all zero, with nothing that renders around it.
fn is_inert(node: &Layer) -> bool {
    let bare = node.mask.is_none()
        && node.vector_mask.is_none()
        && node.adjustment.is_none()
        && node.smart_object.is_none()
        && node.type_tool.is_none()
        && node.extra_blocks.is_empty()
        && node.raw_channels.is_empty()
        && node.blend_if.is_none()
        && node.knockout == Knockout::None;
    if !bare {
        return false;
    }
    if node.is_group {
        return node.children.is_empty();
    }
    node.channels
        .iter()
        .find(|c| c.id == -1)
        .is_some_and(|alpha| alpha.data.iter().all(|&a| a == 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_ops::layer_ops::{add_layer_full, insert_node, NewLayerSpec};
    use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, LayerStack, PsdRect};

    const W: u32 = 6;
    const H: u32 = 5;

    fn pixel(name: &str, value: u8, alpha: u8) -> Layer {
        let n = (W * H) as usize;
        Layer {
            name: name.into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: H as i32,
                right: W as i32,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            visible: true,
            channels: [0i16, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![if id == -1 { alpha } else { value }; n].into(),
                })
                .to_vec(),
            ..Default::default()
        }
    }

    fn clipped(name: &str, value: u8) -> Layer {
        Layer {
            clipping: true,
            blend: BlendMode::Multiply,
            ..pixel(name, value, 200)
        }
    }

    fn group(blend: BlendMode, children: Vec<Layer>) -> Layer {
        Layer {
            name: "group".into(),
            blend,
            opacity: 255,
            fill: 255,
            visible: true,
            is_group: true,
            children: LayerStack::from_layers(children),
            ..Default::default()
        }
    }

    fn doc(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    fn spec(clipping: bool) -> NewLayerSpec {
        NewLayerSpec {
            clipping,
            ..NewLayerSpec::default()
        }
    }

    #[test]
    fn insertion_under_clipped_sibling_is_not_neutral() {
        let mut d = doc(vec![pixel("base", 90, 255), clipped("clip", 40)]);
        // Selecting the base inserts directly above it, under the clipped layer.
        let path = add_layer_full(&mut d, "0", &spec(false));
        assert_eq!(path, "1");
        assert!(!insertion_is_content_neutral(&d, &path));
    }

    #[test]
    fn first_clipped_layer_is_not_neutral() {
        let mut d = doc(vec![pixel("base", 90, 255)]);
        let path = add_layer_full(&mut d, "0", &spec(true));
        assert!(!insertion_is_content_neutral(&d, &path));
    }

    #[test]
    fn joining_a_clipped_run_is_neutral() {
        let mut d = doc(vec![pixel("base", 90, 255), clipped("clip", 40)]);
        let path = add_layer_full(&mut d, "1", &spec(true));
        assert_eq!(path, "2");
        assert!(insertion_is_content_neutral(&d, &path));
    }

    #[test]
    fn first_layer_of_empty_document_is_not_neutral() {
        let mut d = doc(Vec::new());
        let path = add_layer_full(&mut d, "", &spec(false));
        assert!(!insertion_is_content_neutral(&d, &path));
    }

    #[test]
    fn mode_neutral_fill_is_not_neutral() {
        let mut d = doc(vec![pixel("base", 90, 255)]);
        let overlay = NewLayerSpec {
            blend: BlendMode::Overlay,
            neutral_fill: true,
            ..NewLayerSpec::default()
        };
        let path = add_layer_full(&mut d, "0", &overlay);
        assert!(!insertion_is_content_neutral(&d, &path));
    }

    #[test]
    fn a_plain_new_layer_is_neutral() {
        let mut d = doc(vec![pixel("base", 90, 255)]);
        let path = add_layer_full(&mut d, "0", &spec(false));
        assert!(insertion_is_content_neutral(&d, &path));
    }

    fn background() -> Layer {
        let mut layer = pixel("Background", 90, 255);
        layer.channels.retain(|c| c.id != -1);
        layer.background = true;
        layer
    }

    #[test]
    fn neutral_duplicates_preserve_composite() {
        let top = pixel("top", 200, 120);
        let stacks: Vec<(Vec<Layer>, &str)> = vec![
            (vec![background()], "0"),
            (vec![background(), top.clone()], "0"),
            (vec![pixel("opaque", 70, 255), top.clone()], "0"),
            (
                vec![
                    background(),
                    group(BlendMode::PassThrough, vec![pixel("in", 50, 255)]),
                ],
                "1/0",
            ),
            (
                vec![
                    background(),
                    group(BlendMode::Multiply, vec![pixel("in", 50, 255)]),
                ],
                "1/0",
            ),
        ];
        for (stack, source) in stacks {
            let mut d = doc(stack);
            let expected = crate::composite_rgba(&d);
            let created = crate::duplicate_paths(&mut d, &[source]);
            assert!(duplicate_is_content_neutral(&d, &created[0]), "{source}");
            assert_eq!(crate::composite_rgba(&d), expected, "{source}");
            // The copy is itself duplicable, as a second Ctrl+J does.
            let again = crate::duplicate_paths(&mut d, &[created[0].as_str()]);
            assert!(duplicate_is_content_neutral(&d, &again[0]), "{source}");
            assert_eq!(crate::composite_rgba(&d), expected, "{source} twice");
        }
    }

    #[test]
    fn excluded_duplicates_are_not_neutral() {
        type Tweak = (&'static str, fn(&mut Layer));
        let tweaks: Vec<Tweak> = vec![
            ("translucent", |l| {
                l.channels.iter_mut().find(|c| c.id == -1).unwrap().data =
                    vec![200; (W * H) as usize].into();
            }),
            ("one soft pixel", |l| {
                l.channels
                    .iter_mut()
                    .find(|c| c.id == -1)
                    .unwrap()
                    .data
                    .as_mut_slice()[7] = 254;
            }),
            ("hidden", |l| l.visible = false),
            ("multiply", |l| l.blend = BlendMode::Multiply),
            ("opacity", |l| l.opacity = 128),
            ("fill", |l| l.fill = 128),
            ("clipped", |l| l.clipping = true),
            ("knockout", |l| l.knockout = Knockout::Shallow),
            ("effects", |l| {
                l.extra_blocks.push(pictura_core::LayerBlock {
                    key: *b"lfx2",
                    ..Default::default()
                })
            }),
            ("no alpha, not a Background", |l| {
                l.channels.retain(|c| c.id != -1)
            }),
        ];
        for (name, tweak) in tweaks {
            let mut source = pixel("source", 90, 255);
            tweak(&mut source);
            let mut d = doc(vec![pixel("base", 10, 255), source]);
            let created = crate::duplicate_paths(&mut d, &["1"]);
            assert!(!duplicate_is_content_neutral(&d, &created[0]), "{name}");
        }

        let mut d = doc(vec![pixel("base", 90, 255), clipped("clip", 40)]);
        let created = crate::duplicate_paths(&mut d, &["0"]);
        assert!(
            !duplicate_is_content_neutral(&d, &created[0]),
            "clipped above"
        );

        let mut d = doc(vec![pixel("base", 90, 255)]);
        d.depth = BitDepth::Sixteen;
        let created = crate::duplicate_paths(&mut d, &["0"]);
        assert!(!duplicate_is_content_neutral(&d, &created[0]), "depth");

        let mut d = doc(vec![pixel("base", 90, 255)]);
        d.mode = ColorMode::Cmyk;
        let created = crate::duplicate_paths(&mut d, &["0"]);
        assert!(!duplicate_is_content_neutral(&d, &created[0]), "mode");
    }

    #[test]
    fn metadata_blocks_do_not_exclude_a_duplicate() {
        let mut base = pixel("base", 90, 255);
        base.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"luni",
            ..Default::default()
        });
        let mut d = doc(vec![base]);
        let created = crate::duplicate_paths(&mut d, &["0"]);
        assert!(duplicate_is_content_neutral(&d, &created[0]));
    }

    #[test]
    fn a_background_copy_gets_an_opaque_alpha_plane() {
        let mut d = doc(vec![background()]);
        let created = crate::duplicate_paths(&mut d, &["0"]);
        let copy = crate::resolve_path(&d, &created[0]).unwrap();
        let alpha = copy.channels.iter().find(|c| c.id == -1).unwrap();
        assert_eq!(alpha.data.len(), (W * H) as usize);
        assert!(alpha.data.iter().all(|&a| a == 255));
    }

    fn all_paths(layers: &[Layer], prefix: &str, out: &mut Vec<String>) {
        for (i, layer) in layers.iter().enumerate() {
            let path = if prefix.is_empty() {
                i.to_string()
            } else {
                format!("{prefix}/{i}")
            };
            out.push(path.clone());
            all_paths(&layer.children, &path, out);
        }
    }

    /// Insert every kind of fresh node above every node of stacks with
    /// clipping, groups, pass-through, and knockout; every insertion the
    /// predicate calls neutral must leave the composite byte-identical.
    #[test]
    fn neutral_insertions_preserve_composite() {
        let knockout = Layer {
            knockout: Knockout::Shallow,
            ..pixel("ko", 30, 180)
        };
        let stacks = vec![
            vec![
                pixel("base", 90, 255),
                clipped("c1", 40),
                clipped("c2", 200),
            ],
            vec![
                pixel("bg", 10, 255),
                group(
                    BlendMode::PassThrough,
                    vec![pixel("in", 120, 128), clipped("inclip", 60)],
                ),
                pixel("top", 220, 90),
            ],
            vec![
                pixel("bg", 10, 255),
                group(BlendMode::Screen, vec![pixel("a", 70, 255), knockout]),
            ],
            vec![group(BlendMode::Normal, Vec::new())],
        ];
        let mut checked = 0;
        for stack in stacks {
            let base = doc(stack);
            let expected = crate::composite_rgba(&base);
            let mut targets = Vec::new();
            all_paths(&base.layers, "", &mut targets);
            for target in &targets {
                let fresh = [
                    super::super::create::transparent_layer(W, H, "new"),
                    Layer {
                        clipping: true,
                        ..super::super::create::transparent_layer(W, H, "new clip")
                    },
                    super::super::create::empty_group("new group"),
                ];
                for node in fresh {
                    let mut d = base.clone();
                    let path = insert_node(&mut d, target, node);
                    assert!(!path.is_empty(), "insert above {target}");
                    if insertion_is_content_neutral(&d, &path) {
                        assert_eq!(
                            crate::composite_rgba(&d),
                            expected,
                            "neutral insertion at {path} changed the composite"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 10, "the stacks exercise the neutral path");
    }
}
