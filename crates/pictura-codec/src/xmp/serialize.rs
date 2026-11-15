//! Standalone XMP packet serializer for the managed property set.
//!
//! [`to_xmp_packet`] builds a fresh packet from scratch, declaring the managed
//! prefixes and writing each property in its RDF form. Every declared prefix is
//! bound, so the result is self-contained and `parse_xmp` reads it back to the
//! same properties.

use super::{canonical, escape_xml, XmpField, XmpProperties, DC_URI, PHOTOSHOP_URI, XMPRIGHTS_URI};

const HEAD: &str = "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\n";
const TAIL: &str = "</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>";

/// Serialize the managed properties into a standalone, well-formed XMP packet.
/// Empty/absent properties are omitted; `parse_xmp` recovers the set.
pub fn to_xmp_packet(props: &XmpProperties) -> String {
    let mut out = String::from(HEAD);
    alt(&mut out, XmpField::Title, props.title.as_deref());
    list(&mut out, XmpField::Creator, "Seq", &props.creator);
    alt(
        &mut out,
        XmpField::Description,
        props.description.as_deref(),
    );
    list(&mut out, XmpField::Subject, "Bag", &props.subject);
    alt(&mut out, XmpField::Rights, props.rights.as_deref());
    simple(&mut out, XmpField::Credit, props.credit.as_deref());
    simple(&mut out, XmpField::Source, props.source.as_deref());
    simple(&mut out, XmpField::Headline, props.headline.as_deref());
    simple(&mut out, XmpField::City, props.city.as_deref());
    simple(&mut out, XmpField::State, props.state.as_deref());
    simple(&mut out, XmpField::Country, props.country.as_deref());
    simple(
        &mut out,
        XmpField::Instructions,
        props.instructions.as_deref(),
    );
    simple(
        &mut out,
        XmpField::AuthorsPosition,
        props.authors_position.as_deref(),
    );
    simple(
        &mut out,
        XmpField::CaptionWriter,
        props.caption_writer.as_deref(),
    );
    simple(
        &mut out,
        XmpField::WebStatement,
        props.web_statement.as_deref(),
    );
    alt(&mut out, XmpField::UsageTerms, props.usage_terms.as_deref());
    if let Some(marked) = props.marked {
        let q = qname(XmpField::Marked);
        let text = if marked { "True" } else { "False" };
        out.push_str(&format!("<{q}>{text}</{q}>\n"));
    }
    out.push_str(TAIL);
    out
}

fn qname(field: XmpField) -> String {
    let (uri, local) = canonical(field);
    let prefix = match uri {
        DC_URI => "dc",
        PHOTOSHOP_URI => "photoshop",
        XMPRIGHTS_URI => "xmpRights",
        _ => "rdf",
    };
    format!("{prefix}:{local}")
}

fn alt(out: &mut String, field: XmpField, value: Option<&str>) {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return;
    };
    let q = qname(field);
    out.push_str(&format!(
        "<{q}><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></{q}>\n",
        escape_xml(value)
    ));
}

fn list(out: &mut String, field: XmpField, container: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    let q = qname(field);
    out.push_str(&format!("<{q}><rdf:{container}>"));
    for value in values {
        out.push_str(&format!("<rdf:li>{}</rdf:li>", escape_xml(value)));
    }
    out.push_str(&format!("</rdf:{container}></{q}>\n"));
}

fn simple(out: &mut String, field: XmpField, value: Option<&str>) {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return;
    };
    let q = qname(field);
    out.push_str(&format!("<{q}>{}</{q}>\n", escape_xml(value)));
}
