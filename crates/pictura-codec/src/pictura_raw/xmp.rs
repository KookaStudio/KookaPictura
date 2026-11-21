//! The Pictura Raw settings as Camera Raw `crs:` XMP, for a raw file's settings
//! record (a sidecar `.xmp`, a database record, or a DNG's embedded packet).
//!
//! Each `crs:` property maps to the `Fltr` key the settings codec already uses,
//! so reading builds `Fltr` items for `decode_items` and writing renders
//! `encode_items`: the two vocabularies share one value-type map.
//!
//! Writing patches an existing packet one property at a time and keeps every
//! other byte. ponytail: only the literal `crs:` prefix is matched, and the
//! `crs` description ends at the first `</rdf:Description>` after its start
//! tag, so a description nested inside it would misplace inserted curves.

use pictura_core::{PicturaRawSettings, HSL_RANGES};
use sha2::{Digest, Sha256};

use crate::descriptor::{get_object_item, DescValue};
use crate::smart_object::find_subslice;

use super::codec::{decode_items, encode_items};

#[derive(Clone, Copy)]
enum Form {
    /// A number written with `decimals` places, `+` on positive values when
    /// `signed`.
    Number {
        decimals: usize,
        signed: bool,
    },
    Bool,
    Text,
    WhiteBalance,
    ProcessVersion,
    Curve,
}

const INT: Form = Form::Number {
    decimals: 0,
    signed: false,
};
const SIGNED: Form = Form::Number {
    decimals: 0,
    signed: true,
};
const TENTHS: Form = Form::Number {
    decimals: 1,
    signed: true,
};

/// `(Fltr key, crs name, form)` for every control outside the HSL ranges.
const CRS_KEYS: &[(&[u8; 4], &str, Form)] = &[
    (b"Temp", "Temperature", INT),
    (b"Tint", "Tint", SIGNED),
    (
        b"Ex12",
        "Exposure2012",
        Form::Number {
            decimals: 2,
            signed: true,
        },
    ),
    (b"Cr12", "Contrast2012", SIGNED),
    (b"Hi12", "Highlights2012", SIGNED),
    (b"Sh12", "Shadows2012", SIGNED),
    (b"Wh12", "Whites2012", SIGNED),
    (b"Bk12", "Blacks2012", SIGNED),
    (b"Cl12", "Clarity2012", SIGNED),
    (b"Vibr", "Vibrance", SIGNED),
    (b"Strt", "Saturation", SIGNED),
    (b"WBal", "WhiteBalance", Form::WhiteBalance),
    (b"AWBV", "AutoWhiteVersion", INT),
    (b"PrVe", "ProcessVersion", Form::ProcessVersion),
    (b"CtoG", "ConvertToGrayscale", Form::Bool),
    (b"PC_S", "ParametricShadows", SIGNED),
    (b"PC_D", "ParametricDarks", SIGNED),
    (b"PC_L", "ParametricLights", SIGNED),
    (b"PC_H", "ParametricHighlights", SIGNED),
    (b"PC_1", "ParametricShadowSplit", INT),
    (b"PC_2", "ParametricMidtoneSplit", INT),
    (b"PC_3", "ParametricHighlightSplit", INT),
    (b"Crv ", "ToneCurvePV2012", Form::Curve),
    (b"CrvR", "ToneCurvePV2012Red", Form::Curve),
    (b"CrvG", "ToneCurvePV2012Green", Form::Curve),
    (b"CrvB", "ToneCurvePV2012Blue", Form::Curve),
    (b"Shrp", "Sharpness", INT),
    (b"ShpR", "SharpenRadius", TENTHS),
    (b"ShpD", "SharpenDetail", INT),
    (b"ShpM", "SharpenEdgeMasking", INT),
    (b"LNR ", "LuminanceSmoothing", INT),
    (b"LNRD", "LuminanceNoiseReductionDetail", INT),
    (b"LNRC", "LuminanceNoiseReductionContrast", INT),
    (b"CNR ", "ColorNoiseReduction", INT),
    (b"CNRD", "ColorNoiseReductionDetail", INT),
    (b"CNRS", "ColorNoiseReductionSmoothness", INT),
    (b"STSH", "SplitToningShadowHue", INT),
    (b"STSS", "SplitToningShadowSaturation", INT),
    (b"STHH", "SplitToningHighlightHue", INT),
    (b"STHS", "SplitToningHighlightSaturation", INT),
    (b"STB ", "SplitToningBalance", SIGNED),
    (b"LPEn", "LensProfileEnable", INT),
    (b"MDis", "LensManualDistortionAmount", SIGNED),
    (b"VigA", "VignetteAmount", SIGNED),
    (b"VigM", "VignetteMidpoint", SIGNED),
    (b"PerV", "PerspectiveVertical", SIGNED),
    (b"PerH", "PerspectiveHorizontal", SIGNED),
    (b"PerS", "PerspectiveScale", INT),
    (b"PerA", "PerspectiveAspect", SIGNED),
    (b"PerU", "PerspectiveUpright", INT),
    (b"PerR", "PerspectiveRotate", TENTHS),
    (b"PerX", "PerspectiveX", TENTHS),
    (b"PerY", "PerspectiveY", TENTHS),
    (b"DfPA", "DefringePurpleAmount", INT),
    (b"DPHL", "DefringePurpleHueLo", INT),
    (b"DPPH", "DefringePurpleHueHi", INT),
    (b"DfGA", "DefringeGreenAmount", INT),
    (b"DPGL", "DefringeGreenHueLo", INT),
    (b"DPGH", "DefringeGreenHueHi", INT),
    (b"GRNA", "GrainAmount", INT),
    (b"GRNS", "GrainSize", INT),
    (b"GRNF", "GrainFrequency", INT),
    (b"Dhze", "Dehaze", SIGNED),
    (b"PCVA", "PostCropVignetteAmount", SIGNED),
    (b"PCVM", "PostCropVignetteMidpoint", INT),
    (b"PCVF", "PostCropVignetteFeather", INT),
    (b"PCVR", "PostCropVignetteRoundness", SIGNED),
    (b"PCVS", "PostCropVignetteStyle", INT),
    (b"PCVH", "PostCropVignetteHighlightContrast", INT),
    (b"CamP", "CameraProfile", Form::Text),
    (b"CP_D", "CameraProfileDigest", Form::Text),
    (b"RHue", "RedHue", SIGNED),
    (b"RSat", "RedSaturation", SIGNED),
    (b"GHue", "GreenHue", SIGNED),
    (b"GSat", "GreenSaturation", SIGNED),
    (b"BHue", "BlueHue", SIGNED),
    (b"BSat", "BlueSaturation", SIGNED),
    (b"BlkB", "ShadowTint", SIGNED),
];

/// The HSL range prefixes and their `crs:` stems.
const HSL_KEYS: [(&[u8; 3], &str); 4] = [
    (b"HA_", "HueAdjustment"),
    (b"SA_", "SaturationAdjustment"),
    (b"LA_", "LuminanceAdjustment"),
    (b"GM_", "GrayMixer"),
];

/// The colour names of [`HSL_RANGES`], in the same order.
const HSL_NAMES: [&str; 8] = [
    "Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta",
];

/// The white-balance labels and the dialog's preset codes.
const WHITE_BALANCE: [(&str, &[u8; 4]); 9] = [
    ("As Shot", b"AsSh"),
    ("Auto", b"Auto"),
    ("Daylight", b"Dayl"),
    ("Cloudy", b"Clou"),
    ("Shade", b"Shad"),
    ("Tungsten", b"Tung"),
    ("Fluorescent", b"Fluo"),
    ("Flash", b"Flsh"),
    ("Custom", b"Cst "),
];

const FRESH_PACKET: &str = "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Kooka Pictura\">\n \
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
<rdf:Description rdf:about=\"\"\n    \
xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"\n   \
crs:Version=\"7.0\"/>\n \
</rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>";

const CRS_DESCRIPTION: &str = "\n  <rdf:Description rdf:about=\"\"\n    \
xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"/>\n ";

/// Every `(Fltr key, crs name, form)`, the HSL ranges expanded.
fn all_keys() -> Vec<(Vec<u8>, String, Form)> {
    let mut keys: Vec<(Vec<u8>, String, Form)> = CRS_KEYS
        .iter()
        .map(|(key, name, form)| (key.to_vec(), name.to_string(), *form))
        .collect();
    for (prefix, stem) in HSL_KEYS {
        for (code, colour) in HSL_RANGES.iter().zip(HSL_NAMES) {
            let key = vec![prefix[0], prefix[1], prefix[2], *code];
            keys.push((key, format!("{stem}{colour}"), SIGNED));
        }
    }
    keys
}

/// The lowercase hex SHA-256 of a raw file's bytes: its settings-database key.
pub fn raw_settings_key(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Every modelled control a `crs:` packet sets; a missing or unparsable
/// property leaves its control unset.
pub fn pictura_raw_from_xmp(packet: &[u8]) -> PicturaRawSettings {
    let items: Vec<(Vec<u8>, DescValue)> = all_keys()
        .into_iter()
        .filter_map(|(key, name, form)| {
            let prop = find_prop(packet, &name)?;
            Some((key, read_value(&packet[prop.inner.0..prop.inner.1], form)?))
        })
        .collect();
    decode_items(&items)
}

/// `settings` written into `existing` (or a fresh packet): each modelled
/// property replaced, inserted, or (when unset) removed, every other byte
/// kept, and `crs:HasSettings="True"`. `None` when `existing` has no
/// `rdf:RDF` element to patch.
pub fn pictura_raw_to_xmp(
    existing: Option<&[u8]>,
    settings: &PicturaRawSettings,
) -> Option<Vec<u8>> {
    let mut packet = existing.unwrap_or(FRESH_PACKET.as_bytes()).to_vec();
    if description_start(&packet).is_none() {
        let at = find_subslice(&packet, b"</rdf:RDF>")?;
        packet.splice(at..at, CRS_DESCRIPTION.bytes());
    }
    let items = encode_items(settings);
    for (key, name, form) in all_keys() {
        let value = get_object_item(&items, &key);
        packet = set_prop(packet, &name, form, value)?;
    }
    set_prop(
        packet,
        "HasSettings",
        Form::Bool,
        Some(&DescValue::Bool(true)),
    )
}

/// A property's whole extent (with its leading whitespace) and its value text.
struct Prop {
    whole: (usize, usize),
    inner: (usize, usize),
    element: bool,
}

fn find_prop(packet: &[u8], name: &str) -> Option<Prop> {
    let needle = format!("crs:{name}");
    let needle = needle.as_bytes();
    let mut from = 0;
    while let Some(rel) = find_subslice(&packet[from..], needle) {
        let at = from + rel;
        let after = at + needle.len();
        from = after;
        if packet.get(after).copied().is_none_or(is_name_byte) || at == 0 {
            continue;
        }
        let before = packet[at - 1];
        let lead = leading_ws(packet, if before == b'<' { at - 1 } else { at });
        if before == b'<' {
            let gt = tag_end(packet, after)?;
            if packet[gt - 1] == b'/' {
                return Some(Prop {
                    whole: (lead, gt + 1),
                    inner: (gt, gt),
                    element: true,
                });
            }
            let close = format!("</crs:{name}>");
            let end = find_subslice(&packet[gt + 1..], close.as_bytes())? + gt + 1;
            return Some(Prop {
                whole: (lead, end + close.len()),
                inner: (gt + 1, end),
                element: true,
            });
        }
        if !is_ws(before) {
            continue;
        }
        let i = skip_ws(packet, after);
        if packet.get(i) != Some(&b'=') {
            continue;
        }
        let i = skip_ws(packet, i + 1);
        let quote = *packet.get(i)?;
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        let end = packet[i + 1..].iter().position(|&b| b == quote)? + i + 1;
        return Some(Prop {
            whole: (lead, end + 1),
            inner: (i + 1, end),
            element: false,
        });
    }
    None
}

/// Set, keep, or remove one property. An existing value that already reads as
/// `value` is left byte-for-byte.
fn set_prop(
    mut packet: Vec<u8>,
    name: &str,
    form: Form,
    value: Option<&DescValue>,
) -> Option<Vec<u8>> {
    let existing = find_prop(&packet, name);
    let Some(value) = value else {
        if let Some(prop) = existing {
            packet.drain(prop.whole.0..prop.whole.1);
        }
        return Some(packet);
    };
    if let Some(prop) = &existing {
        let current = read_value(&packet[prop.inner.0..prop.inner.1], form);
        if current.is_some_and(|current| same_value(&current, value)) {
            return Some(packet);
        }
    }
    if let Form::Curve = form {
        let element = curve_element(name, value);
        if let Some(prop) = existing {
            packet.splice(prop.whole.0..prop.whole.1, element.bytes());
            return Some(packet);
        }
        return insert_element(packet, &element);
    }
    let text = escape(&write_value(value, form)?);
    match existing {
        // An empty element `<crs:Name/>` has no text to replace.
        Some(prop) if prop.element && packet[prop.inner.0] == b'>' => {
            let element = format!("<crs:{name}>{text}</crs:{name}>");
            let lt = prop.whole.0 + skip_ws(&packet[prop.whole.0..], 0);
            packet.splice(lt..prop.whole.1, element.bytes());
            Some(packet)
        }
        Some(prop) => {
            packet.splice(prop.inner.0..prop.inner.1, text.bytes());
            Some(packet)
        }
        None => {
            let (start, gt) = description_start(&packet)?;
            let mut k = gt;
            while k > start && is_ws(packet[k - 1]) {
                k -= 1;
            }
            let at = if packet[k - 1] == b'/' { k - 1 } else { gt };
            let attribute = format!("\n   crs:{name}=\"{text}\"");
            packet.splice(at..at, attribute.bytes());
            Some(packet)
        }
    }
}

fn insert_element(mut packet: Vec<u8>, element: &str) -> Option<Vec<u8>> {
    let (start, gt) = description_start(&packet)?;
    if packet[gt - 1] == b'/' {
        let opened = format!(">{element}\n  </rdf:Description>");
        packet.splice(gt - 1..gt + 1, opened.bytes());
        return Some(packet);
    }
    let close = find_subslice(&packet[gt..], b"</rdf:Description>")? + gt;
    let mut at = close;
    while at > start && is_ws(packet[at - 1]) {
        at -= 1;
    }
    packet.splice(at..at, element.bytes());
    Some(packet)
}

/// The start and `>` of the first `rdf:Description` start tag that carries
/// the `crs` namespace or a `crs:` attribute.
fn description_start(packet: &[u8]) -> Option<(usize, usize)> {
    let open = b"<rdf:Description";
    let mut from = 0;
    while let Some(rel) = find_subslice(&packet[from..], open) {
        let at = from + rel;
        let gt = tag_end(packet, at + open.len())?;
        let tag = &packet[at..gt];
        let crs_attribute = tag.windows(5).any(|w| is_ws(w[0]) && &w[1..] == b"crs:");
        if crs_attribute || find_subslice(tag, b"xmlns:crs").is_some() {
            return Some((at, gt));
        }
        from = gt;
    }
    None
}

fn curve_element(name: &str, value: &DescValue) -> String {
    let mut out = format!("\n   <crs:{name}>\n    <rdf:Seq>");
    if let DescValue::List(list) = value {
        let points: Vec<i64> = list
            .iter()
            .filter_map(|v| match v {
                DescValue::Long(v) => Some(i64::from(*v)),
                _ => None,
            })
            .collect();
        for [x, y] in points.as_chunks::<2>().0 {
            out.push_str(&format!("\n     <rdf:li>{x}, {y}</rdf:li>"));
        }
    }
    out.push_str(&format!("\n    </rdf:Seq>\n   </crs:{name}>"));
    out
}

fn read_value(raw: &[u8], form: Form) -> Option<DescValue> {
    if let Form::Curve = form {
        return read_curve(raw);
    }
    let text = unescape(std::str::from_utf8(raw).ok()?);
    let text = text.trim();
    match form {
        Form::Number { .. } => {
            let v: f64 = text.strip_prefix('+').unwrap_or(text).parse().ok()?;
            v.is_finite().then_some(DescValue::Double(v))
        }
        Form::Bool => match text {
            "True" | "true" => Some(DescValue::Bool(true)),
            "False" | "false" => Some(DescValue::Bool(false)),
            _ => None,
        },
        Form::Text => Some(DescValue::Text(text.to_string())),
        Form::WhiteBalance => {
            let (_, code) = WHITE_BALANCE.iter().find(|(label, _)| *label == text)?;
            Some(DescValue::Enum {
                kind: b"WBal".to_vec(),
                value: code.to_vec(),
            })
        }
        Form::ProcessVersion => {
            let v: f64 = text.parse().ok()?;
            let year = if v >= 6.7 {
                2012
            } else if v >= 5.7 {
                2010
            } else {
                2003
            };
            Some(DescValue::Long(year))
        }
        Form::Curve => None,
    }
}

fn read_curve(raw: &[u8]) -> Option<DescValue> {
    let mut points = Vec::new();
    let mut from = 0;
    while let Some(rel) = find_subslice(&raw[from..], b"<rdf:li>") {
        let start = from + rel + b"<rdf:li>".len();
        let end = find_subslice(&raw[start..], b"</rdf:li>")? + start;
        let item = std::str::from_utf8(&raw[start..end]).ok()?;
        let (x, y) = item.split_once(',')?;
        points.push(DescValue::Long(x.trim().parse().ok()?));
        points.push(DescValue::Long(y.trim().parse().ok()?));
        from = end;
    }
    (!points.is_empty()).then_some(DescValue::List(points))
}

fn write_value(value: &DescValue, form: Form) -> Option<String> {
    let number = match value {
        DescValue::Long(v) => Some(f64::from(*v)),
        DescValue::Double(v) => Some(*v),
        _ => None,
    };
    match form {
        Form::Number { decimals, signed } => {
            let text = format!("{:.decimals$}", number?);
            let shown: f64 = text.parse().ok()?;
            // `-0.2` at no decimals prints `-0`; write it as `0`.
            let text = if shown == 0.0 {
                format!("{:.decimals$}", 0.0)
            } else {
                text
            };
            Some(if signed && shown > 0.0 {
                format!("+{text}")
            } else {
                text
            })
        }
        Form::Bool => match value {
            DescValue::Bool(true) => Some("True".into()),
            DescValue::Bool(false) => Some("False".into()),
            _ => None,
        },
        Form::Text => match value {
            DescValue::Text(text) => Some(text.trim_end_matches('\0').to_string()),
            _ => None,
        },
        Form::WhiteBalance => match value {
            DescValue::Enum { value, .. } => WHITE_BALANCE
                .iter()
                .find(|(_, code)| code.as_slice() == value.as_slice())
                .map(|(label, _)| label.to_string()),
            _ => None,
        },
        Form::ProcessVersion => Some(
            match number? as i32 {
                2003 => "5.0",
                2010 => "5.7",
                _ => "6.7",
            }
            .to_string(),
        ),
        Form::Curve => None,
    }
}

/// Whether an existing value already says what `wanted` says.
fn same_value(current: &DescValue, wanted: &DescValue) -> bool {
    let number = |v: &DescValue| match v {
        DescValue::Long(v) => Some(f64::from(*v)),
        DescValue::Double(v) => Some(*v),
        _ => None,
    };
    match (number(current), number(wanted)) {
        (Some(a), Some(b)) => (a - b).abs() < 1e-9,
        _ => match (current, wanted) {
            (DescValue::Text(a), DescValue::Text(b)) => a == b.trim_end_matches('\0'),
            _ => current == wanted,
        },
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(semi) = rest.find(';') else {
            break;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16).ok())
                .unwrap_or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The index of the `>` closing the start tag that continues at `from`,
/// skipping any `>` inside quoted attribute values.
fn tag_end(packet: &[u8], from: usize) -> Option<usize> {
    let mut quote = 0u8;
    for (i, &b) in packet.iter().enumerate().skip(from) {
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'>' {
            return Some(i);
        }
    }
    None
}

fn leading_ws(packet: &[u8], mut at: usize) -> usize {
    while at > 0 && is_ws(packet[at - 1]) {
        at -= 1;
    }
    at
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn skip_ws(packet: &[u8], mut i: usize) -> usize {
    while packet.get(i).copied().is_some_and(is_ws) {
        i += 1;
    }
    i
}
