//! IPTC Extension (Iptc4xmpExt) read and write over the generic XMP tree.

use pictura_core::Document;

use crate::image_resources::{decode_image_resources, XMP_METADATA};
use crate::xmp::tree::{self, Shape, Update};

const NS: &str = "http://iptc.org/std/Iptc4xmpExt/2008-02-29/";

#[derive(Clone, Copy)]
struct Field {
    id: &'static str,
    label: &'static str,
    path: &'static [&'static str],
    shape: Shape,
}

const fn field(
    id: &'static str,
    label: &'static str,
    path: &'static [&'static str],
    shape: Shape,
) -> Field {
    Field {
        id,
        label,
        path,
        shape,
    }
}

const FIELDS: &[Field] = &[
    field(
        "Iptc4xmpExt:PersonInImage",
        "Person Shown",
        &["PersonInImage"],
        Shape::Bag,
    ),
    field(
        "Iptc4xmpExt:OrganisationInImageName",
        "Organisation Shown (Name)",
        &["OrganisationInImageName"],
        Shape::Bag,
    ),
    field(
        "Iptc4xmpExt:OrganisationInImageCode",
        "Organisation Shown (Code)",
        &["OrganisationInImageCode"],
        Shape::Bag,
    ),
    field("Iptc4xmpExt:Event", "Event", &["Event"], Shape::LangAlt),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:City",
        "Location Created - City",
        &["LocationCreated", "City"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:ProvinceState",
        "Location Created - State/Province",
        &["LocationCreated", "ProvinceState"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:CountryName",
        "Location Created - Country",
        &["LocationCreated", "CountryName"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:CountryCode",
        "Location Created - Country Code",
        &["LocationCreated", "CountryCode"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:Sublocation",
        "Location Created - Sublocation",
        &["LocationCreated", "Sublocation"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:WorldRegion",
        "Location Created - World Region",
        &["LocationCreated", "WorldRegion"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:City",
        "Location Shown - City",
        &["LocationShown", "City"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:ProvinceState",
        "Location Shown - State/Province",
        &["LocationShown", "ProvinceState"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:CountryName",
        "Location Shown - Country",
        &["LocationShown", "CountryName"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:CountryCode",
        "Location Shown - Country Code",
        &["LocationShown", "CountryCode"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:Sublocation",
        "Location Shown - Sublocation",
        &["LocationShown", "Sublocation"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:LocationShown/Iptc4xmpExt:WorldRegion",
        "Location Shown - World Region",
        &["LocationShown", "WorldRegion"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:ImageRating",
        "Image Rating",
        &["ImageRating"],
        Shape::Simple,
    ),
    field(
        "Iptc4xmpExt:ModelReleaseStatus",
        "Model Release Status",
        &["ModelReleaseStatus"],
        Shape::Simple,
    ),
    field(
        "Iptc4xmpExt:PropertyReleaseStatus",
        "Property Release Status",
        &["PropertyReleaseStatus"],
        Shape::Simple,
    ),
    field(
        "Iptc4xmpExt:DigitalSourceType",
        "Digital Source Type",
        &["DigitalSourceType"],
        Shape::Simple,
    ),
    field(
        "Iptc4xmpExt:AddlModelInfo",
        "Additional Model Information",
        &["AddlModelInfo"],
        Shape::Simple,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorName",
        "Licensor - Name",
        &["Licensor", "LicensorName"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorID",
        "Licensor - ID",
        &["Licensor", "LicensorID"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorEmail",
        "Licensor - Email",
        &["Licensor", "LicensorEmail"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorPhone",
        "Licensor - Phone",
        &["Licensor", "LicensorPhone"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorCity",
        "Licensor - City",
        &["Licensor", "LicensorCity"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorState",
        "Licensor - State/Province",
        &["Licensor", "LicensorState"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorCountry",
        "Licensor - Country",
        &["Licensor", "LicensorCountry"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorPostalCode",
        "Licensor - Postal Code",
        &["Licensor", "LicensorPostalCode"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorStreetAddress",
        "Licensor - Street Address",
        &["Licensor", "LicensorStreetAddress"],
        Shape::Struct,
    ),
    field(
        "Iptc4xmpExt:Licensor/Iptc4xmpExt:LicensorWebUrl",
        "Licensor - Web URL",
        &["Licensor", "LicensorWebUrl"],
        Shape::Struct,
    ),
];

/// The document's XMP packet as text. `Ok(None)` means there is no XMP
/// resource; `Err(())` means the resource is not valid UTF-8.
fn packet(document: &Document) -> Result<Option<String>, ()> {
    match decode_image_resources(document)
        .into_iter()
        .find(|resource| resource.id == XMP_METADATA)
    {
        Some(resource) => std::str::from_utf8(&resource.data)
            .map(|packet| Some(packet.to_string()))
            .map_err(|_| ()),
        None => Ok(None),
    }
}

/// Every supported field with its current values. A field with no value is
/// emitted once with an empty string so the dialog always has a control.
pub fn edit_rows(document: &Document) -> Vec<(String, String, String)> {
    let Ok(Some(packet)) = packet(document) else {
        return FIELDS
            .iter()
            .map(|field| (field.id.to_string(), field.label.to_string(), String::new()))
            .collect();
    };
    let tree = tree::read(&packet, NS);
    let mut rows = Vec::new();
    for field in FIELDS {
        let values = values_for(&tree, field);
        if values.is_empty() {
            rows.push((field.id.to_string(), field.label.to_string(), String::new()));
        } else {
            for value in values {
                rows.push((field.id.to_string(), field.label.to_string(), value));
            }
        }
    }
    rows
}

/// Read-only rows for artwork, PLUS structures, and repeated structures.
pub fn read_rows(document: &Document) -> Vec<(String, String)> {
    let Ok(Some(packet)) = packet(document) else {
        return Vec::new();
    };
    let tree = tree::read(&packet, NS);
    tree.leaves
        .iter()
        .filter(|leaf| !is_supported(&leaf.path))
        .map(|leaf| (leaf.path.join(" / "), leaf.value.clone()))
        .collect()
}

fn values_for(tree: &tree::Tree, field: &Field) -> Vec<String> {
    let path: Vec<String> = field.path.iter().map(|part| (*part).to_string()).collect();
    match field.shape {
        Shape::Bag => tree
            .leaves
            .iter()
            .filter(|leaf| {
                leaf.path.len() == 2 && leaf.path[0] == path[0] && leaf.path[1].starts_with("li[")
            })
            .map(|leaf| leaf.value.clone())
            .collect(),
        Shape::LangAlt => {
            let mut fallback = None;
            for leaf in &tree.leaves {
                if leaf.path == path {
                    return vec![leaf.value.clone()];
                }
                if leaf.path.len() == 2 && leaf.path[0] == path[0] {
                    if leaf.path[1] == "li[x-default]" {
                        return vec![leaf.value.clone()];
                    }
                    if fallback.is_none() {
                        fallback = Some(leaf.value.clone());
                    }
                }
            }
            fallback.into_iter().collect()
        }
        Shape::Simple => tree
            .leaves
            .iter()
            .find(|leaf| leaf.path == path)
            .map(|leaf| vec![leaf.value.clone()])
            .unwrap_or_default(),
        Shape::Struct => tree
            .leaves
            .iter()
            .find(|leaf| leaf.path == path)
            .map(|leaf| vec![leaf.value.clone()])
            .unwrap_or_default(),
    }
}

fn is_supported(path: &[String]) -> bool {
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    FIELDS.iter().any(|field| {
        field.path == path.as_slice()
            || (matches!(field.shape, Shape::Bag | Shape::LangAlt)
                && path.len() == 2
                && path[0] == field.path[0]
                && path[1].starts_with("li["))
    })
}

/// Apply the supported extension edits. Any unknown id or unwritable shape
/// aborts the whole apply before the document is mutated.
pub fn set_values(document: &mut Document, updates: &[(String, Vec<String>)]) -> bool {
    set_values_outcome(document, updates).unwrap_or(false)
}

/// The checked form of [`set_values`]: `None` means a requested field was
/// refused, `Some(false)` means nothing changed, `Some(true)` means it did.
pub fn set_values_outcome(
    document: &mut Document,
    updates: &[(String, Vec<String>)],
) -> Option<bool> {
    let original = match packet(document) {
        Ok(Some(packet)) => packet,
        Ok(None) => super::minimal_xmp_packet().to_string(),
        Err(()) => return None,
    };
    let mut packet = original.clone();
    for (id, values) in updates {
        let field = FIELDS.iter().find(|field| field.id == id)?;
        let update = Update {
            path: field.path.iter().map(|part| (*part).to_string()).collect(),
            shape: field.shape,
            values: values.clone(),
        };
        packet = tree::patch(&packet, NS, &[update])?;
    }
    if packet == original {
        return Some(false);
    }
    Some(super::set_xmp_packet(document, &packet))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_resources::{encode_image_resources, ImageResource};
    use pictura_core::{BitDepth, ColorMode};

    const HEAD: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">";
    const TAIL: &str = "</rdf:RDF></x:xmpmeta>";

    fn resource(id: u16, name: &str, data: &[u8]) -> ImageResource {
        let mut raw = b"8BIM".to_vec();
        raw.extend_from_slice(&id.to_be_bytes());
        raw.push(name.len() as u8);
        raw.extend_from_slice(name.as_bytes());
        if !(name.len() + 1).is_multiple_of(2) {
            raw.push(0);
        }
        raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
        raw.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            raw.push(0);
        }
        ImageResource {
            id,
            name: name.to_string(),
            data: data.to_vec(),
            raw,
        }
    }

    fn packet_doc(description: &str) -> Document {
        let packet = format!(
            "{HEAD}<rdf:Description xmlns:Iptc4xmpExt=\"{NS}\" xml:lang=\"x-default\">{description}</rdf:Description>{TAIL}"
        );
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources =
            encode_image_resources(&[resource(XMP_METADATA, "xmp", packet.as_bytes())]).into();
        doc
    }

    #[test]
    fn iptc_extension_reads_the_fixture_fields() {
        let bytes = include_bytes!("../../tests/fixtures/metadata.psd");
        let doc = crate::read_psd(bytes).expect("fixture");
        let rows = edit_rows(&doc);
        assert!(rows.iter().any(|(id, _, value)| {
            id == "Iptc4xmpExt:PersonInImage" && value == "Ada Lovelace"
        }));
        assert!(rows
            .iter()
            .any(|(id, _, value)| id == "Iptc4xmpExt:Event" && value == "Fixture Event"));
        assert!(rows.iter().any(|(id, _, value)| {
            id == "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:City" && value == "Paris"
        }));
        assert!(rows
            .iter()
            .any(|(id, _, value)| id == "Iptc4xmpExt:ImageRating" && value == "4"));
    }

    #[test]
    fn iptc_extension_missing_namespace_is_empty() {
        let packet =
            format!("{HEAD}<rdf:Description xmlns:acme=\"http://example.com/acme/\"/>{TAIL}");
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources =
            encode_image_resources(&[resource(XMP_METADATA, "xmp", packet.as_bytes())]).into();
        assert!(read_rows(&doc).is_empty());
    }

    #[test]
    fn iptc_extension_malformed_packet_does_not_panic() {
        let doc = packet_doc("<Iptc4xmpExt:Event>One");
        let _ = read_rows(&doc);
    }

    #[test]
    fn iptc_extension_adds_a_person_bag() {
        let mut doc = packet_doc("");
        assert!(set_values(
            &mut doc,
            &[(
                "Iptc4xmpExt:PersonInImage".to_string(),
                vec!["Ada".to_string(), "Grace".to_string()],
            )]
        ));
        let rows = edit_rows(&doc);
        let people: Vec<_> = rows
            .iter()
            .filter(|(id, _, _)| id == "Iptc4xmpExt:PersonInImage")
            .map(|(_, _, value)| value.as_str())
            .collect();
        assert_eq!(people, vec!["Ada", "Grace"]);
    }

    #[test]
    fn iptc_extension_edits_a_location_leaf() {
        let mut doc = packet_doc(
            "<Iptc4xmpExt:LocationCreated><rdf:Description Iptc4xmpExt:City=\"Paris\"/></Iptc4xmpExt:LocationCreated>",
        );
        assert!(set_values(
            &mut doc,
            &[(
                "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:City".to_string(),
                vec!["Lyon".to_string()],
            )]
        ));
        assert!(edit_rows(&doc).iter().any(|(_, _, value)| value == "Lyon"));
    }

    #[test]
    fn iptc_extension_clears_a_field() {
        let mut doc = packet_doc("<Iptc4xmpExt:Event>Fixture</Iptc4xmpExt:Event>");
        assert!(set_values(
            &mut doc,
            &[("Iptc4xmpExt:Event".to_string(), Vec::new())]
        ));
        assert!(!edit_rows(&doc)
            .iter()
            .any(|(id, _, value)| id == "Iptc4xmpExt:Event" && !value.is_empty()));
    }

    #[test]
    fn iptc_extension_preserves_unknown_extension_bytes() {
        let mut doc = packet_doc(
            "<Iptc4xmpExt:Event>Old</Iptc4xmpExt:Event><Iptc4xmpExt:UnknownThing>keep</Iptc4xmpExt:UnknownThing>",
        );
        assert!(set_values(
            &mut doc,
            &[("Iptc4xmpExt:Event".to_string(), vec!["New".to_string()])]
        ));
        let xmp = crate::read_metadata(&doc).xmp;
        assert!(xmp.contains("<Iptc4xmpExt:UnknownThing>keep</Iptc4xmpExt:UnknownThing>"));
    }

    #[test]
    fn iptc_extension_unsupported_shape_aborts() {
        let mut doc = packet_doc(
            "<Iptc4xmpExt:LocationCreated><!--keep--><rdf:Description Iptc4xmpExt:City=\"Paris\"/></Iptc4xmpExt:LocationCreated>",
        );
        let before = doc.image_resources.clone();
        assert!(!set_values(
            &mut doc,
            &[(
                "Iptc4xmpExt:LocationCreated/Iptc4xmpExt:ProvinceState".to_string(),
                vec!["Ile-de-France".to_string()],
            )]
        ));
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn iptc_extension_artwork_survives_edit() {
        let mut doc = packet_doc(
            "<Iptc4xmpExt:Event>Old</Iptc4xmpExt:Event>\
             <Iptc4xmpExt:ArtworkOrObject><rdf:Description Iptc4xmpExt:Title=\"Art\"/></Iptc4xmpExt:ArtworkOrObject>",
        );
        assert!(set_values(
            &mut doc,
            &[("Iptc4xmpExt:Event".to_string(), vec!["New".to_string()])]
        ));
        let xmp = crate::read_metadata(&doc).xmp;
        assert!(xmp.contains("Iptc4xmpExt:ArtworkOrObject"));
        assert!(xmp.contains("Iptc4xmpExt:Title=\"Art\""));
    }

    #[test]
    fn iptc_extension_lists_artwork_read_only() {
        let doc = packet_doc(
            "<Iptc4xmpExt:ArtworkOrObject><rdf:Description Iptc4xmpExt:Title=\"Art\"/></Iptc4xmpExt:ArtworkOrObject>",
        );
        assert!(read_rows(&doc)
            .iter()
            .any(|(label, value)| label == "ArtworkOrObject / Title" && value == "Art"));
    }

    #[test]
    fn iptc_extension_lists_extra_structures_read_only() {
        let doc = packet_doc(
            "<Iptc4xmpExt:LocationShown><rdf:Description Iptc4xmpExt:City=\"Paris\"/></Iptc4xmpExt:LocationShown>\
             <Iptc4xmpExt:LocationShown><rdf:Description Iptc4xmpExt:City=\"Lyon\"/></Iptc4xmpExt:LocationShown>",
        );
        assert!(read_rows(&doc)
            .iter()
            .any(|(label, _)| label.starts_with("LocationShown[1]")));
    }
    #[test]
    fn iptc_extension_creates_a_packet_when_absent() {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        assert_eq!(
            set_values_outcome(
                &mut doc,
                &[(
                    "Iptc4xmpExt:PersonInImage".to_string(),
                    vec!["Ada".to_string()],
                )]
            ),
            Some(true)
        );
        assert!(edit_rows(&doc)
            .iter()
            .any(|(id, _, value)| id == "Iptc4xmpExt:PersonInImage" && value == "Ada"));
    }

    #[test]
    fn iptc_extension_rejects_non_utf8_packet() {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources =
            encode_image_resources(&[resource(XMP_METADATA, "xmp", &[0xff, 0xfe, 0x00, 0x01])])
                .into();
        let before = doc.image_resources.clone();
        assert_eq!(
            set_values_outcome(
                &mut doc,
                &[(
                    "Iptc4xmpExt:PersonInImage".to_string(),
                    vec!["Ada".to_string()],
                )]
            ),
            None
        );
        assert_eq!(doc.image_resources, before);
    }
}
