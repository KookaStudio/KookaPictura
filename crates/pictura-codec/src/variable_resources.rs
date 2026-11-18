use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pictura_core::layer_comp::layer_id;
use pictura_core::variables::{
    valid_name, Binding, DataSet, Placement, ScaleMethod, Value, Variable, VariableKind,
    VariableStore,
};
use pictura_core::{Document, LayerStack};
use quick_xml::{events::Event, Reader};

use crate::image_resources::{decode_section, frame_image_resource, ImageResource};

const DEFINITIONS: u16 = 7000;
const SETS: u16 = 7001;
const ADOBE: &str = "http://ns.adobe.com/Variables/1.0/";
const KOOKA: &str = "urn:kooka-pictura:variable-data:1";
const MAX_XML_BYTES: usize = 16 * 1024 * 1024;
const MAX_XML_ELEMENTS: usize = 250_000;

#[derive(Default)]
struct Node {
    name: String,
    attributes: BTreeMap<String, String>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(String::as_str)
    }

    fn has_only(&self, keys: &[&str]) -> bool {
        self.attributes
            .keys()
            .all(|key| keys.contains(&key.as_str()))
    }

    fn child(&self, name: &str) -> Option<&Self> {
        let mut found = self.children.iter().filter(|n| n.name == name);
        let only = found.next()?;
        found.next().is_none().then_some(only)
    }

    fn authored(&self) -> bool {
        self.get("xmlns:kp") == Some(KOOKA) && self.get("kp:format") == Some("1")
    }
}

fn parse_xml(bytes: &[u8]) -> Option<Node> {
    if bytes.len() > MAX_XML_BYTES {
        return None;
    }
    let mut reader = Reader::from_str(std::str::from_utf8(bytes).ok()?);
    reader.config_mut().expand_empty_elements = true;
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        match reader.read_event().ok()? {
            Event::Start(start) => {
                count += 1;
                if stack.len() >= 8 || count > MAX_XML_ELEMENTS {
                    return None;
                }
                let mut node = Node {
                    name: start.name().as_ref().into(),
                    ..Node::default()
                };
                for attribute in start.attributes() {
                    let attribute = attribute.ok()?;
                    let key = attribute.key.as_ref();
                    let value = quick_xml::escape::unescape(&attribute.value).ok()?;
                    if node
                        .attributes
                        .insert(key.into(), value.into_owned())
                        .is_some()
                    {
                        return None;
                    }
                }
                stack.push(node);
            }
            Event::End(_) => {
                let node = stack.pop()?;
                stack.last_mut()?.children.push(node);
            }
            Event::Text(text) => stack.last_mut()?.text.push_str(&text),
            Event::CData(text) => stack.last_mut()?.text.push_str(&text),
            Event::GeneralRef(reference) => {
                let reference = reference.as_ref();
                let escaped = format!("&{reference};");
                stack
                    .last_mut()?
                    .text
                    .push_str(&quick_xml::escape::unescape(&escaped).ok()?);
            }
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Eof => break,
            _ => return None,
        }
    }
    let mut root = stack.pop()?;
    (stack.is_empty() && root.text.trim().is_empty() && root.children.len() == 1)
        .then(|| root.children.remove(0))
}

fn original_layers(stack: &LayerStack) -> BTreeMap<u32, usize> {
    fn add(stack: &LayerStack, ids: &mut BTreeMap<u32, usize>) {
        for layer in stack.iter() {
            if let Some(id) = layer_id(layer) {
                *ids.entry(id).or_default() += 1;
            }
            add(layer.children(), ids);
        }
    }
    let mut layers = BTreeMap::new();
    add(stack, &mut layers);
    layers
}

fn placement(node: &Node, authored: bool) -> Option<Placement> {
    let method = node.get("placementMethod")?;
    let align = node.get("align")?;
    let valign = node.get("valign")?;
    let clip = node.get("clip")?;
    let result = if authored {
        let method = match method {
            "fit" => ScaleMethod::Fit,
            "kp-fill" => ScaleMethod::Fill,
            "ratio" => ScaleMethod::AsIs,
            "kp-conform" => ScaleMethod::Conform,
            _ => return None,
        };
        Placement {
            method,
            horizontal: ["left", "center", "right"]
                .iter()
                .position(|v| *v == align)? as u8,
            vertical: ["top", "middle", "bottom"]
                .iter()
                .position(|v| *v == valign)? as u8,
            clip: match clip {
                "true" => true,
                "false" => false,
                _ => return None,
            },
        }
    } else {
        if (align, valign, clip) != ("center", "middle", "false") {
            return None;
        }
        let method = match method {
            "fit" => ScaleMethod::Fit,
            "ratio" => ScaleMethod::AsIs,
            _ => return None,
        };
        Placement {
            method,
            ..Placement::default()
        }
    };
    result.valid().then_some(result)
}

fn decode_definitions(
    root: &Node,
    layers: &BTreeMap<u32, usize>,
    issues: &mut Vec<String>,
    protected: &mut BTreeMap<u32, usize>,
) -> Option<BTreeMap<String, Variable>> {
    if root.name != "variableSets"
        || root.get("xmlns") != Some(ADOBE)
        || !root.has_only(&["xmlns", "xmlns:kp", "kp:format"])
        || root.children.len() != 1
        || (root.attributes.len() > 1 && !root.authored())
    {
        return None;
    }
    let set = root.child("variableSet")?;
    if !set.has_only(&["locked", "varSetName"])
        || set.children.len() != 1
        || !root.text.trim().is_empty()
        || !set.text.trim().is_empty()
        || set.get("locked").is_some_and(|value| value != "none")
    {
        return None;
    }
    let variables = set.child("variables")?;
    if !variables.attributes.is_empty() || !variables.text.trim().is_empty() {
        return None;
    }
    let mut decoded: BTreeMap<String, Variable> = BTreeMap::new();
    let mut claimed = BTreeSet::new();
    for (index, node) in variables.children.iter().enumerate() {
        let name = node.get("varName").unwrap_or("(unnamed)");
        let target = node
            .get("docRef")
            .and_then(|s| s.strip_prefix("id('"))
            .and_then(|s| s.strip_suffix("')"))
            .and_then(|s| s.parse::<u32>().ok());
        if let Some(id) = target {
            if let Some(count) = layers.get(&id).filter(|&&count| count > 1) {
                protected.insert(id, *count);
                issues.push(format!("Variable {name} refers to ambiguous layer ID {id}"));
                continue;
            }
        }
        let entry = (|| {
            if node.name != "variable"
                || !valid_name(name)
                || !node.children.is_empty()
                || !node.text.trim().is_empty()
            {
                return None;
            }
            let kind = match node.get("trait")? {
                "visibility" => VariableKind::Visibility,
                "textcontent" => VariableKind::Text,
                "fileref" => VariableKind::Pixels,
                _ => return None,
            };
            let place = if kind == VariableKind::Pixels {
                if !node.has_only(&[
                    "varName",
                    "trait",
                    "docRef",
                    "placementMethod",
                    "align",
                    "valign",
                    "clip",
                ]) {
                    return None;
                }
                placement(node, root.authored())?
            } else {
                if !node.has_only(&["varName", "trait", "docRef"]) {
                    return None;
                }
                Placement::default()
            };
            let id = target?;
            layers.get(&id)?;
            Some((
                kind,
                Binding {
                    layer_id: id,
                    placement: place,
                },
            ))
        })();
        let Some((kind, binding)) = entry else {
            issues.push(format!(
                "Variable {name} has an unsupported definition or missing layer"
            ));
            continue;
        };
        let variable = decoded.entry(name.into()).or_insert_with(|| Variable {
            id: index as u64 + 1,
            kind,
            bindings: Vec::new(),
        });
        if variable.kind != kind || !claimed.insert((binding.layer_id, kind as u8)) {
            issues.push(format!("Variable {name} has a conflicting layer binding"));
        } else {
            variable.bindings.push(binding);
        }
    }
    Some(decoded)
}

fn decode_sets(
    root: &Node,
    definitions: &BTreeMap<String, Variable>,
    issues: &mut Vec<String>,
) -> Option<Vec<DataSet>> {
    if root.name != "sampleDataSets"
        || !root.has_only(&["psdPath", "xmlns:kp", "kp:format"])
        || ((root.get("xmlns:kp").is_some() || root.get("kp:format").is_some()) && !root.authored())
    {
        return None;
    }
    let mut sets = Vec::new();
    let mut names = BTreeSet::new();
    let authored = root.authored();
    for row in &root.children {
        let name = row.get("dataSetName").unwrap_or_default();
        if row.name != "sampleDataSet"
            || name.trim().is_empty()
            || !row.has_only(&["dataSetName"])
            || !row.text.trim().is_empty()
            || !names.insert(name)
        {
            issues.push(format!("Unsupported or duplicate data set {name}"));
            continue;
        }
        let mut values = BTreeMap::new();
        for field in &row.children {
            let Some(variable) = definitions.get(&field.name) else {
                issues.push(format!(
                    "Data set {name} has an unresolved variable {}",
                    field.name
                ));
                continue;
            };
            let value = if !field.children.is_empty()
                || !field.has_only(&["kp:keep"])
                || (!authored && !field.attributes.is_empty())
            {
                None
            } else {
                match variable.kind {
                    VariableKind::Visibility if authored => match field.text.as_str() {
                        "true" => Some(Value::Visibility(true)),
                        "false" => Some(Value::Visibility(false)),
                        _ => None,
                    },
                    VariableKind::Visibility => None,
                    VariableKind::Text if field.attributes.is_empty() => {
                        Some(Value::Text(field.text.clone()))
                    }
                    VariableKind::Pixels
                        if field.get("kp:keep") == Some("1") && field.text.is_empty() =>
                    {
                        Some(Value::KeepPixels)
                    }
                    VariableKind::Pixels
                        if field.attributes.is_empty() && !field.text.trim().is_empty() =>
                    {
                        Some(Value::File(field.text.clone()))
                    }
                    _ => None,
                }
            };
            match value {
                Some(value) if !values.contains_key(&variable.id) => {
                    values.insert(variable.id, value);
                }
                _ => issues.push(format!(
                    "Data set {name} has an unsupported or duplicate value for {}",
                    field.name
                )),
            }
        }
        if values.len() != definitions.len() {
            issues.push(format!("Data set {name} has missing values"));
        }
        sets.push(DataSet {
            name: name.into(),
            values,
        });
    }
    Some(sets)
}

pub(crate) fn resolve(doc: &mut Document) {
    if doc.image_resources.is_empty() {
        return;
    }
    let (resources, consumed) = decode_section(&doc.image_resources);
    let tail = &doc.image_resources[consumed..];
    let truncated = tail.len() >= 6
        && &tail[..4] == b"MeSa"
        && matches!(u16::from_be_bytes([tail[4], tail[5]]), DEFINITIONS | SETS);
    let resources: Vec<_> = resources
        .into_iter()
        .filter(|r| matches!(r.id, DEFINITIONS | SETS))
        .collect();
    if resources.is_empty() && !truncated {
        return;
    }
    let layers = original_layers(&doc.layers);
    let mut issues = Vec::new();
    if truncated {
        issues.push("Malformed truncated PSD variable resource".into());
    }
    let mut protected = BTreeMap::new();
    let mut read = |id| {
        let matches: Vec<_> = resources.iter().filter(|r| r.id == id).collect();
        match matches.as_slice() {
            [] => None,
            [resource] if resource.raw.starts_with(b"MeSa") => match parse_xml(&resource.data) {
                Some(root) => Some(root),
                None => {
                    issues.push(format!("Malformed PSD resource {id}"));
                    None
                }
            },
            _ => {
                issues.push(format!("Unsupported or duplicate PSD resource {id}"));
                None
            }
        }
    };
    let definitions = read(DEFINITIONS);
    let rows = read(SETS);
    let definitions = definitions
        .map(|root| {
            decode_definitions(&root, &layers, &mut issues, &mut protected).unwrap_or_else(|| {
                issues.push("Unsupported PSD variable definitions".into());
                BTreeMap::new()
            })
        })
        .unwrap_or_default();
    let sets = rows
        .map(|root| {
            decode_sets(&root, &definitions, &mut issues).unwrap_or_else(|| {
                issues.push("Unsupported PSD data sets".into());
                Vec::new()
            })
        })
        .unwrap_or_default();
    doc.variables = VariableStore::from_file(definitions, sets, issues, protected);
}

fn frame_variable_resource(id: u16, data: &[u8]) -> ImageResource {
    let mut resource = frame_image_resource(id, "", data);
    resource.raw[..4].copy_from_slice(b"MeSa");
    resource
}

fn escape(value: &str) -> String {
    quick_xml::escape::escape(value)
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

fn authored_definitions(store: &VariableStore) -> String {
    let mut out = format!(
        r#"<variableSets xmlns="{ADOBE}" xmlns:kp="{KOOKA}" kp:format="1"><variableSet locked="none" varSetName="binding1"><variables>"#
    );
    for (name, variable) in store.definitions() {
        for binding in &variable.bindings {
            let trait_name = match variable.kind {
                VariableKind::Visibility => "visibility",
                VariableKind::Text => "textcontent",
                VariableKind::Pixels => "fileref",
            };
            let _ = write!(
                out,
                r#"<variable varName="{name}" trait="{trait_name}" docRef="id('{}')""#,
                binding.layer_id
            );
            if variable.kind == VariableKind::Pixels {
                let p = binding.placement;
                let method = match p.method {
                    ScaleMethod::Fit => "fit",
                    ScaleMethod::AsIs => "ratio",
                    ScaleMethod::Fill => "kp-fill",
                    ScaleMethod::Conform => "kp-conform",
                };
                let align = ["left", "center", "right"][usize::from(p.horizontal.min(2))];
                let valign = ["top", "middle", "bottom"][usize::from(p.vertical.min(2))];
                let _ = write!(
                    out,
                    r#" placementMethod="{method}" align="{align}" valign="{valign}" clip="{}""#,
                    p.clip
                );
            }
            out.push_str("/>");
        }
    }
    out.push_str("</variables></variableSet></variableSets>");
    out
}

fn authored_rows(store: &VariableStore) -> String {
    let mut out = format!(r#"<sampleDataSets psdPath="" xmlns:kp="{KOOKA}" kp:format="1">"#);
    for row in store.sets() {
        let _ = write!(
            out,
            r#"<sampleDataSet dataSetName="{}">"#,
            escape(&row.name)
        );
        for (name, variable) in store.definitions() {
            let Some(value) = row.values.get(&variable.id) else {
                continue;
            };
            match value {
                Value::KeepPixels => {
                    let _ = write!(out, r#"<{name} kp:keep="1"/>"#);
                }
                Value::Text(text) | Value::File(text) => {
                    let _ = write!(out, "<{name}>{}</{name}>", escape(text));
                }
                Value::Visibility(value) => {
                    let _ = write!(out, "<{name}>{value}</{name}>");
                }
            }
        }
        out.push_str("</sampleDataSet>");
    }
    out.push_str("</sampleDataSets>");
    out
}

pub(crate) fn output(doc: &Document, section: Vec<u8>) -> Result<Vec<u8>, crate::PsdError> {
    let store = &doc.variables;
    if !store.preserves_identities(&doc.layers)
        || (!store.protected_ids().is_empty() && doc.layer_comps.dirty)
    {
        return Err(crate::PsdError::Invalid(
            "This edit would overwrite ambiguous variable layer IDs or dependent layer comps"
                .into(),
        ));
    }
    if !store.dirty() {
        return Ok(section);
    }
    let (resources, consumed) = decode_section(&section);
    let mut output = Vec::new();
    for resource in resources
        .into_iter()
        .filter(|r| !matches!(r.id, DEFINITIONS | SETS))
    {
        output.extend_from_slice(&resource.raw);
    }
    if !store.definitions().is_empty() {
        let definitions = authored_definitions(store);
        let rows = (!store.sets().is_empty()).then(|| authored_rows(store));
        if definitions.len() > MAX_XML_BYTES
            || rows.as_ref().is_some_and(|rows| rows.len() > MAX_XML_BYTES)
        {
            return Err(crate::PsdError::Invalid(
                "Variable resources exceed the PSD XML budget".into(),
            ));
        }
        output.extend(frame_variable_resource(DEFINITIONS, definitions.as_bytes()).raw);
        if let Some(rows) = rows {
            output.extend(frame_variable_resource(SETS, rows.as_bytes()).raw);
        }
    }
    output.extend_from_slice(&section[consumed..]);
    Ok(output)
}

#[cfg(test)]
mod tests;
