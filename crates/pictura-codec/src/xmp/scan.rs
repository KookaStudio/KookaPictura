//! XMP tokenizer and managed-field scanner.
use super::{
    managed_field, ContainerKind, Description, Form, LiVal, Scan, SpanInfo, XmpField, MAX_ATTRS,
    MAX_DEPTH, MAX_ENTITY_RUN, MAX_PACKET_BYTES, MAX_TOKENS, MAX_VALUE_BYTES, RDF_URI,
};
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub(super) struct RawAttr {
    pub(super) name: (usize, usize),
    pub(super) value: (usize, usize),
}

#[derive(Debug)]
pub(super) enum RawKind {
    Start {
        qname: (usize, usize),
        self_closing: bool,
        attrs: Vec<RawAttr>,
    },
    End {
        qname: (usize, usize),
    },
    Text,
    Ignored,
}

#[derive(Debug)]
pub(super) struct RawToken {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) kind: RawKind,
}

#[derive(Debug)]
struct Frame {
    qname: (usize, usize),
    is_description: bool,
    field: Option<XmpField>,
    start: usize,
    values: Vec<LiVal>,
    container: Option<(ContainerKind, String)>,
    unsafe_: bool,
    text_buf: Option<Vec<u8>>,
    is_li: bool,
    li_lang: Option<String>,
    li_owner: Option<usize>,
    /// Declarations this element added, restored when its scope closes.
    ns_undo: Vec<(String, Option<String>)>,
}
pub(super) fn str_at(bytes: &[u8], span: (usize, usize)) -> &str {
    std::str::from_utf8(&bytes[span.0..span.1]).unwrap_or("")
}

pub(super) fn split_qname(name: &str) -> (&str, &str) {
    match name.split_once(':') {
        Some((prefix, local)) => (prefix, local),
        None => ("", name),
    }
}

fn xmlns_prefix(name: &str) -> Option<&str> {
    if name == "xmlns" {
        Some("")
    } else {
        name.strip_prefix("xmlns:")
    }
}

pub(super) fn push_declarations(
    current: &mut HashMap<String, String>,
    bytes: &[u8],
    attrs: &[RawAttr],
) -> Vec<(String, Option<String>)> {
    let mut declared = Vec::new();
    for attr in attrs {
        let name = str_at(bytes, attr.name);
        if let Some(prefix) = xmlns_prefix(name) {
            let uri = decode_text(&bytes[attr.value.0..attr.value.1]).text;
            let old = current.insert(prefix.to_string(), uri);
            declared.push((prefix.to_string(), old));
        }
    }
    declared
}

pub(super) fn restore_declarations(
    current: &mut HashMap<String, String>,
    declared: Vec<(String, Option<String>)>,
) {
    for (prefix, old) in declared.into_iter().rev() {
        match old {
            Some(uri) => {
                current.insert(prefix, uri);
            }
            None => {
                current.remove(&prefix);
            }
        }
    }
}

fn nearest_field(stack: &[Frame]) -> Option<usize> {
    stack.iter().rposition(|frame| frame.field.is_some())
}

pub(super) struct Decoded {
    pub(super) text: String,
    pub(super) unsafe_: bool,
}

pub(super) fn decode_text(raw: &[u8]) -> Decoded {
    let source = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(source.len());
    let mut unsafe_ = false;
    let mut rest = source.as_ref();
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        let after = &rest[index + 1..];
        let mut semi = None;
        for (offset, ch) in after.char_indices() {
            if offset > MAX_ENTITY_RUN {
                break;
            }
            if ch == ';' {
                semi = Some(offset);
                break;
            }
        }
        match semi {
            Some(offset) => {
                match &after[..offset] {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    other => {
                        unsafe_ = true;
                        out.push('&');
                        out.push_str(other);
                        out.push(';');
                    }
                }
                rest = &after[offset + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Decoded { text: out, unsafe_ }
}

fn read_name_end(bytes: &[u8], start: usize, limit: usize) -> Option<usize> {
    if start >= limit {
        return None;
    }
    let mut i = start;
    while i < limit {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>' | b'=' => break,
            _ => i += 1,
        }
    }
    Some(i)
}

fn skip_ws(bytes: &[u8], mut i: usize, limit: usize) -> usize {
    while i < limit && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }
    i
}

fn find_from(bytes: &[u8], start: usize, needle: &[u8], limit: usize) -> Option<usize> {
    if start > limit || needle.is_empty() {
        return None;
    }
    let mut i = start;
    while i + needle.len() <= limit {
        if &bytes[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub(super) fn tokenize(bytes: &[u8]) -> (Vec<RawToken>, bool) {
    let mut tokens: Vec<RawToken> = Vec::new();
    let limit = bytes.len().min(MAX_PACKET_BYTES);
    let mut pos = 0;
    while pos < limit {
        if tokens.len() >= MAX_TOKENS {
            return (tokens, false);
        }
        if bytes[pos] != b'<' {
            let start = pos;
            while pos < limit && bytes[pos] != b'<' {
                pos += 1;
            }
            tokens.push(RawToken {
                start,
                end: pos,
                kind: RawKind::Text,
            });
            continue;
        }
        if bytes[pos..].starts_with(b"<!--") {
            let Some(close) = find_from(bytes, pos + 4, b"-->", limit) else {
                return (tokens, false);
            };
            tokens.push(RawToken {
                start: pos,
                end: close + 3,
                kind: RawKind::Ignored,
            });
            pos = close + 3;
            continue;
        }
        if bytes[pos..].starts_with(b"<![CDATA[") {
            let Some(close) = find_from(bytes, pos + 9, b"]]>", limit) else {
                return (tokens, false);
            };
            tokens.push(RawToken {
                start: pos + 9,
                end: close,
                kind: RawKind::Text,
            });
            pos = close + 3;
            continue;
        }
        if bytes[pos..].starts_with(b"</") {
            let name_start = pos + 2;
            let Some(name_end) = read_name_end(bytes, name_start, limit) else {
                return (tokens, false);
            };
            let Some(gt) = find_from(bytes, name_end, b">", limit) else {
                return (tokens, false);
            };
            tokens.push(RawToken {
                start: pos,
                end: gt + 1,
                kind: RawKind::End {
                    qname: (name_start, name_end),
                },
            });
            pos = gt + 1;
            continue;
        }
        if bytes[pos..].starts_with(b"<?") {
            let Some(close) = find_from(bytes, pos + 2, b"?>", limit) else {
                return (tokens, false);
            };
            tokens.push(RawToken {
                start: pos,
                end: close + 2,
                kind: RawKind::Ignored,
            });
            pos = close + 2;
            continue;
        }
        if bytes[pos..].starts_with(b"<!") {
            let mut i = pos + 2;
            let mut depth = 0i32;
            let mut end = None;
            while i < limit {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    b'>' if depth <= 0 => {
                        end = Some(i);
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }
            let Some(close) = end else {
                return (tokens, false);
            };
            tokens.push(RawToken {
                start: pos,
                end: close + 1,
                kind: RawKind::Ignored,
            });
            pos = close + 1;
            continue;
        }
        let name_start = pos + 1;
        let Some(name_end) = read_name_end(bytes, name_start, limit) else {
            return (tokens, false);
        };
        if name_end == name_start {
            return (tokens, false);
        }
        let mut attrs: Vec<RawAttr> = Vec::new();
        let mut self_closing = false;
        let mut i = name_end;
        loop {
            i = skip_ws(bytes, i, limit);
            if i >= limit {
                return (tokens, false);
            }
            match bytes[i] {
                b'>' => {
                    i += 1;
                    break;
                }
                b'/' => {
                    if i + 1 < limit && bytes[i + 1] == b'>' {
                        self_closing = true;
                        i += 2;
                        break;
                    }
                    return (tokens, false);
                }
                _ => {}
            }
            let Some(attr_end) = read_name_end(bytes, i, limit) else {
                return (tokens, false);
            };
            if attr_end == i {
                return (tokens, false);
            }
            let name = (i, attr_end);
            i = skip_ws(bytes, attr_end, limit);
            if i >= limit || bytes[i] != b'=' {
                return (tokens, false);
            }
            i += 1;
            i = skip_ws(bytes, i, limit);
            if i >= limit || (bytes[i] != b'"' && bytes[i] != b'\'') {
                return (tokens, false);
            }
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < limit && bytes[i] != quote {
                i += 1;
            }
            if i >= limit {
                return (tokens, false);
            }
            if attrs.len() >= MAX_ATTRS {
                return (tokens, false);
            }
            attrs.push(RawAttr {
                name,
                value: (value_start, i),
            });
            i += 1;
        }
        tokens.push(RawToken {
            start: pos,
            end: i,
            kind: RawKind::Start {
                qname: (name_start, name_end),
                self_closing,
                attrs,
            },
        });
        pos = i;
    }
    (tokens, true)
}

pub(super) fn scan_packet(bytes: &[u8]) -> Scan {
    let (tokens, complete) = tokenize(bytes);
    let mut scan = Scan {
        malformed: !complete,
        // A packet past the byte cap may have been tokenised as "complete" on a
        // token boundary, so refuse to patch it.
        over_limit: bytes.len() > MAX_PACKET_BYTES,
        ..Scan::default()
    };
    let mut stack: Vec<Frame> = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();

    for tok in &tokens {
        if stack.len() > MAX_DEPTH {
            scan.over_limit = true;
            break;
        }
        match &tok.kind {
            RawKind::Text => {
                let sink = stack.iter().rposition(|frame| frame.text_buf.is_some());
                if let Some(index) = sink {
                    let frame = &mut stack[index];
                    let len = tok.end - tok.start;
                    match &mut frame.text_buf {
                        Some(buf) if buf.len() + len <= MAX_VALUE_BYTES => {
                            buf.extend_from_slice(&bytes[tok.start..tok.end]);
                        }
                        _ => frame.unsafe_ = true,
                    }
                }
            }
            RawKind::Ignored => {
                // A comment/PI/declaration inside a managed property would be
                // dropped by the rewrite, so fail closed instead.
                if let Some(owner) = nearest_field(&stack) {
                    stack[owner].unsafe_ = true;
                }
            }
            RawKind::Start {
                qname,
                self_closing,
                attrs,
            } => {
                let declared = push_declarations(&mut current, bytes, attrs);
                let name = str_at(bytes, *qname);
                let (prefix, local) = split_qname(name);
                let uri = current.get(prefix).cloned();
                let field = uri.as_deref().and_then(|u| managed_field(u, local));
                let is_rdf = uri.as_deref() == Some(RDF_URI);
                let is_description = is_rdf && local == "Description";
                let parent_is_description = stack.last().is_some_and(|frame| frame.is_description);

                let mut pushed: Option<Frame> = None;
                if is_description {
                    for attr in attrs {
                        let attr_name = str_at(bytes, attr.name);
                        let (attr_prefix, attr_local) = split_qname(attr_name);
                        let Some(attr_uri) = current.get(attr_prefix) else {
                            continue;
                        };
                        let Some(managed) = managed_field(attr_uri, attr_local) else {
                            continue;
                        };
                        let decoded = decode_text(&bytes[attr.value.0..attr.value.1]);
                        scan.spans.push(SpanInfo {
                            field: managed,
                            form: Form::Attribute,
                            start: attr.name.0,
                            end: attr.value.1 + 1,
                            name: attr_name.to_string(),
                            values: vec![LiVal {
                                lang: None,
                                value: decoded.text,
                            }],
                            container: None,
                            unsafe_: decoded.unsafe_,
                            complete: true,
                        });
                    }
                    if scan.description.is_none() {
                        scan.description = Some(Description {
                            qname: *qname,
                            start_tag: (tok.start, tok.end),
                            self_closing: *self_closing,
                            content_end: tok.end,
                            scope: current.clone(),
                        });
                    }
                    if !*self_closing {
                        let mut description = frame(*qname, tok.start);
                        description.is_description = true;
                        pushed = Some(description);
                    }
                } else if let Some(managed) = field.filter(|_| parent_is_description) {
                    // The writer emits no attributes on a managed property
                    // element, so any attribute on it would be dropped by the
                    // rewrite.
                    let unsafe_ = !attrs.is_empty();
                    if *self_closing {
                        scan.spans.push(SpanInfo {
                            field: managed,
                            form: Form::Element,
                            start: tok.start,
                            end: tok.end,
                            name: name.to_string(),
                            values: Vec::new(),
                            container: None,
                            unsafe_,
                            complete: true,
                        });
                    } else {
                        let mut managed_frame = frame(*qname, tok.start);
                        managed_frame.field = Some(managed);
                        managed_frame.unsafe_ = unsafe_;
                        managed_frame.text_buf = Some(Vec::new());
                        pushed = Some(managed_frame);
                    }
                } else if is_rdf && matches!(local, "Alt" | "Seq" | "Bag") {
                    if let Some(owner) = nearest_field(&stack) {
                        let kind = match local {
                            "Alt" => ContainerKind::Alt,
                            "Seq" => ContainerKind::Seq,
                            _ => ContainerKind::Bag,
                        };
                        stack[owner].container = Some((kind, prefix.to_string()));
                        stack[owner].text_buf = None;
                        if !attrs.is_empty() {
                            stack[owner].unsafe_ = true;
                        }
                    }
                    if !*self_closing {
                        pushed = Some(frame(*qname, tok.start));
                    }
                } else if is_rdf && local == "li" {
                    let owner = stack
                        .iter()
                        .rposition(|frame| frame.field.is_some() && frame.container.is_some());
                    let lang = attrs.iter().find_map(|attr| {
                        (str_at(bytes, attr.name) == "xml:lang")
                            .then(|| decode_text(&bytes[attr.value.0..attr.value.1]).text)
                    });
                    if let Some(owner) = owner {
                        if attrs
                            .iter()
                            .any(|attr| str_at(bytes, attr.name) != "xml:lang")
                        {
                            stack[owner].unsafe_ = true;
                        }
                    }
                    if *self_closing {
                        if let Some(owner) = owner {
                            stack[owner].values.push(LiVal {
                                lang,
                                value: String::new(),
                            });
                        }
                    } else {
                        let mut li_frame = frame(*qname, tok.start);
                        li_frame.is_li = true;
                        li_frame.li_lang = lang;
                        li_frame.li_owner = owner;
                        li_frame.text_buf = Some(Vec::new());
                        pushed = Some(li_frame);
                    }
                } else {
                    if let Some(owner) = nearest_field(&stack) {
                        stack[owner].unsafe_ = true;
                    }
                    if !*self_closing {
                        pushed = Some(frame(*qname, tok.start));
                    }
                }
                match pushed {
                    Some(mut pending) => {
                        pending.ns_undo = declared;
                        stack.push(pending);
                    }
                    None => restore_declarations(&mut current, declared),
                }
            }
            RawKind::End { qname } => {
                let Some(top) = stack.last() else {
                    scan.malformed = true;
                    break;
                };
                if str_at(bytes, top.qname) != str_at(bytes, *qname) {
                    scan.malformed = true;
                    break;
                }
                let mut frame = stack.pop().expect("checked non-empty");
                restore_declarations(&mut current, std::mem::take(&mut frame.ns_undo));
                if frame.is_description {
                    if let Some(description) = scan.description.as_mut() {
                        description.content_end = tok.start;
                    }
                    continue;
                }
                if frame.is_li {
                    if let Some(owner) = frame.li_owner {
                        let decoded = decode_text(frame.text_buf.as_deref().unwrap_or(&[]));
                        if let Some(target) = stack.get_mut(owner) {
                            target.unsafe_ |= decoded.unsafe_;
                            target.values.push(LiVal {
                                lang: frame.li_lang,
                                value: decoded.text,
                            });
                        }
                    }
                    continue;
                }
                if let Some(managed) = frame.field {
                    let (values, unsafe_) = if frame.container.is_some() {
                        (frame.values, frame.unsafe_)
                    } else {
                        let decoded = decode_text(frame.text_buf.as_deref().unwrap_or(&[]));
                        (
                            vec![LiVal {
                                lang: None,
                                value: decoded.text,
                            }],
                            frame.unsafe_ || decoded.unsafe_,
                        )
                    };
                    scan.spans.push(SpanInfo {
                        field: managed,
                        form: Form::Element,
                        start: frame.start,
                        end: tok.end,
                        name: str_at(bytes, frame.qname).to_string(),
                        values,
                        container: frame.container,
                        unsafe_,
                        complete: true,
                    });
                }
            }
        }
    }

    let unclosed = !stack.is_empty();
    while let Some(mut frame) = stack.pop() {
        restore_declarations(&mut current, std::mem::take(&mut frame.ns_undo));
        if frame.is_li {
            if let Some(owner) = frame.li_owner {
                let decoded = decode_text(frame.text_buf.as_deref().unwrap_or(&[]));
                if let Some(target) = stack.get_mut(owner) {
                    target.values.push(LiVal {
                        lang: frame.li_lang,
                        value: decoded.text,
                    });
                }
            }
            continue;
        }
        if let Some(managed) = frame.field {
            let (values, unsafe_) = if frame.container.is_some() {
                (frame.values, frame.unsafe_)
            } else {
                let decoded = decode_text(frame.text_buf.as_deref().unwrap_or(&[]));
                (
                    vec![LiVal {
                        lang: None,
                        value: decoded.text,
                    }],
                    frame.unsafe_ || decoded.unsafe_,
                )
            };
            scan.spans.push(SpanInfo {
                field: managed,
                form: Form::Element,
                start: frame.start,
                end: bytes.len(),
                name: str_at(bytes, frame.qname).to_string(),
                values,
                container: frame.container,
                unsafe_,
                complete: false,
            });
        }
    }
    scan.malformed |= unclosed;
    scan
}

fn frame(qname: (usize, usize), start: usize) -> Frame {
    Frame {
        qname,
        is_description: false,
        field: None,
        start,
        values: Vec::new(),
        container: None,
        unsafe_: false,
        text_buf: None,
        is_li: false,
        li_lang: None,
        li_owner: None,
        ns_undo: Vec::new(),
    }
}
