//! Generic XMP property tree and path patcher.
//!
//! Reads the simple leaves of one namespace (attributes, element text, lang
//! alternatives, bag items, and one-level struct leaves) and patches them in
//! place. Unknown namespaces, unknown properties, and the packet wrapper are
//! copied verbatim. Shapes the patcher cannot rewrite safely fail closed.

use std::collections::HashMap;

use super::scan::{
    decode_text, push_declarations, restore_declarations, scan_packet, split_qname, str_at,
    tokenize, RawKind,
};
use super::{escape_xml, prefix_for, MAX_PACKET_BYTES, RDF_URI};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    pub path: Vec<String>,
    pub value: String,
    pub span: (usize, usize),
    pub text_span: Option<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub path: Vec<String>,
    pub start: usize,
    pub end: usize,
    pub inner_description_start: Option<(usize, usize)>,
    pub inner_description_name: Option<String>,
    pub inner_description_end: Option<usize>,
    pub unsafe_: bool,
}

#[derive(Debug, Default)]
pub struct Tree {
    pub leaves: Vec<Leaf>,
    pub elements: Vec<Element>,
    pub description_content_end: Option<usize>,
    pub description_self_closing: bool,
    pub description_start_tag: Option<(usize, usize)>,
    pub description_name: Option<String>,
    pub description_scope: HashMap<String, String>,
    pub malformed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Simple,
    LangAlt,
    Bag,
    Struct,
}

#[derive(Debug, Clone)]
pub struct Update {
    pub path: Vec<String>,
    pub shape: Shape,
    pub values: Vec<String>,
}

struct Frame {
    qname: (usize, usize),
    target_path: Vec<String>,
    is_target: bool,
    is_description: bool,
    is_container: bool,
    is_li: bool,
    container_kind: Option<String>,
    li_index: usize,
    start: usize,
    text_start: usize,
    text: Option<Vec<u8>>,
    element: Option<usize>,
    undo: Vec<(String, Option<String>)>,
}

/// Read the target namespace's leaves and target elements.
pub fn read(packet: &str, namespace: &str) -> Tree {
    let bytes = packet.as_bytes();
    if bytes.len() > MAX_PACKET_BYTES {
        return Tree::default();
    }
    let (tokens, complete) = tokenize(bytes);
    let scan = scan_packet(bytes);
    let mut tree = Tree {
        description_content_end: scan.description.as_ref().map(|d| d.content_end),
        description_self_closing: scan.description.as_ref().is_some_and(|d| d.self_closing),
        description_start_tag: scan.description.as_ref().map(|d| d.start_tag),
        description_name: scan
            .description
            .as_ref()
            .map(|d| str_at(bytes, d.qname).to_string()),
        description_scope: scan
            .description
            .as_ref()
            .map(|d| d.scope.clone())
            .unwrap_or_default(),
        malformed: scan.malformed || !complete,
        ..Tree::default()
    };
    if scan.over_limit {
        tree.malformed = true;
        return tree;
    }
    let main_description = scan.description.as_ref().map(|d| d.start_tag.0);
    let mut current: HashMap<String, String> = HashMap::new();
    let mut stack: Vec<Frame> = Vec::new();
    for tok in &tokens {
        if stack.len() > super::MAX_DEPTH {
            tree.malformed = true;
            break;
        }
        match &tok.kind {
            RawKind::Text => {
                if let Some(frame) = stack.iter_mut().rev().find(|f| f.text.is_some()) {
                    let len = tok.end - tok.start;
                    if let Some(buf) = frame.text.as_mut() {
                        if buf.len() + len <= super::MAX_VALUE_BYTES {
                            buf.extend_from_slice(&bytes[tok.start..tok.end]);
                        } else {
                            tree.malformed = true;
                        }
                    }
                }
            }
            RawKind::Ignored => {
                if let Some(element) = stack.iter().rev().find_map(|f| f.element) {
                    tree.elements[element].unsafe_ = true;
                }
            }
            RawKind::Start {
                qname,
                self_closing,
                attrs,
            } => {
                let undo = push_declarations(&mut current, bytes, attrs);
                let name = str_at(bytes, *qname);
                let (prefix, local) = split_qname(name);
                let uri = current.get(prefix).cloned();
                let parent_target = stack
                    .iter()
                    .rev()
                    .find(|f| f.is_target)
                    .map(|f| f.target_path.clone())
                    .unwrap_or_default();
                let is_rdf = uri.as_deref() == Some(RDF_URI);
                let is_description = is_rdf && local == "Description";
                let mut frame = Frame {
                    qname: *qname,
                    target_path: parent_target.clone(),
                    is_target: false,
                    is_description,
                    is_container: false,
                    is_li: false,
                    container_kind: None,
                    li_index: 0,
                    start: tok.start,
                    text_start: tok.end,
                    text: None,
                    element: None,
                    undo,
                };
                if is_description {
                    let is_main = main_description == Some(tok.start);
                    let desc_path = if is_main {
                        Vec::new()
                    } else {
                        parent_target.clone()
                    };
                    if is_main {
                        tree.description_start_tag = Some((tok.start, tok.end));
                        tree.description_name = Some(name.to_string());
                    } else if let Some(element) = stack.iter().rev().find_map(|f| f.element) {
                        tree.elements[element].inner_description_start = Some((tok.start, tok.end));
                        tree.elements[element].inner_description_name = Some(name.to_string());
                    }
                    for attr in attrs {
                        let attr_name = str_at(bytes, attr.name);
                        let (attr_prefix, attr_local) = split_qname(attr_name);
                        if attr_prefix == "xmlns" {
                            continue;
                        }
                        let Some(attr_uri) = current.get(attr_prefix) else {
                            continue;
                        };
                        if attr_uri != namespace {
                            continue;
                        }
                        let decoded = decode_text(&bytes[attr.value.0..attr.value.1]);
                        let mut path = desc_path.clone();
                        path.push(attr_local.to_string());
                        tree.leaves.push(Leaf {
                            path,
                            value: decoded.text,
                            span: (attr.name.0, attr.value.1 + 1),
                            text_span: Some((attr.value.0, attr.value.1)),
                        });
                    }
                    if *self_closing {
                        restore_declarations(&mut current, frame.undo);
                    } else {
                        stack.push(frame);
                    }
                } else if uri.as_deref() == Some(namespace) {
                    let mut path = parent_target.clone();
                    let same = tree
                        .elements
                        .iter()
                        .filter(|element| {
                            element.path.len() == path.len() + 1
                                && element.path[..path.len()] == parent_target[..]
                                && element.path.last().map(String::as_str) == Some(local)
                        })
                        .count();
                    if same == 0 {
                        path.push(local.to_string());
                    } else {
                        path.push(format!("{local}[{same}]"));
                    }
                    let unsafe_ = attrs
                        .iter()
                        .any(|attr| !str_at(bytes, attr.name).starts_with("xmlns"));
                    let element = tree.elements.len();
                    tree.elements.push(Element {
                        path: path.clone(),
                        start: tok.start,
                        end: tok.end,
                        inner_description_start: None,
                        inner_description_name: None,
                        inner_description_end: None,
                        unsafe_,
                    });
                    frame.is_target = true;
                    frame.target_path = path;
                    frame.text = if *self_closing {
                        None
                    } else {
                        Some(Vec::new())
                    };
                    frame.element = Some(element);
                    if !*self_closing {
                        stack.push(frame);
                    } else {
                        restore_declarations(&mut current, frame.undo);
                    }
                } else if is_rdf && matches!(local, "Alt" | "Seq" | "Bag") {
                    frame.is_container = true;
                    frame.container_kind = Some(local.to_string());
                    if !*self_closing {
                        stack.push(frame);
                    } else {
                        restore_declarations(&mut current, frame.undo);
                    }
                } else if is_rdf && local == "li" {
                    let lang = attrs.iter().find_map(|attr| {
                        (str_at(bytes, attr.name) == "xml:lang")
                            .then(|| decode_text(&bytes[attr.value.0..attr.value.1]).text)
                    });
                    let container = stack.iter_mut().rev().find(|f| f.is_container && !f.is_li);
                    let (mut path, index, kind) = match container {
                        Some(container) => {
                            let index = container.li_index;
                            container.li_index += 1;
                            (
                                container.target_path.clone(),
                                index,
                                container.container_kind.clone(),
                            )
                        }
                        None => (parent_target.clone(), 0, None),
                    };
                    if kind.as_deref() == Some("Alt") {
                        path.push(format!("li[{}]", lang.as_deref().unwrap_or("x-default")));
                    } else {
                        path.push(format!("li[{index}]"));
                    }
                    frame.target_path = path;
                    frame.is_li = true;
                    frame.container_kind = kind;
                    frame.element = None;
                    if *self_closing {
                        tree.leaves.push(Leaf {
                            path: frame.target_path.clone(),
                            value: String::new(),
                            span: (tok.start, tok.end),
                            text_span: None,
                        });
                        restore_declarations(&mut current, frame.undo);
                    } else {
                        frame.text = Some(Vec::new());
                        stack.push(frame);
                    }
                } else if !*self_closing {
                    stack.push(frame);
                } else {
                    restore_declarations(&mut current, frame.undo);
                }
            }
            RawKind::End { qname } => {
                let Some(frame) = stack.pop() else {
                    tree.malformed = true;
                    break;
                };
                if str_at(bytes, *qname) != str_at(bytes, frame.qname) {
                    tree.malformed = true;
                    break;
                }
                if frame.is_description {
                    if let Some(element) = stack.iter().rev().find_map(|f| f.element) {
                        tree.elements[element].inner_description_end = Some(tok.start);
                    }
                } else if frame.is_target {
                    if let Some(element) = frame.element {
                        tree.elements[element].end = tok.end;
                    }
                    if let Some(text) = frame.text {
                        let value = String::from_utf8_lossy(&text).trim().to_string();
                        if !value.is_empty() {
                            tree.leaves.push(Leaf {
                                path: frame.target_path.clone(),
                                value,
                                span: (frame.start, tok.end),
                                text_span: Some((frame.text_start, tok.start)),
                            });
                        }
                    }
                } else if frame.is_li {
                    if let Some(text) = frame.text {
                        let value = String::from_utf8_lossy(&text).trim().to_string();
                        if !value.is_empty() {
                            tree.leaves.push(Leaf {
                                path: frame.target_path.clone(),
                                value,
                                span: (frame.start, tok.end),
                                text_span: Some((frame.text_start, tok.start)),
                            });
                        }
                    }
                }
                restore_declarations(&mut current, frame.undo);
            }
        }
    }
    tree
}

/// Patch or create the requested properties. Returns `None` when a shape
/// cannot be rewritten safely or the edits overlap.
pub fn patch(packet: &str, namespace: &str, updates: &[Update]) -> Option<String> {
    let tree = read(packet, namespace);
    if tree.malformed {
        return None;
    }
    let bytes = packet.as_bytes();
    let prefix = canonical_prefix(namespace);
    let rdf = prefix_for(&tree.description_scope, RDF_URI).unwrap_or_else(|| "rdf".to_string());
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for update in updates {
        let values: Vec<&str> = update
            .values
            .iter()
            .filter(|value| !value.is_empty())
            .map(String::as_str)
            .collect();
        if update.path.is_empty() {
            return None;
        }
        if update.shape == Shape::Struct {
            if update.path.len() != 2 {
                return None;
            }
            if let Some(leaf) = tree.leaves.iter().find(|leaf| leaf.path == update.path) {
                if tree
                    .elements
                    .iter()
                    .any(|element| element.path == vec![update.path[0].clone()] && element.unsafe_)
                {
                    return None;
                }
                if values.is_empty() {
                    edits.push((leaf.span.0, leaf.span.1, String::new()));
                } else if leaf.value == values[0] {
                    continue;
                } else if let Some((start, end)) = leaf.text_span {
                    edits.push((start, end, escape_xml(values[0])));
                } else {
                    return None;
                }
                continue;
            }
            if values.is_empty() {
                continue;
            }
            let parent = tree
                .elements
                .iter()
                .find(|element| element.path == vec![update.path[0].clone()]);
            let Some(parent) = parent else {
                let fragment = format!(
                    "<{prefix}:{parent_name} xmlns:{prefix}=\"{namespace}\"><{rdf}:Description><{prefix}:{leaf_name} xmlns:{prefix}=\"{namespace}\">{}</{prefix}:{leaf_name}></{rdf}:Description></{prefix}:{parent_name}>",
                    escape_xml(values[0]),
                    parent_name = update.path[0],
                    leaf_name = update.path[1]
                );
                if tree.description_self_closing {
                    let (start, end) = tree.description_start_tag?;
                    let raw = bytes.get(start..end)?;
                    let head = raw.strip_suffix(b"/>")?;
                    let name = tree.description_name.as_deref()?;
                    edits.push((
                        start,
                        end,
                        format!(
                            "{}>{}</{}>",
                            std::str::from_utf8(head).ok()?,
                            fragment,
                            name
                        ),
                    ));
                } else {
                    let at = tree.description_content_end?;
                    edits.push((at, at, fragment));
                }
                continue;
            };
            if parent.unsafe_ {
                return None;
            }
            let leaf_name = &update.path[1];
            if let Some(at) = parent.inner_description_end {
                edits.push((
                    at,
                    at,
                    format!(
                        "<{prefix}:{leaf_name} xmlns:{prefix}=\"{namespace}\">{}</{prefix}:{leaf_name}>",
                        escape_xml(values[0])
                    ),
                ));
                continue;
            }
            if let Some((start, end)) = parent.inner_description_start {
                let raw = bytes.get(start..end)?;
                let head = raw.strip_suffix(b"/>")?;
                let name = parent.inner_description_name.as_deref()?;
                edits.push((
                    start,
                    end,
                    format!(
                        "{}><{prefix}:{leaf_name} xmlns:{prefix}=\"{namespace}\">{}</{prefix}:{leaf_name}></{}>",
                        std::str::from_utf8(head).ok()?,
                        escape_xml(values[0]),
                        name
                    ),
                ));
                continue;
            }
            return None;
        }
        if update.shape == Shape::LangAlt {
            let has_other = tree.leaves.iter().any(|leaf| {
                leaf.path.len() == 2
                    && leaf.path[0] == update.path[0]
                    && leaf.path[1].starts_with("li[")
                    && leaf.path[1] != "li[x-default]"
            });
            if has_other {
                return None;
            }
            if let Some(leaf) = tree.leaves.iter().find(|leaf| {
                leaf.path.len() == 2
                    && leaf.path[0] == update.path[0]
                    && leaf.path[1] == "li[x-default]"
            }) {
                if values.len() == 1 && leaf.value == values[0] {
                    continue;
                }
            }
        }
        if update.shape == Shape::Bag {
            let current: Vec<&str> = tree
                .leaves
                .iter()
                .filter(|leaf| {
                    leaf.path.len() == 2
                        && leaf.path[0] == update.path[0]
                        && leaf.path[1].starts_with("li[")
                })
                .map(|leaf| leaf.value.as_str())
                .collect();
            if !current.is_empty() && current == values {
                continue;
            }
        }
        if update.shape != Shape::Bag {
            if let Some(leaf) = tree.leaves.iter().find(|leaf| leaf.path == update.path) {
                if tree
                    .elements
                    .iter()
                    .any(|element| element.path == vec![update.path[0].clone()] && element.unsafe_)
                {
                    return None;
                }
                if values.is_empty() {
                    edits.push((leaf.span.0, leaf.span.1, String::new()));
                } else if leaf.value == values[0] {
                    continue;
                } else if let Some((start, end)) = leaf.text_span {
                    edits.push((start, end, escape_xml(values[0])));
                } else {
                    return None;
                }
                continue;
            }
        }
        let existing = tree
            .elements
            .iter()
            .find(|element| element.path.len() == 1 && element.path[0] == update.path[0]);
        if values.is_empty() {
            if let Some(element) = existing {
                if element.unsafe_ {
                    return None;
                }
                edits.push((element.start, element.end, String::new()));
                continue;
            }
            let leaf = tree.leaves.iter().find(|leaf| leaf.path == update.path);
            if let Some(leaf) = leaf {
                edits.push((leaf.span.0, leaf.span.1, String::new()));
                continue;
            }
            continue;
        }
        if let Some(element) = existing {
            if element.unsafe_ {
                return None;
            }
            let body = match update.shape {
                Shape::Simple => values[0].to_string(),
                Shape::LangAlt => format!(
                    "<{rdf}:Alt><{rdf}:li xml:lang=\"x-default\">{}</{rdf}:li></{rdf}:Alt>",
                    escape_xml(values[0])
                ),
                Shape::Bag => format!(
                    "<{rdf}:Bag>{}</{rdf}:Bag>",
                    values
                        .iter()
                        .map(|v| format!("<{rdf}:li>{}</{rdf}:li>", escape_xml(v)))
                        .collect::<String>()
                ),
                Shape::Struct => return None,
            };
            let name = format!("{prefix}:{}", update.path[0]);
            edits.push((
                element.start,
                element.end,
                format!("<{name} xmlns:{prefix}=\"{namespace}\">{body}</{name}>"),
            ));
            continue;
        }
        if update.shape == Shape::Bag && tree.leaves.iter().any(|leaf| leaf.path == update.path) {
            return None;
        }
        let name = format!("{prefix}:{}", update.path[0]);
        let body = match update.shape {
            Shape::Simple => escape_xml(values[0]),
            Shape::LangAlt => format!(
                "<{rdf}:Alt><{rdf}:li xml:lang=\"x-default\">{}</{rdf}:li></{rdf}:Alt>",
                escape_xml(values[0])
            ),
            Shape::Bag => format!(
                "<{rdf}:Bag>{}</{rdf}:Bag>",
                values
                    .iter()
                    .map(|v| format!("<{rdf}:li>{}</{rdf}:li>", escape_xml(v)))
                    .collect::<String>()
            ),
            Shape::Struct => return None,
        };
        let fragment = format!("<{name} xmlns:{prefix}=\"{namespace}\">{body}</{name}>");
        if tree.description_self_closing {
            let (start, end) = tree.description_start_tag?;
            let raw = bytes.get(start..end)?;
            let head = raw.strip_suffix(b"/>")?;
            let name = tree.description_name.as_deref()?;
            edits.push((
                start,
                end,
                format!(
                    "{}>{}</{}>",
                    std::str::from_utf8(head).ok()?,
                    fragment,
                    name
                ),
            ));
        } else {
            let at = tree.description_content_end?;
            edits.push((at, at, fragment));
        }
    }
    if edits.is_empty() {
        return Some(packet.to_string());
    }
    edits.sort_by_key(|edit| (edit.0, edit.1));
    for pair in edits.windows(2) {
        if pair[1].0 < pair[0].1 {
            return None;
        }
    }
    let mut out = String::with_capacity(packet.len() + 128);
    let mut cursor = 0;
    for (start, end, fragment) in edits {
        if start < cursor || end > bytes.len() {
            return None;
        }
        out.push_str(&packet[cursor..start]);
        out.push_str(&fragment);
        cursor = end;
    }
    out.push_str(&packet[cursor..]);
    Some(out)
}

fn canonical_prefix(namespace: &str) -> &'static str {
    match namespace {
        "http://iptc.org/std/Iptc4xmpExt/2008-02-29/" => "Iptc4xmpExt",
        "http://ns.adobe.com/xap/1.0/rights/" => "xmpRights",
        "http://ns.adobe.com/photoshop/1.0/" => "photoshop",
        "http://ns.adobe.com/xap/1.0/mm/" => "xmpMM",
        "http://ns.adobe.com/xap/1.0/" => "xmp",
        _ => "ns",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "http://iptc.org/std/Iptc4xmpExt/2008-02-29/";
    const HEAD: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">";
    const TAIL: &str = "</rdf:RDF></x:xmpmeta>";

    fn packet(description: &str) -> String {
        format!("{HEAD}{description}{TAIL}")
    }

    fn description(attrs: &str, body: &str) -> String {
        format!(
            "<rdf:Description xmlns:Iptc4xmpExt=\"{NS}\" xmlns:acme=\"http://example.com/acme/\" {attrs}>{body}</rdf:Description>"
        )
    }

    #[test]
    fn xmp_reads_attribute_and_element_forms() {
        let tree = read(
            &packet(&description(
                "Iptc4xmpExt:Event=\"Fixture Event\"",
                "<Iptc4xmpExt:ImageRating>4</Iptc4xmpExt:ImageRating>",
            )),
            NS,
        );
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["Event"] && leaf.value == "Fixture Event"));
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["ImageRating"] && leaf.value == "4"));
    }

    #[test]
    fn xmp_resolves_namespace_prefixes() {
        let packet = packet(
            "<rdf:Description xmlns:e=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\"><e:Event>One</e:Event></rdf:Description>",
        );
        let tree = read(&packet, NS);
        assert!(tree.leaves.iter().any(|leaf| leaf.path == ["Event"]));
    }

    #[test]
    fn xmp_reads_bag_and_struct_leaves() {
        let body = "<Iptc4xmpExt:PersonInImage><rdf:Bag><rdf:li>Ada</rdf:li></rdf:Bag></Iptc4xmpExt:PersonInImage>\
                    <Iptc4xmpExt:LocationCreated><rdf:Description Iptc4xmpExt:City=\"Paris\"/></Iptc4xmpExt:LocationCreated>";
        let tree = read(&packet(&description("", body)), NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["PersonInImage", "li[0]"] && leaf.value == "Ada"));
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| { leaf.path == ["LocationCreated", "City"] && leaf.value == "Paris" }));
    }

    #[test]
    fn xmp_malformed_packet_reads_what_it_can() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event>Keep</Iptc4xmpExt:Event><Iptc4xmpExt:ImageRating>4",
        ));
        let tree = read(&packet, NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["Event"] && leaf.value == "Keep"));
    }

    #[test]
    fn xmp_patch_preserves_unknown_namespace_bytes() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event>Old</Iptc4xmpExt:Event><acme:Note>keep</acme:Note>",
        ));
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::LangAlt,
            values: vec!["New".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        assert!(out.contains("New"));
        assert!(out.contains("<acme:Note>keep</acme:Note>"));
    }

    #[test]
    fn xmp_patch_creates_a_bag() {
        let packet = packet(&description("", ""));
        let update = Update {
            path: vec!["PersonInImage".to_string()],
            shape: Shape::Bag,
            values: vec!["Ada".to_string(), "Grace".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        let tree = read(&out, NS);
        let values: Vec<_> = tree
            .leaves
            .iter()
            .filter(|leaf| leaf.path.first().map(String::as_str) == Some("PersonInImage"))
            .map(|leaf| leaf.value.as_str())
            .collect();
        assert_eq!(values, vec!["Ada", "Grace"]);
    }

    #[test]
    fn xmp_patch_creates_a_struct_with_local_namespace() {
        let packet =
            packet("<rdf:Description xmlns:acme=\"http://example.com/acme/\"></rdf:Description>");
        let update = Update {
            path: vec!["LocationCreated".to_string(), "City".to_string()],
            shape: Shape::Struct,
            values: vec!["Paris".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        assert!(out.contains("xmlns:Iptc4xmpExt"));
        let tree = read(&out, NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["LocationCreated", "City"] && leaf.value == "Paris"));
    }

    #[test]
    fn xmp_patch_fails_closed_on_unsupported_shape() {
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::Simple,
            values: vec!["x".to_string()],
        };
        assert!(patch("<not-xmp/>", NS, &[update]).is_none());
    }

    #[test]
    fn xmp_patch_removes_only_the_property() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event>Old</Iptc4xmpExt:Event><acme:Note>keep</acme:Note>",
        ));
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::LangAlt,
            values: Vec::new(),
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        assert!(!out.contains("Iptc4xmpExt:Event"));
        assert!(out.contains("<acme:Note>keep</acme:Note>"));
    }
    #[test]
    fn xmp_patch_reuses_a_non_canonical_prefix() {
        let packet = packet(
            "<rdf:Description xmlns:e=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\">\
             <e:PersonInImage><rdf:Bag><rdf:li>Old</rdf:li></rdf:Bag></e:PersonInImage>\
             </rdf:Description>",
        );
        let update = Update {
            path: vec!["PersonInImage".to_string()],
            shape: Shape::Bag,
            values: vec!["New".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        assert!(out.contains("xmlns:Iptc4xmpExt"));
        let tree = read(&out, NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["PersonInImage", "li[0]"] && leaf.value == "New"));
    }

    #[test]
    fn xmp_patch_adds_a_leaf_to_an_attribute_struct() {
        let packet = packet(
            "<rdf:Description xmlns:Iptc4xmpExt=\"http://iptc.org/std/Iptc4xmpExt/2008-02-29/\">\
             <Iptc4xmpExt:LocationCreated><rdf:Description Iptc4xmpExt:City=\"Paris\"/></Iptc4xmpExt:LocationCreated>\
             </rdf:Description>",
        );
        let update = Update {
            path: vec!["LocationCreated".to_string(), "ProvinceState".to_string()],
            shape: Shape::Struct,
            values: vec!["Ile-de-France".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        let tree = read(&out, NS);
        assert!(tree.leaves.iter().any(|leaf| {
            leaf.path == ["LocationCreated", "ProvinceState"] && leaf.value == "Ile-de-France"
        }));
    }

    #[test]
    fn xmp_patch_creates_a_property_in_a_self_closing_description() {
        let packet = packet("<rdf:Description xmlns:acme=\"http://example.com/acme/\"/>");
        let update = Update {
            path: vec!["PersonInImage".to_string()],
            shape: Shape::Bag,
            values: vec!["Ada".to_string()],
        };
        let out = patch(&packet, NS, &[update]).expect("patch");
        let tree = read(&out, NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["PersonInImage", "li[0]"] && leaf.value == "Ada"));
    }

    #[test]
    fn xmp_patch_is_a_noop_for_equal_values() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event>Old</Iptc4xmpExt:Event>",
        ));
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::LangAlt,
            values: vec!["Old".to_string()],
        };
        assert_eq!(
            patch(&packet, NS, &[update]).as_deref(),
            Some(packet.as_str())
        );
    }

    #[test]
    fn xmp_patch_fails_closed_on_a_comment() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event>Old<!-- keep --></Iptc4xmpExt:Event>",
        ));
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::LangAlt,
            values: vec!["New".to_string()],
        };
        assert!(patch(&packet, NS, &[update]).is_none());
    }
    #[test]
    fn xmp_reads_lang_alternatives_by_lang() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event><rdf:Alt><rdf:li xml:lang=\"fr\">Evenement</rdf:li><rdf:li xml:lang=\"x-default\">Event</rdf:li></rdf:Alt></Iptc4xmpExt:Event>",
        ));
        let tree = read(&packet, NS);
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["Event", "li[fr]"] && leaf.value == "Evenement"));
        assert!(tree
            .leaves
            .iter()
            .any(|leaf| leaf.path == ["Event", "li[x-default]"] && leaf.value == "Event"));
    }

    #[test]
    fn xmp_patch_fails_closed_on_a_multilanguage_alt() {
        let packet = packet(&description(
            "",
            "<Iptc4xmpExt:Event><rdf:Alt><rdf:li xml:lang=\"fr\">Evenement</rdf:li><rdf:li xml:lang=\"x-default\">Event</rdf:li></rdf:Alt></Iptc4xmpExt:Event>",
        ));
        let update = Update {
            path: vec!["Event".to_string()],
            shape: Shape::LangAlt,
            values: vec!["New".to_string()],
        };
        assert!(patch(&packet, NS, &[update]).is_none());
    }
}
