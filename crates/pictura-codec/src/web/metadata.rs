//! Save for Web metadata levels: filter the document's preserved EXIF, IPTC,
//! and XMP before a JPEG or PNG write.

use crate::error::PsdError;
use crate::exif::{parse_exif, ExifValue};
use crate::iptc::{encode_iptc, parse_iptc, Iptc};
use crate::raster_metadata::{embed_raster_metadata, RasterMetadata};
use crate::xmp::parse_xmp;

/// The raw metadata blocks gathered from the document once, before the level
/// is chosen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebMetadata {
    pub exif: Option<Vec<u8>>,
    pub iptc: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
}

impl WebMetadata {
    pub fn is_empty(&self) -> bool {
        self.exif.is_none() && self.iptc.is_none() && self.xmp.is_none()
    }
}

/// The Save for Web Metadata menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataLevel {
    None,
    Copyright,
    CopyrightContact,
    AllExceptCamera,
    All,
}

impl MetadataLevel {
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => MetadataLevel::Copyright,
            2 => MetadataLevel::CopyrightContact,
            3 => MetadataLevel::AllExceptCamera,
            4 => MetadataLevel::All,
            _ => MetadataLevel::None,
        }
    }
}

/// The filtered blocks for `level`.
pub fn sfw_metadata(raw: &WebMetadata, level: MetadataLevel) -> RasterMetadata {
    match level {
        MetadataLevel::All => RasterMetadata {
            exif: raw.exif.clone(),
            iptc: raw.iptc.clone(),
            xmp: raw.xmp.clone(),
            ..RasterMetadata::default()
        },
        MetadataLevel::AllExceptCamera => RasterMetadata {
            exif: None,
            iptc: raw.iptc.clone(),
            xmp: raw.xmp.clone(),
            ..RasterMetadata::default()
        },
        MetadataLevel::None => RasterMetadata {
            exif: raw.exif.as_deref().and_then(minimal_copyright),
            ..RasterMetadata::default()
        },
        MetadataLevel::Copyright | MetadataLevel::CopyrightContact => {
            let iptc = raw.iptc.as_deref().map(parse_iptc).unwrap_or_default();
            let xmp = raw
                .xmp
                .as_deref()
                .map(|bytes| parse_xmp(&String::from_utf8_lossy(bytes)))
                .unwrap_or_default();
            let exif = raw.exif.as_deref().and_then(exif_copyright);
            let contact = level == MetadataLevel::CopyrightContact;
            RasterMetadata {
                exif: None,
                iptc: build_iptc(&iptc, &xmp, contact, exif.as_deref()),
                xmp: build_xmp(&iptc, &xmp, contact, exif.as_deref()),
                ..RasterMetadata::default()
            }
        }
    }
}

/// Filter and embed metadata into an encoded JPEG or PNG. GIF, WBMP, and an
/// empty selection return the bytes unchanged.
pub fn embed_web_metadata(
    bytes: &[u8],
    suffix: &str,
    raw: &WebMetadata,
    level: MetadataLevel,
) -> Result<Vec<u8>, PsdError> {
    let metadata = sfw_metadata(raw, level);
    if suffix.eq_ignore_ascii_case("png") || suffix.eq_ignore_ascii_case("png8") {
        let Some(xmp) = metadata.xmp.as_deref() else {
            return Ok(bytes.to_vec());
        };
        return crate::png::attach_xmp(bytes, xmp)
            .ok_or_else(|| PsdError::Invalid("PNG has no IEND chunk".into()));
    }
    embed_raster_metadata(bytes, suffix, &metadata)
}

fn build_iptc(
    iptc: &Iptc,
    xmp: &crate::xmp::XmpProperties,
    contact: bool,
    exif_copyright: Option<&str>,
) -> Option<Vec<u8>> {
    let copyright = iptc
        .text(2, 116)
        .or_else(|| xmp.rights.clone())
        .or_else(|| exif_copyright.map(str::to_string))
        .filter(|v| !v.is_empty());
    let creator = iptc
        .text(2, 80)
        .or_else(|| xmp.creator.first().cloned())
        .filter(|v| !v.is_empty());
    let credit = iptc
        .text(2, 110)
        .or_else(|| xmp.credit.clone())
        .filter(|v| !v.is_empty());
    let contact_info = iptc.text(2, 118).filter(|v| !v.is_empty());
    let mut out = Iptc::default();
    let mut any = false;
    if let Some(value) = copyright {
        any |= out.set(2, 116, value.as_bytes());
    }
    if contact {
        for (dataset, value) in [(80u8, creator), (110, credit), (118, contact_info)] {
            if let Some(value) = value {
                any |= out.set(2, dataset, value.as_bytes());
            }
        }
    }
    any.then(|| encode_iptc(&out))
}

fn build_xmp(
    iptc: &Iptc,
    xmp: &crate::xmp::XmpProperties,
    contact: bool,
    exif_copyright: Option<&str>,
) -> Option<Vec<u8>> {
    let rights = xmp
        .rights
        .clone()
        .filter(|v| !v.is_empty())
        .or_else(|| iptc.text(2, 116).filter(|v| !v.is_empty()))
        .or_else(|| exif_copyright.map(str::to_string));
    let creator: Vec<String> = if xmp.creator.is_empty() {
        iptc.text(2, 80)
            .filter(|v| !v.is_empty())
            .into_iter()
            .collect()
    } else {
        xmp.creator.clone()
    };
    let credit = xmp
        .credit
        .clone()
        .filter(|v| !v.is_empty())
        .or_else(|| iptc.text(2, 110).filter(|v| !v.is_empty()));
    let has_creator = contact && !creator.is_empty();
    let has_credit = contact && credit.is_some();
    if rights.is_none() && !has_creator && !has_credit {
        return None;
    }
    let mut out = String::from(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n",
    );
    if let Some(rights) = rights {
        out.push_str(&format!(
            "<dc:rights><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:rights>\n",
            escape_xml(&rights)
        ));
    }
    if has_creator {
        out.push_str("<dc:creator><rdf:Seq>");
        for name in &creator {
            out.push_str(&format!("<rdf:li>{}</rdf:li>", escape_xml(name)));
        }
        out.push_str("</rdf:Seq></dc:creator>\n");
    }
    if let Some(credit) = credit.filter(|_| has_credit) {
        out.push_str(&format!(
            "<photoshop:Credit>{}</photoshop:Credit>\n",
            escape_xml(&credit)
        ));
    }
    out.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>\n");
    Some(out.into_bytes())
}

fn exif_copyright(exif: &[u8]) -> Option<String> {
    match parse_exif(exif).get(0x8298)? {
        ExifValue::Ascii(text) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn minimal_copyright(exif: &[u8]) -> Option<Vec<u8>> {
    exif_copyright(exif).map(|text| copyright_exif(&text))
}

fn copyright_exif(text: &str) -> Vec<u8> {
    let value = text.as_bytes();
    let size = (value.len() + 1) as u32;
    let value_at = 8 + 2 + 12 + 4;
    let mut out = Vec::with_capacity(value_at + if size > 4 { value.len() + 1 } else { 0 });
    out.extend_from_slice(b"II*\0");
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0x8298u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    if size <= 4 {
        let mut inline = [0u8; 4];
        inline[..value.len()].copy_from_slice(value);
        out.extend_from_slice(&inline);
    } else {
        out.extend_from_slice(&(value_at as u32).to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    if size > 4 {
        out.extend_from_slice(value);
        out.push(0);
    }
    out
}

fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw() -> WebMetadata {
        let mut iptc = Iptc::default();
        iptc.set(2, 116, b"(c) 2026 Kooka");
        iptc.set(2, 80, b"Ada Lovelace");
        iptc.set(2, 110, b"Studio");
        iptc.set(2, 118, b"ada@example.test");
        let xmp = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
                   xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
                   <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
                   <dc:rights><rdf:Alt><rdf:li xml:lang=\"x-default\">(c) 2026 Kooka</rdf:li></rdf:Alt></dc:rights>\
                   <dc:creator><rdf:Seq><rdf:li>Ada Lovelace</rdf:li></rdf:Seq></dc:creator>\
                   </rdf:Description></rdf:RDF></x:xmpmeta>";
        WebMetadata {
            exif: minimal_copyright_from("(c) 2026 Kooka"),
            iptc: Some(encode_iptc(&iptc)),
            xmp: Some(xmp.as_bytes().to_vec()),
        }
    }

    fn minimal_copyright_from(text: &str) -> Option<Vec<u8>> {
        Some(copyright_exif(text))
    }

    #[test]
    fn metadata_levels() {
        let raw = raw();
        let none = sfw_metadata(&raw, MetadataLevel::None);
        assert!(none.iptc.is_none() && none.xmp.is_none());
        let exif = parse_exif(none.exif.as_deref().unwrap());
        assert!(matches!(exif.get(0x8298), Some(ExifValue::Ascii(_))));

        let all = sfw_metadata(&raw, MetadataLevel::All);
        assert_eq!(all.exif, raw.exif);
        assert_eq!(all.iptc, raw.iptc);
        assert_eq!(all.xmp, raw.xmp);

        let except = sfw_metadata(&raw, MetadataLevel::AllExceptCamera);
        assert!(except.exif.is_none());
        assert_eq!(except.iptc, raw.iptc);
        assert_eq!(except.xmp, raw.xmp);

        let copyright = sfw_metadata(&raw, MetadataLevel::Copyright);
        let iptc = parse_iptc(copyright.iptc.as_deref().unwrap());
        assert_eq!(iptc.text(2, 116).as_deref(), Some("(c) 2026 Kooka"));
        assert!(iptc.text(2, 80).is_none());
        let xmp = String::from_utf8(copyright.xmp.unwrap()).unwrap();
        assert!(xmp.contains("dc:rights"));
        assert!(!xmp.contains("dc:creator"));

        let contact = sfw_metadata(&raw, MetadataLevel::CopyrightContact);
        let iptc = parse_iptc(contact.iptc.as_deref().unwrap());
        assert_eq!(iptc.text(2, 80).as_deref(), Some("Ada Lovelace"));
        let xmp = String::from_utf8(contact.xmp.unwrap()).unwrap();
        assert!(xmp.contains("dc:creator"));

        // With no XMP, the Copyright levels build the XMP from the IPTC.
        let mut iptc_only = Iptc::default();
        iptc_only.set(2, 116, b"(c) 2026 IPTC");
        iptc_only.set(2, 80, b"Grace Hopper");
        iptc_only.set(2, 110, b"Studio");
        let raw_iptc = WebMetadata {
            exif: None,
            iptc: Some(encode_iptc(&iptc_only)),
            xmp: None,
        };
        let copyright = sfw_metadata(&raw_iptc, MetadataLevel::Copyright);
        let xmp = String::from_utf8(copyright.xmp.unwrap()).unwrap();
        assert!(xmp.contains("dc:rights"));
        assert!(xmp.contains("(c) 2026 IPTC"));
        let contact = sfw_metadata(&raw_iptc, MetadataLevel::CopyrightContact);
        let xmp = String::from_utf8(contact.xmp.unwrap()).unwrap();
        assert!(xmp.contains("dc:creator"));
        assert!(xmp.contains("Grace Hopper"));
        assert!(xmp.contains("photoshop:Credit"));
    }

    #[test]
    fn none_keeps_exif_copyright() {
        let raw = raw();
        let none = sfw_metadata(&raw, MetadataLevel::None);
        let exif = parse_exif(none.exif.as_deref().unwrap());
        assert_eq!(
            exif.get(0x8298),
            Some(&ExifValue::Ascii("(c) 2026 Kooka".into()))
        );
    }

    #[test]
    fn copyright_levels_use_exif_notice() {
        let raw = WebMetadata {
            exif: minimal_copyright_from("(c) 2026 Exif"),
            iptc: None,
            xmp: None,
        };
        let copyright = sfw_metadata(&raw, MetadataLevel::Copyright);
        let iptc = parse_iptc(copyright.iptc.as_deref().unwrap());
        assert_eq!(iptc.text(2, 116).as_deref(), Some("(c) 2026 Exif"));
        let xmp = String::from_utf8(copyright.xmp.unwrap()).unwrap();
        assert!(xmp.contains("(c) 2026 Exif"));
    }

    #[test]
    fn short_copyright_is_inline() {
        let exif = copyright_exif("ABC");
        assert_eq!(exif.len(), 26);
        assert_eq!(
            parse_exif(&exif).get(0x8298),
            Some(&ExifValue::Ascii("ABC".into()))
        );
    }
}
