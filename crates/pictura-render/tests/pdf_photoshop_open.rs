//! Reopening a Photoshop PDF saved with Preserve Photoshop Editing
//! Capabilities (#627, `interop/pdf-eps-import`): the page's
//! `/PieceInfo /AdobePhotoshop /Private` data is rebuilt into a PSD.
//!
//! The fixture repackages a PSD Pictura writes into the layout Photoshop's own
//! output uses (read from a file Photoshop saved): `/ImageResources`, the
//! layer data as `8BIMLayr` / `8BIMLMsk` / further tagged blocks padded to 4
//! bytes in `/StandardImageFileData`, and a Flate `/CompositeImage` with TIFF
//! predictor 2.

use std::io::Write as _;

use pictura_codec::pdf::photoshop_psd;
use pictura_core::{Document, TypeSpec};

fn type_doc() -> Document {
    let (w, h) = (96usize, 40usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        rgba.extend([(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, 255]);
    }
    let mut doc = Document::from_rgba("photo", w as u32, h as u32, &rgba);
    let mut spec = TypeSpec::new("PDF FROM PHOTOSHOP", "Liberation Sans", 12.0);
    spec.origin = (4.0, 24.0);
    assert!(!pictura_render::add_type_layer(&mut doc, "0", &spec).is_empty());
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

fn be32(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn flate(data: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(data).unwrap();
    z.finish().unwrap()
}

fn pad4(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// `psd`'s image resources and layer data, plus `doc`'s composite, laid out as
/// Photoshop lays them out in a PDF page's `/PieceInfo /AdobePhotoshop`.
fn photoshop_style_pdf(psd: &[u8], doc: &Document) -> Vec<u8> {
    let mut at = 26;
    at += 4 + be32(psd, at); // colour mode data
    let resources = psd[at + 4..at + 4 + be32(psd, at)].to_vec();
    at += 4 + be32(psd, at);
    let section_end = at + 4 + be32(psd, at);
    at += 4;
    let layer_info = &psd[at + 4..at + 4 + be32(psd, at)];
    at += 4 + be32(psd, at);
    let global_mask = &psd[at + 4..at + 4 + be32(psd, at)];
    at += 4 + be32(psd, at);

    let mut blocks = Vec::new();
    for (key, data) in [(b"Layr", layer_info), (b"LMsk", global_mask)] {
        blocks.extend_from_slice(b"8BIM");
        blocks.extend_from_slice(key);
        blocks.extend_from_slice(&(data.len() as u32).to_be_bytes());
        blocks.extend_from_slice(data);
        pad4(&mut blocks);
    }
    while at + 12 <= section_end && &psd[at..at + 2] == b"8B" {
        let len = be32(psd, at + 8);
        blocks.extend_from_slice(&psd[at..at + 12 + len]);
        pad4(&mut blocks);
        at += 12 + len;
        while at < section_end && psd[at] == 0 {
            at += 1;
        }
    }

    // Interleaved RGB, then TIFF predictor 2 (each sample minus its left neighbour).
    let (w, h) = (doc.width as usize, doc.height as usize);
    let plane = w * h;
    let mut rgb = Vec::with_capacity(plane * 3);
    for p in 0..plane {
        for c in 0..3 {
            rgb.push(doc.composite.data[c * plane + p]);
        }
    }
    let mut predicted = rgb.clone();
    for y in 0..h {
        for i in 3..w * 3 {
            let at = y * w * 3 + i;
            predicted[at] = rgb[at].wrapping_sub(rgb[at - 3]);
        }
    }

    let stream = |dict: String, data: Vec<u8>| {
        let mut out = format!("<<{dict}/Length {}>>\nstream\n", data.len()).into_bytes();
        out.extend_from_slice(&data);
        out.extend_from_slice(b"\nendstream");
        out
    };
    let objects: Vec<Vec<u8>> = vec![
        b"<</Pages 2 0 R/Type/Catalog>>".to_vec(),
        b"<</Count 1/Kids[3 0 R]/Type/Pages>>".to_vec(),
        format!(
            "<</MediaBox[0 0 {w} {h}]/Parent 2 0 R/PieceInfo<</AdobePhotoshop<</LastModified(D:20261017130844)/Private 4 0 R>>>>/Type/Page>>"
        )
        .into_bytes(),
        b"<</CompositeImage 5 0 R/ImageResources 6 0 R/RoundTrip true/StandardImageFileData 7 0 R>>"
            .to_vec(),
        stream(
            format!(
                "/BitsPerComponent 8/ColorSpace/DeviceRGB/DecodeParms<</BitsPerComponent 8/Colors 3/Columns {w}/Predictor 2>>/Filter/FlateDecode/Height {h}/Subtype/Image/Type/XObject/Width {w}"
            ),
            flate(&predicted),
        ),
        stream("/Filter/FlateDecode".into(), flate(&resources)),
        stream("/Filter/FlateDecode".into(), flate(&blocks)),
    ];
    let mut pdf = b"%PDF-1.4\r%\xe2\xe3\xcf\xd3\r\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\r", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\rendobj\r");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

// pdf_photoshop_layers
#[test]
fn photoshop_pdf_reopens_with_its_layers() {
    let doc = type_doc();
    let psd = pictura_codec::write_psd(&doc).expect("writes");
    let pdf = photoshop_style_pdf(&psd, &doc);

    let rebuilt = photoshop_psd(&pdf).expect("the editing data rebuilds a PSD");
    let reopened = pictura_codec::read_psd(&rebuilt).expect("the PSD reads");
    assert_eq!((reopened.width, reopened.height), (doc.width, doc.height));
    let names = |d: &Document| d.layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&reopened), names(&doc));

    let text = reopened
        .layers
        .iter()
        .find(|l| l.is_type())
        .expect("the type layer stays a type layer");
    assert_eq!(
        text.type_tool.as_ref().map(|t| t.text.as_str()),
        Some("PDF FROM PHOTOSHOP")
    );
    assert_eq!(
        reopened.layers[0].channels[0].data.as_ref(),
        doc.layers[0].channels[0].data.as_ref(),
        "layer pixels survive"
    );
    // The composite came through predictor 2 intact.
    let plane = (doc.width * doc.height) as usize;
    assert_eq!(
        &reopened.composite.data[..3 * plane],
        &doc.composite.data[..3 * plane]
    );
}

// pdf_photoshop_layers
#[test]
fn a_pdf_without_photoshop_data_is_not_rebuilt() {
    let pdf = b"%PDF-1.4\n1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\ntrailer\n<</Root 1 0 R>>\n%%EOF\n";
    assert!(photoshop_psd(pdf).is_none());
}
