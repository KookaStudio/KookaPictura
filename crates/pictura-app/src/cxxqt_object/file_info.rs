//! File Info bridge rows, edits, and raw-packet apply.

use super::qobject;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

impl qobject::PictureView {
    pub fn exif_rows(&self) -> QStringList {
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        let metadata = pictura_codec::read_metadata(doc);
        let mut rows: Vec<String> = Vec::new();
        for (tag, value) in metadata.exif.entries() {
            let label = pictura_codec::exif_tag_name(*tag)
                .map(str::to_string)
                .unwrap_or_else(|| format!("Tag 0x{tag:04X}"));
            rows.push(format!("Camera Data\t{label}\t{}", value.display()));
        }
        if let Some(gps) = metadata.exif.gps() {
            if let Some(value) = gps.latitude {
                rows.push(format!("GPS Data\tLatitude\t{value:.6}"));
            }
            if let Some(value) = gps.longitude {
                rows.push(format!("GPS Data\tLongitude\t{value:.6}"));
            }
            if let Some(value) = gps.altitude {
                rows.push(format!("GPS Data\tAltitude\t{value:.1} m"));
            }
            if let Some(value) = &gps.timestamp {
                rows.push(format!("GPS Data\tTimestamp\t{value}"));
            }
            if let Some(value) = &gps.date {
                rows.push(format!("GPS Data\tDate\t{value}"));
            }
            if let Some(value) = &gps.datum {
                rows.push(format!("GPS Data\tDatum\t{value}"));
            }
        }
        rows.into_iter().map(QString::from).collect()
    }

    pub fn iptc_rows(&self) -> QStringList {
        const EDITABLE: &[(u8, u8)] = &[
            (2, 5),
            (2, 25),
            (2, 40),
            (2, 80),
            (2, 85),
            (2, 90),
            (2, 95),
            (2, 101),
            (2, 105),
            (2, 110),
            (2, 115),
            (2, 116),
            (2, 120),
            (2, 122),
        ];
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        let metadata = pictura_codec::read_metadata(doc);
        metadata
            .iptc
            .records()
            .iter()
            .filter(|(record, dataset, _)| !EDITABLE.contains(&(*record, *dataset)))
            .map(|(record, dataset, value)| {
                let label = pictura_codec::iptc_field_name(*record, *dataset)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{record}:{dataset}"));
                QString::from(format!("IPTC\t{label}\t{}", String::from_utf8_lossy(value)))
            })
            .collect()
    }

    pub fn xmp_packet(&self) -> QString {
        self.rust()
            .session
            .doc
            .as_ref()
            .map(|doc| QString::from(pictura_codec::read_metadata(doc).xmp))
            .unwrap_or_default()
    }

    /// Read-only XMP rows outside the editable sets: the Photoshop tab and the
    /// unsupported IPTC Extension leaves.
    pub fn xmp_rows(&self) -> QStringList {
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        let mut rows: Vec<String> = Vec::new();
        for (label, value) in pictura_codec::photoshop_rows(doc) {
            rows.push(format!("Photoshop\t{label}\t{value}"));
        }
        for (label, value) in pictura_codec::iptc_extension::read_rows(doc) {
            rows.push(format!("IPTC Extension\t{label}\t{value}"));
        }
        rows.into_iter().map(QString::from).collect()
    }

    /// Every editable File Info field as `"Section\tid\tLabel\tValue"`.
    /// A supported field is emitted even when its value is empty.
    pub fn file_info_edit_fields(&self) -> QStringList {
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        let metadata = pictura_codec::read_metadata(doc);
        let props = pictura_codec::xmp_properties(doc);
        let mut rows: Vec<String> = Vec::new();
        let mut push = |section: &str, id: &str, label: &str, value: String| {
            rows.push(format!("{section}\t{id}\t{label}\t{value}"));
        };
        let xmp_value = |record: u8, dataset: u8| -> String {
            match (record, dataset) {
                (2, 5) => props.title.clone(),
                (2, 80) => props.creator.first().cloned(),
                (2, 85) => props.authors_position.clone(),
                (2, 116) => props.rights.clone(),
                (2, 120) => props.description.clone(),
                (2, 110) => props.credit.clone(),
                (2, 115) => props.source.clone(),
                (2, 105) => props.headline.clone(),
                (2, 40) => props.instructions.clone(),
                (2, 90) => props.city.clone(),
                (2, 95) => props.state.clone(),
                (2, 101) => props.country.clone(),
                (2, 122) => props.caption_writer.clone(),
                _ => None,
            }
            .unwrap_or_default()
        };
        let iim = |record: u8, dataset: u8| {
            let value = metadata.iptc.text(record, dataset).unwrap_or_default();
            if value.is_empty() {
                xmp_value(record, dataset)
            } else {
                value
            }
        };
        push("Description", "2:5", "Document Title", iim(2, 5));
        push("Description", "2:80", "Author", iim(2, 80));
        push("Description", "2:85", "Author Title", iim(2, 85));
        push("Description", "2:120", "Description", iim(2, 120));
        push("Description", "2:122", "Description Writer", iim(2, 122));
        let keywords: Vec<String> = {
            let values: Vec<String> = metadata
                .iptc
                .values(2, 25)
                .map(|value| String::from_utf8_lossy(value).into_owned())
                .collect();
            if values.is_empty() {
                props.subject.clone()
            } else {
                values
            }
        };
        if keywords.is_empty() {
            push("Description", "2:25", "Keywords", String::new());
        } else {
            for keyword in keywords {
                push("Description", "2:25", "Keywords", keyword);
            }
        }
        let copyright_status = match props.marked {
            Some(true) => "Copyrighted",
            Some(false) => "Public Domain",
            None => "",
        };
        push(
            "Description",
            "xmpRights:Marked",
            "Copyright Status",
            copyright_status.to_string(),
        );
        push("Description", "2:116", "Copyright Notice", iim(2, 116));
        push(
            "Description",
            "xmpRights:WebStatement",
            "Copyright Info URL",
            props.web_statement.clone().unwrap_or_default(),
        );

        push("IPTC", "2:5", "Object Name", iim(2, 5));
        push("IPTC", "2:80", "By-line", iim(2, 80));
        push("IPTC", "2:116", "Copyright Notice", iim(2, 116));
        push("IPTC", "2:120", "Caption/Abstract", iim(2, 120));
        push("IPTC", "2:110", "Credit", iim(2, 110));
        push("IPTC", "2:115", "Source", iim(2, 115));

        push("Origin", "2:90", "City", iim(2, 90));
        push("Origin", "2:95", "State/Province", iim(2, 95));
        push("Origin", "2:101", "Country", iim(2, 101));
        push("Origin", "2:105", "Headline", iim(2, 105));
        push("Origin", "2:110", "Credit", iim(2, 110));
        push("Origin", "2:115", "Source", iim(2, 115));
        push("Origin", "2:40", "Instructions", iim(2, 40));

        for (id, label, value) in pictura_codec::iptc_extension::edit_rows(doc) {
            push("IPTC Extension", &id, &label, value);
        }
        push(
            "IPTC Extension",
            "xmpRights:UsageTerms",
            "Rights Usage Terms",
            props.usage_terms.clone().unwrap_or_default(),
        );

        rows.into_iter().map(QString::from).collect()
    }

    /// Apply `"id\tValue"` edits and a raw packet row as one undo state.
    /// Returns whether anything changed.
    pub fn apply_metadata_edits(mut self: Pin<&mut Self>, edits: &QStringList) -> bool {
        let mut raw: Option<String> = None;
        let mut shared: Vec<(u8, u8, String)> = Vec::new();
        let mut keywords_present = false;
        let mut keywords: Vec<String> = Vec::new();
        let mut xmp: Vec<(pictura_codec::XmpField, Vec<String>)> = Vec::new();
        let mut extension: Vec<(String, Vec<String>)> = Vec::new();

        for row in edits.iter() {
            let row = row.to_string();
            let Some((id, value)) = row.split_once('\t') else {
                continue;
            };
            if id == "xmp-packet" {
                raw = Some(value.to_string());
                continue;
            }
            if id == "2:25" {
                keywords_present = true;
                if !value.is_empty() {
                    keywords.push(value.to_string());
                }
                continue;
            }
            if let Some((record, dataset)) = id.split_once(':') {
                if let (Ok(record), Ok(dataset)) = (record.parse::<u8>(), dataset.parse::<u8>()) {
                    shared.push((record, dataset, value.to_string()));
                    continue;
                }
            }
            match id {
                "xmpRights:Marked" => {
                    let value = match value {
                        "Copyrighted" => "true",
                        "Public Domain" => "false",
                        _ => "",
                    };
                    xmp.push((pictura_codec::XmpField::Marked, vec![value.to_string()]));
                }
                "xmpRights:WebStatement" => xmp.push((
                    pictura_codec::XmpField::WebStatement,
                    vec![value.to_string()],
                )),
                "xmpRights:UsageTerms" => {
                    xmp.push((pictura_codec::XmpField::UsageTerms, vec![value.to_string()]))
                }
                _ if id.starts_with("Iptc4xmpExt:") => {
                    match extension.iter_mut().find(|(existing, _)| existing == id) {
                        Some((_, values)) => values.push(value.to_string()),
                        None => extension.push((id.to_string(), vec![value.to_string()])),
                    }
                }
                _ => return false,
            }
        }

        let mut changed = false;
        let mut refused = false;
        if let Some(packet) = raw {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            if !pictura_codec::set_xmp_packet(doc, &packet) {
                return false;
            }
            changed = true;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            if !shared.is_empty() {
                changed |= pictura_codec::set_file_info_fields(doc, &shared);
            }
            if keywords_present {
                changed |= pictura_codec::set_file_info_values(doc, &[(2, 25, keywords.clone())]);
            }
            if !xmp.is_empty() {
                changed |= pictura_codec::set_xmp_values(doc, &xmp);
            }
            if !extension.is_empty() {
                match pictura_codec::iptc_extension::set_values_outcome(doc, &extension) {
                    Some(true) => changed = true,
                    Some(false) => {}
                    None => refused = true,
                }
            }
        }
        if changed {
            self.as_mut().edit("File Info", |_| Edit::Meta);
        }
        if refused {
            return false;
        }
        changed
    }
}
