use super::*;

/// Adjustment layers authored by psd-tools: keys and payload bytes must survive
/// read, and the codec must write the same key+bytes back unchanged.
#[test]
fn adjustment_layers_preserve_key_and_bytes() {
    let doc = load("adjustment.psd");
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Base",
            "Invert",
            "Posterize",
            "Threshold",
            "BrightnessContrast",
            "Levels",
            "PhotoFilter",
        ]
    );

    let find = |n: &str| doc.layers.iter().find(|l| l.name == n).unwrap();
    let invert = find("Invert").adjustment.as_ref().expect("invert block");
    assert_eq!(invert.key, *b"nvrt");
    assert!(invert.data.is_empty(), "Invert has no payload");
    assert_eq!(
        find("Posterize").adjustment.as_ref().unwrap().data,
        [0, 4, 0, 0]
    );
    assert_eq!(
        find("Threshold").adjustment.as_ref().unwrap().data,
        [0, 128, 0, 0]
    );
    let brit = find("BrightnessContrast").adjustment.as_ref().unwrap();
    assert_eq!(brit.key, *b"brit");
    assert_eq!(brit.data, [0, 10, 0, 20, 0, 0, 0, 0]);
    let levl = find("Levels").adjustment.as_ref().unwrap();
    assert_eq!(levl.key, *b"levl");
    assert_eq!(levl.data.len(), 292);
    let phfl = find("PhotoFilter").adjustment.as_ref().unwrap();
    assert_eq!(phfl.key, *b"phfl");
    assert_eq!(
        phfl.data,
        [0, 2, 0, 0, 0, 255, 0, 180, 0, 80, 0, 0, 0, 0, 0, 25, 1, 0, 0, 0,],
        "version 2, sRGB (255,180,80), density 25, luminosity"
    );

    // Round-trip through pictura-codec: whole document, including adjustments.
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
}

/// The Gradient Map fixture: the psd-tools-authored `grdm` block survives read
/// and whole-document round-trip unchanged.
#[test]
fn gradient_map_layer_preserves_key_and_bytes() {
    let doc = load("gradient_map.psd");
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Base", "Gradient Map"]);

    let gm = doc
        .layers
        .iter()
        .find(|l| l.name == "Gradient Map")
        .and_then(|l| l.adjustment.as_ref())
        .expect("gradient map adjustment block");
    assert_eq!(gm.key, *b"grdm");
    assert_eq!(gm.data.len(), 140, "psd-tools version-1 grdm payload");
    assert_eq!(&gm.data[0..2], &[0, 1], "version 1");
    assert_eq!(gm.data[2], 0, "not reversed");
    assert_eq!(gm.data[3], 0, "not dithered");

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
}

/// The solid-color fill fixture: the psd-tools-authored `SoCo` descriptor
/// survives read and whole-document round-trip, with the expected `Clr `
/// `RGBC` doubles.
#[test]
fn solid_fill_layer_preserves_descriptor() {
    let doc = load("solid_fill.psd");
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Base", "Solid Fill"]);

    let adj = doc
        .layers
        .iter()
        .find(|l| l.name == "Solid Fill")
        .and_then(|l| l.adjustment.as_ref())
        .expect("solid fill adjustment block");
    assert_eq!(adj.key, *b"SoCo");

    let desc = read_descriptor(&adj.data).expect("version-16 descriptor");
    let DescValue::Object { items, .. } = desc else {
        panic!("top level is an object");
    };
    let clr = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Clr ")
        .map(|(_, v)| v)
        .expect("Clr item");
    let DescValue::Object {
        class_id, items, ..
    } = clr
    else {
        panic!("Clr is an object");
    };
    assert_eq!(class_id, b"RGBC");
    let get = |key: &[u8]| {
        items
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
            .expect("component")
    };
    assert_eq!(get(b"Rd  "), &DescValue::Double(10.0));
    assert_eq!(get(b"Grn "), &DescValue::Double(20.0));
    assert_eq!(get(b"Bl  "), &DescValue::Double(30.0));

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
}

/// The gradient fill fixture: the psd-tools-authored `GdFl` descriptor survives
/// read and whole-document round-trip, with the expected kind, angle, and stops.
#[test]
fn gradient_fill_layer_preserves_descriptor() {
    fn get<'a>(obj: &'a DescValue, key: &[u8]) -> Option<&'a DescValue> {
        let DescValue::Object { items, .. } = obj else {
            return None;
        };
        items
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
    }

    let doc = load("gradient_fill.psd");
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Base", "Gradient Fill"]);

    let adj = doc
        .layers
        .iter()
        .find(|l| l.name == "Gradient Fill")
        .and_then(|l| l.adjustment.as_ref())
        .expect("gradient fill adjustment block");
    assert_eq!(adj.key, *b"GdFl");

    let desc = read_descriptor(&adj.data).expect("version-16 descriptor");
    let kind = get(&desc, b"Type").expect("Type enum");
    assert_eq!(
        kind,
        &DescValue::Enum {
            kind: b"GrdT".to_vec(),
            value: b"Lnr ".to_vec(),
        }
    );
    assert_eq!(get(&desc, b"Angl"), Some(&DescValue::Double(0.0)));

    let grad = get(&desc, b"Grad").expect("Grad object");
    let DescValue::List(clrs) = get(grad, b"Clrs").expect("Clrs list") else {
        panic!("Clrs is a list");
    };
    assert_eq!(clrs.len(), 2, "two stops");
    let components: Vec<(u16, u8, u8, u8)> = clrs
        .iter()
        .map(|stop| {
            let clr = get(stop, b"Clr ").expect("Clr object");
            let c = |key: &[u8]| match get(clr, key) {
                Some(DescValue::Double(v)) => v.round() as u8,
                other => panic!("{key:?} not a double: {other:?}"),
            };
            let location = match get(stop, b"Lctn") {
                Some(DescValue::Double(v)) => v.round() as u16,
                other => panic!("Lctn not a double: {other:?}"),
            };
            (location, c(b"Rd  "), c(b"Grn "), c(b"Bl  "))
        })
        .collect();
    assert_eq!(components, [(0, 0, 0, 0), (4096, 255, 255, 255)]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
}

/// The pattern fill fixture: the psd-tools-authored `PtFl` descriptor and the
/// `Patt` tagged block's pixels survive read and whole-document round-trip.
#[test]
fn pattern_fill_layer_preserves_descriptor_and_pattern() {
    fn get<'a>(obj: &'a DescValue, key: &[u8]) -> Option<&'a DescValue> {
        let DescValue::Object { items, .. } = obj else {
            return None;
        };
        items
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
    }

    let doc = load("pattern_fill.psd");
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Base", "Pattern Fill"]);

    let adj = doc
        .layers
        .iter()
        .find(|l| l.name == "Pattern Fill")
        .and_then(|l| l.adjustment.as_ref())
        .expect("pattern fill adjustment block");
    assert_eq!(
        adj.key, *b"PtFl",
        "the PtFl block is routed to layer.adjustment"
    );

    let desc = read_descriptor(&adj.data).expect("version-16 descriptor");
    let ptrn = get(&desc, b"Ptrn").expect("Ptrn object");
    assert_eq!(
        get(ptrn, b"Idnt"),
        Some(&DescValue::Text("pictura-pattern\0".into())),
        "the descriptor references the pattern by id"
    );

    let patterns = pictura_codec::decode_patterns(&doc);
    assert_eq!(patterns.len(), 1, "one Patt pattern");
    assert_eq!(patterns[0].pattern_id, "pictura-pattern");
    assert_eq!((patterns[0].width, patterns[0].height), (2, 2));
    assert_eq!(
        patterns[0].rgba,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,],
        "row-major red, green / blue, white"
    );

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc, "the whole document round-trips");
    assert_eq!(
        pictura_codec::decode_patterns(&back),
        patterns,
        "the Patt pixels survive a round-trip"
    );
}

/// The masked pattern fill fixture: Photoshop's 24 colour slots decode to the
/// authored 2x2 RGB pattern, and the fill layer's painted mask is attached.
#[test]
fn pattern_fill_masked_fixture_decodes_pattern_and_mask() {
    let doc = load("pattern_fill_masked.psd");
    let fill = doc
        .layers
        .iter()
        .find(|l| l.name == "Pattern Fill")
        .expect("pattern fill layer");
    let adj = fill.adjustment.as_ref().expect("PtFl block");
    assert_eq!(adj.key, *b"PtFl");

    let patterns = pictura_codec::decode_patterns(&doc);
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_id, "pictura-pattern-photoshop");
    assert_eq!((patterns[0].width, patterns[0].height), (2, 2));
    assert_eq!(
        patterns[0].rgba,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255],
        "the three written colour slots and the trailing alpha"
    );

    let mask = fill.mask.as_ref().expect("layer mask");
    assert_eq!((mask.rect.width(), mask.rect.height()), (8, 8));
    let data = mask.data.as_ref().expect("mask data");
    assert_eq!(data.len(), 64);
    assert_eq!(data.sample(0), Some(255), "top-left is revealed");
    assert_eq!(data.sample(7 * 8), Some(0), "bottom-left is hidden");

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc, "the pattern and mask round-trip");
}

/// The 16-bit pattern fixture: the `PtFl` block is still recognised, but the
/// 16-bit pattern planes are skipped (not misdecoded as 8-bit pixels), and the
/// document still round-trips.
#[test]
fn pattern_fill_16bit_pattern_is_skipped() {
    let doc = load("pattern_fill_16bit.psd");
    let adj = doc
        .layers
        .iter()
        .find(|l| l.name == "Pattern Fill 16")
        .and_then(|l| l.adjustment.as_ref())
        .expect("pattern fill adjustment block");
    assert_eq!(adj.key, *b"PtFl");
    assert!(
        pictura_codec::decode_patterns(&doc).is_empty(),
        "16-bit pattern planes are skipped"
    );

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc, "the whole document round-trips");
}
