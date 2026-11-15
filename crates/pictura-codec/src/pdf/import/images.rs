//! Images mode (design D4): the image XObjects and inline images a PDF's pages
//! draw, and a one-page PDF that draws one of them full-page so Ghostscript
//! decodes it.

use std::collections::{HashMap, HashSet};

use super::objects::{decode_stream, write_object, Dict, Object, PdfObjects};
use super::{MAX_IMAGES, MAX_PAGES};

/// The deepest page-tree or Form XObject nesting followed.
const MAX_TREE_DEPTH: u32 = 64;

/// One image: the page it was first found on (1-based), its object number when
/// it is an XObject, and its pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfImage {
    pub page: u32,
    pub object: Option<u32>,
    pub width: u32,
    pub height: u32,
}

/// Where one listed image's samples come from.
#[derive(Debug, Clone)]
pub(crate) enum ImageSource {
    XObject(u32),
    Inline(Dict, Vec<u8>),
}

pub(crate) struct Listed {
    pub images: Vec<PdfImage>,
    pub sources: Vec<ImageSource>,
}

struct PageEntry {
    resources: Option<Object>,
    contents: Option<Object>,
}

/// The distinct images of every page, in page order: XObjects (including those
/// inside Form XObjects), then inline images in page content-stream order.
/// Images in patterns and inline images inside Form XObject content streams are
/// not listed. A page content stream with an unsupported filter yields no
/// inline images but does not stop the other streams or the XObject listing. At
/// most [`MAX_PAGES`] pages are searched and [`MAX_IMAGES`] images listed.
/// `None` when the file cannot be read or is encrypted.
pub(crate) fn list_images(file: &PdfObjects) -> Option<Listed> {
    if file.encrypted() {
        return None;
    }
    let catalog = file.resolve_dict(file.trailer.get("Root"))?;
    let mut pages = Vec::new();
    collect_pages(
        file,
        catalog.get("Pages")?,
        None,
        &mut pages,
        &mut HashSet::new(),
        0,
    );
    let mut seen = HashSet::new();
    let mut listed = Listed {
        images: Vec::new(),
        sources: Vec::new(),
    };
    for (index, page) in pages.iter().enumerate() {
        if listed.images.len() >= MAX_IMAGES {
            break;
        }
        let mut forms = HashSet::new();
        collect_images(
            file,
            page.resources.as_ref(),
            index as u32 + 1,
            &mut seen,
            &mut forms,
            &mut listed,
            0,
        );
        if listed.images.len() >= MAX_IMAGES {
            break;
        }
        if let Some(contents) = page.contents.as_ref() {
            if let Some(data) = content_bytes(file, contents) {
                let remaining = MAX_IMAGES.saturating_sub(listed.images.len());
                for (dict, data) in inline_images(&data, remaining) {
                    let width = size(file, &dict, "Width");
                    let height = size(file, &dict, "Height");
                    if let (Some(width), Some(height)) = (width, height) {
                        listed.images.push(PdfImage {
                            page: index as u32 + 1,
                            object: None,
                            width,
                            height,
                        });
                        listed.sources.push(ImageSource::Inline(dict, data));
                    }
                }
            }
        }
    }
    listed.images.truncate(MAX_IMAGES);
    listed.sources.truncate(MAX_IMAGES);
    Some(listed)
}

/// Each leaf page's effective `/Resources` and `/Contents` (resources are
/// inherited from its ancestors).
fn collect_pages(
    file: &PdfObjects,
    node: &Object,
    inherited: Option<&Object>,
    pages: &mut Vec<PageEntry>,
    visited: &mut HashSet<u32>,
    depth: u32,
) {
    if depth > MAX_TREE_DEPTH || pages.len() >= MAX_PAGES {
        return;
    }
    if let Object::Ref(num) = node {
        if !visited.insert(*num) {
            return;
        }
    }
    let Some(dict) = file.resolve(node).dict() else {
        return;
    };
    let resources = dict.get("Resources").or(inherited);
    match dict.get("Kids").map(|k| file.resolve(k)) {
        Some(Object::Array(kids)) => {
            for kid in kids {
                collect_pages(file, kid, resources, pages, visited, depth + 1);
            }
        }
        _ => pages.push(PageEntry {
            resources: resources.cloned(),
            contents: dict.get("Contents").cloned(),
        }),
    }
}

fn collect_images(
    file: &PdfObjects,
    resources: Option<&Object>,
    page: u32,
    seen: &mut HashSet<u32>,
    forms: &mut HashSet<u32>,
    listed: &mut Listed,
    depth: u32,
) {
    if depth > MAX_TREE_DEPTH || listed.images.len() >= MAX_IMAGES {
        return;
    }
    let Some(resources) = file.resolve_dict(resources) else {
        return;
    };
    let Some(xobjects) = file.resolve_dict(resources.get("XObject")) else {
        return;
    };
    for (_, entry) in &xobjects.0 {
        if listed.images.len() >= MAX_IMAGES {
            break;
        }
        let Object::Ref(num) = entry else {
            continue;
        };
        let Some(Object::Stream(dict, _)) = file.get(*num) else {
            continue;
        };
        let subtype = dict.get("Subtype");
        if subtype.is_some_and(|s| s.is_name("Image")) {
            if let (Some(width), Some(height)) =
                (size(file, dict, "Width"), size(file, dict, "Height"))
            {
                if seen.insert(*num) {
                    listed.images.push(PdfImage {
                        page,
                        object: Some(*num),
                        width,
                        height,
                    });
                    listed.sources.push(ImageSource::XObject(*num));
                }
            }
        } else if subtype.is_some_and(|s| s.is_name("Form")) && forms.insert(*num) {
            // A form without its own resources uses the page's.
            let fallback = Object::Dict(resources.clone());
            let inner = dict.get("Resources").unwrap_or(&fallback);
            collect_images(file, Some(inner), page, seen, forms, listed, depth + 1);
        }
    }
}

fn size(file: &PdfObjects, dict: &Dict, key: &str) -> Option<u32> {
    let value = file.resolve(dict.get(key)?).as_int()?;
    (value > 0 && value <= i64::from(u32::MAX)).then_some(value as u32)
}

fn content_bytes(file: &PdfObjects, contents: &Object) -> Option<Vec<u8>> {
    let items: Vec<&Object> = match file.resolve(contents) {
        Object::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    let mut out = Vec::new();
    for item in items {
        let Object::Stream(dict, data) = file.resolve(item) else {
            continue;
        };
        let Some(decoded) = decode_stream(file, dict, data) else {
            continue;
        };
        out.extend(decoded);
        out.push(b'\n');
    }
    (!out.is_empty()).then_some(out)
}

fn is_space(b: u8) -> bool {
    matches!(b, b'\0' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// The inline images in one decoded content stream, up to `limit`. A malformed
/// `BI` ends the scan: rescanning the rest for each malformed pair would be
/// quadratic on hostile input.
fn inline_images(data: &[u8], limit: usize) -> Vec<(Dict, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() && out.len() < limit {
        match data[at] {
            b'%' => {
                while at < data.len() && data[at] != b'\n' && data[at] != b'\r' {
                    at += 1;
                }
            }
            b'(' => at = skip_string(data, at),
            b'<' if data.get(at + 1) == Some(&b'<') => at = skip_dict(data, at),
            b'<' => {
                at += 1;
                while at < data.len() && data[at] != b'>' {
                    at += 1;
                }
                at = (at + 1).min(data.len());
            }
            b'/' => {
                at += 1;
                while at < data.len() && !is_space(data[at]) && !is_delimiter(data[at]) {
                    at += 1;
                }
            }
            b if is_space(b) || is_delimiter(b) => at += 1,
            _ => {
                let start = at;
                while at < data.len() && !is_space(data[at]) && !is_delimiter(data[at]) {
                    at += 1;
                }
                if &data[start..at] == b"BI" {
                    let Some((dict, body, end)) = super::objects::inline_image_at(data, at) else {
                        // Malformed inline data: the rest of the stream cannot
                        // be tokenized reliably, and rescanning it for each
                        // BI/ID pair would make a hostile stream quadratic.
                        // Treat it as the end of the inline images.
                        break;
                    };
                    out.push((normalize_inline(dict), body));
                    at = end;
                }
            }
        }
    }
    out
}

fn skip_string(data: &[u8], mut at: usize) -> usize {
    at += 1;
    let mut depth = 1;
    while at < data.len() && depth > 0 {
        match data[at] {
            b'\\' => at = (at + 2).min(data.len()),
            b'(' => {
                depth += 1;
                at += 1;
            }
            b')' => {
                depth -= 1;
                at += 1;
            }
            _ => at += 1,
        }
    }
    at
}

fn skip_dict(data: &[u8], mut at: usize) -> usize {
    at += 2;
    let mut depth = 1;
    while at < data.len() && depth > 0 {
        if data[at] == b'<' && data.get(at + 1) == Some(&b'<') {
            depth += 1;
            at += 2;
        } else if data[at] == b'>' && data.get(at + 1) == Some(&b'>') {
            depth -= 1;
            at += 2;
        } else if data[at] == b'(' {
            at = skip_string(data, at);
        } else {
            at += 1;
        }
    }
    at
}

fn normalize_inline(dict: Dict) -> Dict {
    let mut out = Dict::default();
    out.set("Type", Object::Name(b"XObject".to_vec()));
    out.set("Subtype", Object::Name(b"Image".to_vec()));
    for (key, value) in dict.0 {
        let key = match key.as_slice() {
            b"W" => b"Width".to_vec(),
            b"H" => b"Height".to_vec(),
            b"BPC" => b"BitsPerComponent".to_vec(),
            b"CS" => b"ColorSpace".to_vec(),
            b"D" => b"Decode".to_vec(),
            b"DP" => b"DecodeParms".to_vec(),
            b"F" => b"Filter".to_vec(),
            b"IM" => b"ImageMask".to_vec(),
            b"I" => b"Interpolate".to_vec(),
            b"Mask" => b"Mask".to_vec(),
            other => other.to_vec(),
        };
        let value = if key.as_slice() == &b"ColorSpace"[..] {
            expand_space(value)
        } else if key.as_slice() == &b"Filter"[..] {
            expand_filter(value)
        } else {
            value
        };
        out.0.push((key, value));
    }
    out
}

fn expand_space(value: Object) -> Object {
    match value {
        Object::Name(name) => Object::Name(space_name(&name).to_vec()),
        Object::Array(mut items) => {
            if let Some(Object::Name(name)) = items.first_mut() {
                let expanded = space_name(name).to_vec();
                *name = expanded;
            }
            Object::Array(items)
        }
        other => other,
    }
}

fn space_name(name: &[u8]) -> &[u8] {
    match name {
        b"G" => b"DeviceGray",
        b"RGB" => b"DeviceRGB",
        b"CMYK" => b"DeviceCMYK",
        b"I" => b"Indexed",
        other => other,
    }
}

fn expand_filter(value: Object) -> Object {
    match value {
        Object::Name(name) => Object::Name(filter_name(&name).to_vec()),
        Object::Array(items) => Object::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Object::Name(name) => Object::Name(filter_name(&name).to_vec()),
                    other => other,
                })
                .collect(),
        ),
        other => other,
    }
}

fn filter_name(name: &[u8]) -> &[u8] {
    match name {
        b"AHx" => b"ASCIIHexDecode",
        b"A85" => b"ASCII85Decode",
        b"Fl" => b"FlateDecode",
        b"DCT" => b"DCTDecode",
        b"CCF" => b"CCITTFaxDecode",
        b"RL" => b"RunLengthDecode",
        b"LZW" => b"LZWDecode",
        other => other,
    }
}

/// A one-page PDF whose MediaBox is the image's pixel size and whose content
/// draws it over the whole page, with every object it references copied along.
pub(crate) fn wrap_image(file: &PdfObjects, object: u32) -> Option<Vec<u8>> {
    wrap_object(file, file.get(object)?.clone())
}

pub(crate) fn wrap_stream(file: &PdfObjects, dict: Dict, data: Vec<u8>) -> Option<Vec<u8>> {
    wrap_object(file, Object::Stream(dict, data))
}

fn wrap_object(file: &PdfObjects, object: Object) -> Option<Vec<u8>> {
    let Object::Stream(dict, _) = &object else {
        return None;
    };
    let width = dict
        .get("Width")
        .map(|v| file.resolve(v))
        .and_then(Object::as_int)?;
    let height = dict
        .get("Height")
        .map(|v| file.resolve(v))
        .and_then(Object::as_int)?;
    if width <= 0 || height <= 0 {
        return None;
    }

    // 1 catalog, 2 pages, 3 page, 4 content; the image is object 5 and the
    // objects it references are copied from 6.
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut order = Vec::new();
    for target in references(&object) {
        if !renumber.contains_key(&target) && file.get(target).is_some() {
            renumber.insert(target, 6 + order.len() as u32);
            order.push(target);
        }
    }
    let mut at = 0;
    while at < order.len() {
        let source = file.get(order[at]).cloned().unwrap_or(Object::Null);
        for target in references(&source) {
            if !renumber.contains_key(&target) && file.get(target).is_some() {
                renumber.insert(target, 6 + order.len() as u32);
                order.push(target);
            }
        }
        at += 1;
    }

    let content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q").into_bytes();
    let mut objects: Vec<Object> = vec![
        Object::Dict(Dict(vec![
            (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
            (b"Pages".to_vec(), Object::Ref(2)),
        ])),
        Object::Dict(Dict(vec![
            (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
            (b"Kids".to_vec(), Object::Array(vec![Object::Ref(3)])),
            (b"Count".to_vec(), Object::Int(1)),
        ])),
        Object::Dict(Dict(vec![
            (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
            (b"Parent".to_vec(), Object::Ref(2)),
            (
                b"MediaBox".to_vec(),
                Object::Array(vec![
                    Object::Int(0),
                    Object::Int(0),
                    Object::Int(width),
                    Object::Int(height),
                ]),
            ),
            (
                b"Resources".to_vec(),
                Object::Dict(Dict(vec![(
                    b"XObject".to_vec(),
                    Object::Dict(Dict(vec![(b"Im0".to_vec(), Object::Ref(5))])),
                )])),
            ),
            (b"Contents".to_vec(), Object::Ref(4)),
        ])),
        Object::Stream(Dict::default(), content),
    ];
    objects.push(renumbered(object, &renumber));
    for num in &order {
        let copy = file.get(*num).cloned().unwrap_or(Object::Null);
        objects.push(renumbered(copy, &renumber));
    }

    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        write_object(&mut out, object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(&super::super::object::xref_section(1, &offsets));
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    Some(out)
}

/// Every object number `object` refers to.
fn references(object: &Object) -> Vec<u32> {
    let mut out = Vec::new();
    let mut stack = vec![object];
    while let Some(object) = stack.pop() {
        match object {
            Object::Ref(num) => out.push(*num),
            Object::Array(items) => stack.extend(items),
            Object::Dict(dict) | Object::Stream(dict, _) => {
                stack.extend(dict.0.iter().map(|(_, v)| v))
            }
            _ => {}
        }
    }
    out
}

fn renumbered(object: Object, map: &HashMap<u32, u32>) -> Object {
    match object {
        Object::Ref(num) => map.get(&num).map_or(Object::Null, |n| Object::Ref(*n)),
        Object::Array(items) => {
            Object::Array(items.into_iter().map(|i| renumbered(i, map)).collect())
        }
        Object::Dict(dict) => Object::Dict(renumbered_dict(dict, map)),
        Object::Stream(dict, data) => Object::Stream(renumbered_dict(dict, map), data),
        other => other,
    }
}

fn renumbered_dict(dict: Dict, map: &HashMap<u32, u32>) -> Dict {
    Dict(
        dict.0
            .into_iter()
            .map(|(k, v)| (k, renumbered(v, map)))
            .collect(),
    )
}
