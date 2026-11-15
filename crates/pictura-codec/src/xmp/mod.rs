//! XMP (RDF/XML) property reader and span patcher.
//!
//! A bounded, entity-free scanner locates the managed properties inside the
//! `rdf:Description` element(s) and records their byte spans in the original
//! packet; [`patch_xmp`] replaces only those spans, so unknown namespaces,
//! unknown properties, comments, and the packet wrapper round-trip verbatim.
//! Only the five predefined XML entities are decoded and no external resource
//! is ever fetched, so an XXE payload has no effect.
//!
//! ponytail: only the fixed property set is modelled; arbitrary RDF and IPTC
//! Extension are preserved but not editable. A packet without a recognisable
//! `rdf:Description`, or a managed property using a construct the patcher
//! cannot safely rewrite, leaves the resource untouched.

use std::collections::HashMap;

const RDF_URI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const DC_URI: &str = "http://purl.org/dc/elements/1.1/";
const PHOTOSHOP_URI: &str = "http://ns.adobe.com/photoshop/1.0/";
const XMPRIGHTS_URI: &str = "http://ns.adobe.com/xap/1.0/rights/";

/// Caps so a hostile packet cannot spin the scanner.
const MAX_TOKENS: usize = 8192;
const MAX_ATTRS: usize = 8192;
const MAX_DEPTH: usize = 64;
const MAX_PACKET_BYTES: usize = 8 << 20;
const MAX_VALUE_BYTES: usize = 1 << 20;
const MAX_ENTITY_RUN: usize = 32;

/// A managed XMP property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XmpField {
    Title,
    Creator,
    Description,
    Subject,
    Rights,
    Credit,
    Source,
    Headline,
    Marked,
    City,
    State,
    Country,
    Instructions,
    AuthorsPosition,
    CaptionWriter,
    WebStatement,
    UsageTerms,
}

/// The managed property set of a document's XMP packet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmpProperties {
    pub title: Option<String>,
    pub creator: Vec<String>,
    pub description: Option<String>,
    pub subject: Vec<String>,
    pub rights: Option<String>,
    pub credit: Option<String>,
    pub source: Option<String>,
    pub headline: Option<String>,
    pub marked: Option<bool>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub instructions: Option<String>,
    pub authors_position: Option<String>,
    pub caption_writer: Option<String>,
    pub web_statement: Option<String>,
    pub usage_terms: Option<String>,
}

impl XmpProperties {
    /// Whether no managed property is present.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.creator.is_empty()
            && self.description.is_none()
            && self.subject.is_empty()
            && self.rights.is_none()
            && self.credit.is_none()
            && self.source.is_none()
            && self.headline.is_none()
            && self.marked.is_none()
            && self.city.is_none()
            && self.state.is_none()
            && self.country.is_none()
            && self.instructions.is_none()
            && self.authors_position.is_none()
            && self.caption_writer.is_none()
            && self.web_statement.is_none()
            && self.usage_terms.is_none()
    }
}

/// Parse the fixed property set out of an XMP packet. A malformed or truncated
/// packet yields whatever decoded so far and never panics.
pub fn parse_xmp(packet: &str) -> XmpProperties {
    scan_packet(packet.as_bytes()).properties()
}

/// Whether a packet is complete and within the scanner's bounds.
pub(crate) fn is_well_formed(packet: &str) -> bool {
    let scan = scan_packet(packet.as_bytes());
    !scan.malformed && !scan.over_limit
}

mod patch;
mod scan;
mod serialize;
pub mod tree;

pub use patch::{patch_xmp, patch_xmp_values};
pub use serialize::to_xmp_packet;

use scan::scan_packet;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Attribute,
    Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    Alt,
    Seq,
    Bag,
}

#[derive(Debug, Clone)]
struct LiVal {
    lang: Option<String>,
    value: String,
}

#[derive(Debug, Clone)]
struct SpanInfo {
    field: XmpField,
    form: Form,
    start: usize,
    end: usize,
    name: String,
    values: Vec<LiVal>,
    container: Option<(ContainerKind, String)>,
    unsafe_: bool,
    complete: bool,
}

impl SpanInfo {
    fn scalar_value(&self) -> Option<String> {
        self.values
            .iter()
            .find(|v| v.lang.as_deref() == Some("x-default"))
            .or_else(|| self.values.first())
            .map(|v| v.value.clone())
    }
}

#[derive(Debug, Clone)]
struct Description {
    qname: (usize, usize),
    start_tag: (usize, usize),
    self_closing: bool,
    content_end: usize,
    /// The namespace declarations in scope at the Description start tag, so an
    /// insertion only reuses a prefix actually bound there.
    scope: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct Scan {
    spans: Vec<SpanInfo>,
    description: Option<Description>,
    malformed: bool,
    over_limit: bool,
}

impl Scan {
    fn properties(&self) -> XmpProperties {
        let mut props = XmpProperties::default();
        for span in &self.spans {
            match span.field {
                XmpField::Title if props.title.is_none() => {
                    props.title = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Creator if props.creator.is_empty() => {
                    props.creator = span
                        .values
                        .iter()
                        .map(|v| v.value.clone())
                        .filter(|v| !v.is_empty())
                        .collect();
                }
                XmpField::Description if props.description.is_none() => {
                    props.description = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Subject if props.subject.is_empty() => {
                    props.subject = span
                        .values
                        .iter()
                        .map(|v| v.value.clone())
                        .filter(|v| !v.is_empty())
                        .collect();
                }
                XmpField::Rights if props.rights.is_none() => {
                    props.rights = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Credit if props.credit.is_none() => {
                    props.credit = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Source if props.source.is_none() => {
                    props.source = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Headline if props.headline.is_none() => {
                    props.headline = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Marked if props.marked.is_none() => {
                    props.marked = span.scalar_value().and_then(|v| parse_xmp_bool(&v));
                }
                XmpField::City if props.city.is_none() => {
                    props.city = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::State if props.state.is_none() => {
                    props.state = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Country if props.country.is_none() => {
                    props.country = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::Instructions if props.instructions.is_none() => {
                    props.instructions = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::AuthorsPosition if props.authors_position.is_none() => {
                    props.authors_position = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::CaptionWriter if props.caption_writer.is_none() => {
                    props.caption_writer = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::WebStatement if props.web_statement.is_none() => {
                    props.web_statement = span.scalar_value().filter(|v| !v.is_empty());
                }
                XmpField::UsageTerms if props.usage_terms.is_none() => {
                    props.usage_terms = span.scalar_value().filter(|v| !v.is_empty());
                }
                _ => {}
            }
        }
        props
    }
}

/// The lexicographically smallest declared prefix bound to `uri` in `scope`.
fn prefix_for(scope: &HashMap<String, String>, uri: &str) -> Option<String> {
    scope
        .iter()
        .filter(|(prefix, declared)| !prefix.is_empty() && declared.as_str() == uri)
        .map(|(prefix, _)| prefix.as_str())
        .min()
        .map(str::to_string)
}

fn managed_field(uri: &str, local: &str) -> Option<XmpField> {
    Some(match (uri, local) {
        (DC_URI, "title") => XmpField::Title,
        (DC_URI, "creator") => XmpField::Creator,
        (DC_URI, "description") => XmpField::Description,
        (DC_URI, "subject") => XmpField::Subject,
        (DC_URI, "rights") => XmpField::Rights,
        (PHOTOSHOP_URI, "Credit") => XmpField::Credit,
        (PHOTOSHOP_URI, "Source") => XmpField::Source,
        (PHOTOSHOP_URI, "Headline") => XmpField::Headline,
        (XMPRIGHTS_URI, "Marked") => XmpField::Marked,
        (PHOTOSHOP_URI, "City") => XmpField::City,
        (PHOTOSHOP_URI, "State") => XmpField::State,
        (PHOTOSHOP_URI, "Country") => XmpField::Country,
        (PHOTOSHOP_URI, "Instructions") => XmpField::Instructions,
        (PHOTOSHOP_URI, "AuthorsPosition") => XmpField::AuthorsPosition,
        (PHOTOSHOP_URI, "CaptionWriter") => XmpField::CaptionWriter,
        (XMPRIGHTS_URI, "WebStatement") => XmpField::WebStatement,
        (XMPRIGHTS_URI, "UsageTerms") => XmpField::UsageTerms,
        _ => return None,
    })
}

fn canonical(field: XmpField) -> (&'static str, &'static str) {
    match field {
        XmpField::Title => (DC_URI, "title"),
        XmpField::Creator => (DC_URI, "creator"),
        XmpField::Description => (DC_URI, "description"),
        XmpField::Subject => (DC_URI, "subject"),
        XmpField::Rights => (DC_URI, "rights"),
        XmpField::Credit => (PHOTOSHOP_URI, "Credit"),
        XmpField::Source => (PHOTOSHOP_URI, "Source"),
        XmpField::Headline => (PHOTOSHOP_URI, "Headline"),
        XmpField::Marked => (XMPRIGHTS_URI, "Marked"),
        XmpField::City => (PHOTOSHOP_URI, "City"),
        XmpField::State => (PHOTOSHOP_URI, "State"),
        XmpField::Country => (PHOTOSHOP_URI, "Country"),
        XmpField::Instructions => (PHOTOSHOP_URI, "Instructions"),
        XmpField::AuthorsPosition => (PHOTOSHOP_URI, "AuthorsPosition"),
        XmpField::CaptionWriter => (PHOTOSHOP_URI, "CaptionWriter"),
        XmpField::WebStatement => (XMPRIGHTS_URI, "WebStatement"),
        XmpField::UsageTerms => (XMPRIGHTS_URI, "UsageTerms"),
    }
}
fn parse_xmp_bool(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}
#[cfg(test)]
mod tests;
