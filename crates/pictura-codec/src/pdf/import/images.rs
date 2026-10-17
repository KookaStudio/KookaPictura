//! Images mode (design D4): the image XObjects a PDF's pages draw, and a
//! one-page PDF that draws one of them full-page so Ghostscript decodes it.

use std::collections::{HashMap, HashSet};

use super::objects::{write_object, Dict, Object, PdfObjects};

/// One image XObject: the page it was first found on (1-based), its object
/// number, and its pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfImage {
    pub page: u32,
    pub object: u32,
    pub width: u32,
    pub height: u32,
}

/// The distinct image XObjects of every page, in page order, including those
/// inside Form XObjects. Inline images and images in patterns are not listed.
/// `None` when the file cannot be read or is encrypted.
pub(crate) fn list_images(file: &PdfObjects) -> Option<Vec<PdfImage>> {
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
    );
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (index, resources) in pages.iter().enumerate() {
        let mut forms = HashSet::new();
        collect_images(
            file,
            resources.as_ref(),
            index as u32 + 1,
            &mut seen,
            &mut forms,
            &mut out,
        );
    }
    Some(out)
}

/// Each leaf page's effective `/Resources` (inherited from its ancestors).
fn collect_pages(
    file: &PdfObjects,
    node: &Object,
    inherited: Option<&Object>,
    pages: &mut Vec<Option<Object>>,
    visited: &mut HashSet<u32>,
) {
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
                collect_pages(file, kid, resources, pages, visited);
            }
        }
        _ => pages.push(resources.cloned()),
    }
}

fn collect_images(
    file: &PdfObjects,
    resources: Option<&Object>,
    page: u32,
    seen: &mut HashSet<u32>,
    forms: &mut HashSet<u32>,
    out: &mut Vec<PdfImage>,
) {
    let Some(resources) = file.resolve_dict(resources) else {
        return;
    };
    let Some(xobjects) = file.resolve_dict(resources.get("XObject")) else {
        return;
    };
    for (_, entry) in &xobjects.0 {
        let Object::Ref(num) = entry else {
            continue;
        };
        let Some(Object::Stream(dict, _)) = file.get(*num) else {
            continue;
        };
        let subtype = dict.get("Subtype");
        if subtype.is_some_and(|s| s.is_name("Image")) {
            let size = |key| {
                dict.get(key)
                    .map(|v| file.resolve(v))
                    .and_then(Object::as_int)
                    .filter(|v| *v > 0)
            };
            if let (Some(width), Some(height)) = (size("Width"), size("Height")) {
                if seen.insert(*num) {
                    out.push(PdfImage {
                        page,
                        object: *num,
                        width: width as u32,
                        height: height as u32,
                    });
                }
            }
        } else if subtype.is_some_and(|s| s.is_name("Form")) && forms.insert(*num) {
            // A form without its own resources uses the page's.
            let fallback = Object::Dict(resources.clone());
            let inner = dict.get("Resources").unwrap_or(&fallback);
            collect_images(file, Some(inner), page, seen, forms, out);
        }
    }
}

/// A one-page PDF whose MediaBox is the image's pixel size and whose content
/// draws image object `object` over the whole page, with every object it
/// references copied along. `None` when `object` is not an image.
pub(crate) fn wrap_image(file: &PdfObjects, object: u32) -> Option<Vec<u8>> {
    let Some(Object::Stream(dict, _)) = file.get(object) else {
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

    // 1 catalog, 2 pages, 3 page, 4 content; copies are renumbered from 5.
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut order = vec![object];
    renumber.insert(object, 5);
    let mut at = 0;
    while at < order.len() {
        let source = file.get(order[at]).cloned().unwrap_or(Object::Null);
        for target in references(&source) {
            if !renumber.contains_key(&target) && file.get(target).is_some() {
                renumber.insert(target, 5 + order.len() as u32);
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
