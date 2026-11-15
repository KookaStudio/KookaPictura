//! XMP span patching and insertion.
use super::scan::{scan_packet, str_at};
use super::{canonical, escape_xml, parse_xmp_bool, prefix_for, Form, SpanInfo, XmpField, RDF_URI};
use std::collections::HashMap;
fn normalize_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter(|value| !value.is_empty())
        .cloned()
        .collect()
}

pub fn patch_xmp_values(packet: &str, updates: &[(XmpField, Vec<String>)]) -> Option<String> {
    if updates.is_empty() {
        return Some(packet.to_string());
    }
    let bytes = packet.as_bytes();
    let scan = scan_packet(bytes);
    let description = scan.description.as_ref()?;
    if scan.malformed || scan.over_limit {
        return None;
    }

    let mut requested: Vec<(XmpField, Vec<String>)> = Vec::new();
    for (field, values) in updates {
        let values = normalize_values(values);
        match requested.iter_mut().find(|(f, _)| f == field) {
            Some(slot) => slot.1 = values,
            None => requested.push((*field, values)),
        }
    }

    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut insertions = String::new();
    for (field, values) in &requested {
        if scan.spans.iter().filter(|s| s.field == *field).count() > 1 {
            return None;
        }
        match scan.spans.iter().find(|s| s.field == *field) {
            Some(span) => {
                if span.unsafe_ || !span.complete {
                    return None;
                }
                if values.is_empty() {
                    // Clearing removes the property bytes entirely; an absent
                    // property already satisfies an empty request.
                    edits.push((span.start, span.end, String::new()));
                    continue;
                }
                if *field == XmpField::Marked && parse_xmp_bool(&values[0]).is_none() {
                    return None;
                }
                if already_matches(span, *field, values) {
                    continue;
                }
                edits.push((
                    span.start,
                    span.end,
                    replacement(span, *field, values, &description.scope)?,
                ));
            }
            None => {
                if values.is_empty() {
                    continue;
                }
                if *field == XmpField::Marked && parse_xmp_bool(&values[0]).is_none() {
                    return None;
                }
                insertions.push_str(&insertion(*field, values, &description.scope)?);
            }
        }
    }
    if edits.is_empty() && insertions.is_empty() {
        return Some(packet.to_string());
    }

    if !insertions.is_empty() {
        // ponytail: a self-closing rdf:Description that needs both an attribute
        // edit and an insertion fails closed, because the insertion rewrites the
        // whole start tag and would overlap the attribute's span.
        if description.self_closing {
            let raw = &bytes[description.start_tag.0..description.start_tag.1];
            let head = raw.strip_suffix(b"/>")?;
            let head = std::str::from_utf8(head).ok()?;
            let qname = str_at(bytes, description.qname);
            edits.push((
                description.start_tag.0,
                description.start_tag.1,
                format!("{head}>{insertions}</{qname}>"),
            ));
        } else {
            let at = description.content_end;
            edits.push((at, at, insertions));
        }
    }

    edits.sort_by_key(|e| (e.0, e.1));
    for pair in edits.windows(2) {
        if pair[1].0 < pair[0].1 {
            return None;
        }
    }
    let mut out = String::with_capacity(packet.len() + 64);
    let mut cursor = 0;
    for (start, end, fragment) in &edits {
        if *start < cursor {
            return None;
        }
        out.push_str(&packet[cursor..*start]);
        out.push_str(fragment);
        cursor = *end;
    }
    out.push_str(&packet[cursor..]);
    Some(out)
}

/// Set single managed properties. See [`patch_xmp_values`] for list fields.
pub fn patch_xmp(packet: &str, updates: &[(XmpField, String)]) -> Option<String> {
    let updates: Vec<(XmpField, Vec<String>)> = updates
        .iter()
        .map(|(field, value)| (*field, vec![value.clone()]))
        .collect();
    patch_xmp_values(packet, &updates)
}

/// Drop empty elements so a single empty value (or an all-empty list) clears the
/// property, matching the parser's absent-when-empty view.
/// Whether the property already holds the requested value(s). Scalar fields use
/// the property's scalar value — the `x-default` item for an `rdf:Alt`, the text
/// for a simple property — so it agrees with the reader and the IPTC-Core sync
/// gate. List fields (`Creator`/`Subject`) match the full item list, so a
/// shorter list can replace a longer one.
fn already_matches(span: &SpanInfo, field: XmpField, values: &[String]) -> bool {
    if is_list_field(field) {
        let current: Vec<&str> = span.values.iter().map(|v| v.value.as_str()).collect();
        let requested: Vec<&str> = values.iter().map(String::as_str).collect();
        return current == requested;
    }
    let value = values.first().map(String::as_str).unwrap_or("");
    if field == XmpField::Marked {
        return span.values.first().and_then(|v| parse_xmp_bool(&v.value)) == parse_xmp_bool(value);
    }
    span.scalar_value().as_deref() == Some(value)
}

/// The `rdf:` prefix in scope for a rewritten property: the existing container's
/// prefix, or one declared on the `rdf:Description`. `None` fails the rewrite
/// closed rather than dropping items or emitting an unbound prefix.
fn rdf_prefix(span: &SpanInfo, scope: &HashMap<String, String>) -> Option<String> {
    match span.container.as_ref() {
        Some((_, prefix)) if !prefix.is_empty() => Some(prefix.clone()),
        Some(_) => None,
        None => prefix_for(scope, RDF_URI),
    }
}

fn list_element(
    name: &str,
    kind: &str,
    span: &SpanInfo,
    values: &[String],
    scope: &HashMap<String, String>,
) -> Option<String> {
    let rdf = rdf_prefix(span, scope)?;
    Some(format!(
        "<{name}><{rdf}:{kind}>{}</{rdf}:{kind}></{name}>",
        seq_items(&rdf, values)
    ))
}

fn replacement(
    span: &SpanInfo,
    field: XmpField,
    values: &[String],
    scope: &HashMap<String, String>,
) -> Option<String> {
    if span.form == Form::Attribute {
        return Some(format!("{}=\"{}\"", span.name, escape_xml(values.first()?)));
    }
    let name = span.name.as_str();
    match field {
        XmpField::Creator => list_element(name, "Seq", span, values, scope),
        XmpField::Subject => list_element(name, "Bag", span, values, scope),
        XmpField::Title | XmpField::Description | XmpField::Rights | XmpField::UsageTerms
            if span.container.is_some() =>
        {
            let rdf = rdf_prefix(span, scope)?;
            let value = values.first()?;
            Some(format!(
                "<{name}><{rdf}:Alt><{rdf}:li xml:lang=\"x-default\">{}</{rdf}:li></{rdf}:Alt></{name}>",
                escape_xml(value)
            ))
        }
        XmpField::Marked => {
            let text = if parse_xmp_bool(values.first()?)? {
                "True"
            } else {
                "False"
            };
            Some(format!("<{name}>{text}</{name}>"))
        }
        _ => Some(format!("<{name}>{}</{name}>", escape_xml(values.first()?))),
    }
}

fn is_list_field(field: XmpField) -> bool {
    matches!(field, XmpField::Creator | XmpField::Subject)
}

fn seq_items(rdf: &str, values: &[String]) -> String {
    let mut out = String::new();
    for value in values {
        out.push_str(&format!("<{rdf}:li>{}</{rdf}:li>", escape_xml(value)));
    }
    out
}

fn insertion(
    field: XmpField,
    values: &[String],
    scope: &HashMap<String, String>,
) -> Option<String> {
    let (uri, local) = canonical(field);
    // ponytail: an inserted property reuses a prefix the receiving
    // rdf:Description already has in scope; an unbound prefix fails closed
    // rather than editing the namespace declarations.
    let prefix = prefix_for(scope, uri)?;
    match field {
        XmpField::Title | XmpField::Description | XmpField::Rights | XmpField::UsageTerms => {
            let rdf = prefix_for(scope, RDF_URI)?;
            let value = values.first()?;
            Some(format!(
                "<{prefix}:{local}><{rdf}:Alt><{rdf}:li xml:lang=\"x-default\">{}</{rdf}:li></{rdf}:Alt></{prefix}:{local}>",
                escape_xml(value)
            ))
        }
        XmpField::Creator => {
            let rdf = prefix_for(scope, RDF_URI)?;
            Some(format!(
                "<{prefix}:{local}><{rdf}:Seq>{}</{rdf}:Seq></{prefix}:{local}>",
                seq_items(&rdf, values)
            ))
        }
        XmpField::Subject => {
            let rdf = prefix_for(scope, RDF_URI)?;
            Some(format!(
                "<{prefix}:{local}><{rdf}:Bag>{}</{rdf}:Bag></{prefix}:{local}>",
                seq_items(&rdf, values)
            ))
        }
        XmpField::Marked => {
            let text = if parse_xmp_bool(values.first()?)? {
                "True"
            } else {
                "False"
            };
            Some(format!("<{prefix}:{local}>{text}</{prefix}:{local}>"))
        }
        _ => Some(format!(
            "<{prefix}:{local}>{}</{prefix}:{local}>",
            escape_xml(values.first()?)
        )),
    }
}
