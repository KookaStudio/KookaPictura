//! The document's XMP metadata packet and the PDF / XMP date strings, kept
//! equal to the `/Info` entries as PDF/A-1 requires.

use super::options::Standard;

/// A UTC civil time from Unix seconds.
fn civil(unix: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    // Howard Hinnant's days-from-civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month,
        day,
        (secs / 3600) as u32,
        (secs / 60 % 60) as u32,
        (secs % 60) as u32,
    )
}

/// `D:YYYYMMDDHHmmSSZ`.
pub(crate) fn pdf_date(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix);
    format!("D:{y:04}{mo:02}{d:02}{h:02}{mi:02}{s:02}Z")
}

/// `YYYY-MM-DDTHH:mm:SSZ`.
pub(crate) fn xmp_date(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The metadata the packet and `/Info` share.
pub(crate) struct DocInfo<'a> {
    pub title: &'a str,
    pub producer: &'a str,
    pub creator: &'a str,
    pub now: i64,
    pub document_id: &'a str,
}

/// A complete XMP packet with Dublin Core, XMP basic, PDF, media management,
/// and the standard's identification schema.
pub(crate) fn packet(info: &DocInfo, standard: Standard) -> String {
    let date = xmp_date(info.now);
    let mut identification = String::new();
    match standard {
        Standard::A1b2005 => identification.push_str(
            "   <rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n\
             \x20   <pdfaid:part>1</pdfaid:part>\n\
             \x20   <pdfaid:conformance>B</pdfaid:conformance>\n\
             \x20  </rdf:Description>\n",
        ),
        Standard::X42008 => identification.push_str(
            "   <rdf:Description rdf:about=\"\" xmlns:pdfxid=\"http://www.npes.org/pdfx/ns/id/\">\n\
             \x20   <pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>\n\
             \x20  </rdf:Description>\n",
        ),
        _ => {}
    }
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
   <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
    <dc:format>application/pdf</dc:format>\n\
    <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{title}</rdf:li></rdf:Alt></dc:title>\n\
   </rdf:Description>\n\
   <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n\
    <xmp:CreateDate>{date}</xmp:CreateDate>\n\
    <xmp:ModifyDate>{date}</xmp:ModifyDate>\n\
    <xmp:MetadataDate>{date}</xmp:MetadataDate>\n\
    <xmp:CreatorTool>{creator}</xmp:CreatorTool>\n\
   </rdf:Description>\n\
   <rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n\
    <pdf:Producer>{producer}</pdf:Producer>\n\
    <pdf:Trapped>False</pdf:Trapped>\n\
   </rdf:Description>\n\
   <rdf:Description rdf:about=\"\" xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\">\n\
    <xmpMM:DocumentID>uuid:{id}</xmpMM:DocumentID>\n\
    <xmpMM:InstanceID>uuid:{id}</xmpMM:InstanceID>\n\
    <xmpMM:VersionID>1</xmpMM:VersionID>\n\
    <xmpMM:RenditionClass>default</xmpMM:RenditionClass>\n\
   </rdf:Description>\n\
{identification} </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>",
        title = escape(info.title),
        creator = escape(info.creator),
        producer = escape(info.producer),
        id = info.document_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_format_in_utc() {
        // 2026-10-17 12:34:56 UTC.
        let t = 1_792_240_496;
        assert_eq!(pdf_date(t), "D:20261017123456Z");
        assert_eq!(xmp_date(t), "2026-10-17T12:34:56Z");
        assert_eq!(xmp_date(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn the_packet_identifies_the_standard() {
        let info = DocInfo {
            title: "a <b>",
            producer: "Kooka Pictura",
            creator: "Kooka Pictura",
            now: 0,
            document_id: "00",
        };
        let a = packet(&info, Standard::A1b2005);
        assert!(a.contains("<pdfaid:part>1</pdfaid:part>"));
        assert!(a.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
        assert!(a.contains("a &lt;b&gt;"));
        assert!(packet(&info, Standard::X42008).contains(">PDF/X-4<"));
        assert!(!packet(&info, Standard::None).contains("pdfaid"));
    }
}
