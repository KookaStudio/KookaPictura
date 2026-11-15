use super::*;

const HEAD: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">";
const TAIL: &str = "</rdf:RDF></x:xmpmeta>";

fn packet(description: &str) -> String {
    format!("{HEAD}{description}{TAIL}")
}

fn element_packet() -> String {
    packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Title</rdf:li></rdf:Alt></dc:title>\
         <photoshop:Credit>Old Credit</photoshop:Credit>\
         <acme:Note xmlns:acme=\"http://example.com/acme/\">keep me</acme:Note>\
         </rdf:Description>",
    )
}

#[test]
fn decodes_attribute_and_element_forms() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         dc:title=\"Attr Title\" photoshop:Credit=\"Cred\">\
         <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">Desc</rdf:li></rdf:Alt></dc:description>\
         <dc:creator><rdf:Seq><rdf:li>Ada</rdf:li></rdf:Seq></dc:creator>\
         <dc:subject><rdf:Bag><rdf:li>one</rdf:li><rdf:li>two</rdf:li></rdf:Bag></dc:subject>\
         </rdf:Description>",
    );
    let props = parse_xmp(&packet);
    assert_eq!(props.title.as_deref(), Some("Attr Title"));
    assert_eq!(props.credit.as_deref(), Some("Cred"));
    assert_eq!(props.description.as_deref(), Some("Desc"));
    assert_eq!(props.creator, vec!["Ada"]);
    assert_eq!(props.subject, vec!["one", "two"]);
}

#[test]
fn resolves_an_odd_prefix_by_namespace() {
    let packet = packet(
        "<rdf:Description xmlns:foo=\"http://purl.org/dc/elements/1.1/\">\
         <foo:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Weird</rdf:li></rdf:Alt></foo:title>\
         </rdf:Description>",
    );
    assert_eq!(parse_xmp(&packet).title.as_deref(), Some("Weird"));
    let edited = patch_xmp(&packet, &[(XmpField::Title, "New".into())]).unwrap();
    assert!(edited.contains("foo:title"), "the odd prefix is reused");
    assert_eq!(parse_xmp(&edited).title.as_deref(), Some("New"));
}

#[test]
fn edit_preserves_unknown_namespace_bytes() {
    let packet = element_packet();
    let unknown = "<acme:Note xmlns:acme=\"http://example.com/acme/\">keep me</acme:Note>";
    assert!(packet.contains(unknown));
    let edited = patch_xmp(
        &packet,
        &[
            (XmpField::Title, "New Title".into()),
            (XmpField::Credit, "New Credit".into()),
        ],
    )
    .expect("recognised packet patches");
    assert!(
        edited.contains(unknown),
        "unknown namespace is byte-identical"
    );
    assert_eq!(parse_xmp(&edited).title.as_deref(), Some("New Title"));
    assert_eq!(parse_xmp(&edited).credit.as_deref(), Some("New Credit"));
}

#[test]
fn attribute_form_is_patched_in_place() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:acme=\"http://example.com/acme/\" acme:Custom=\"keep\" dc:title=\"Old\"/>",
    );
    let edited = patch_xmp(&packet, &[(XmpField::Title, "New".into())]).unwrap();
    assert!(
        edited.contains("acme:Custom=\"keep\""),
        "unknown attr survives"
    );
    assert!(edited.contains("dc:title=\"New\""));
    assert_eq!(parse_xmp(&edited).title.as_deref(), Some("New"));
}

#[test]
fn unrecognised_packet_is_none() {
    assert!(patch_xmp("<not-xmp/>", &[(XmpField::Title, "x".into())]).is_none());
    assert!(patch_xmp("", &[(XmpField::Title, "x".into())]).is_none());
    assert!(patch_xmp("hello", &[(XmpField::Title, "x".into())]).is_none());
}

#[test]
fn setting_an_equal_value_is_a_noop() {
    let packet = element_packet();
    let out = patch_xmp(&packet, &[(XmpField::Title, "Old Title".into())]).unwrap();
    assert_eq!(out, packet);
}

#[test]
fn entity_payload_is_not_expanded() {
    let packet = packet(
        "<?xml version=\"1.0\"?>\
         <!DOCTYPE foo [ <!ENTITY xxe \"x\"> ]>\
         <rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" dc:title=\"&xxe;\"/>",
    );
    let props = parse_xmp(&packet);
    assert_ne!(
        props.title.as_deref(),
        Some("x"),
        "the entity is not expanded"
    );
    assert!(patch_xmp(&packet, &[(XmpField::Title, "y".into())]).is_none());
}

#[test]
fn the_five_predefined_entities_are_decoded_and_re_escaped() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">A &amp; B &lt; C</rdf:li></rdf:Alt></dc:title>\
         </rdf:Description>",
    );
    assert_eq!(parse_xmp(&packet).title.as_deref(), Some("A & B < C"));
    let edited = patch_xmp(&packet, &[(XmpField::Title, "A & B < C".into())]).unwrap();
    assert_eq!(edited, packet, "the equal value is a no-op");
    let changed = patch_xmp(&packet, &[(XmpField::Title, "x & <y>".into())]).unwrap();
    assert!(changed.contains("x &amp; &lt;y&gt;"));
    assert_eq!(parse_xmp(&changed).title.as_deref(), Some("x & <y>"));
}

#[test]
fn marked_accepts_true_and_false() {
    let packet = packet(
        "<rdf:Description xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <xmpRights:Marked>True</xmpRights:Marked></rdf:Description>",
    );
    assert_eq!(parse_xmp(&packet).marked, Some(true));
    let edited = patch_xmp(&packet, &[(XmpField::Marked, "false".into())]).unwrap();
    assert!(edited.contains("<xmpRights:Marked>False</xmpRights:Marked>"));
    assert_eq!(parse_xmp(&edited).marked, Some(false));
}

#[test]
fn malformed_and_truncated_never_panic() {
    let full = element_packet();
    for cut in 0..=full.len() {
        if let Ok(prefix) = std::str::from_utf8(&full.as_bytes()[..cut]) {
            let _ = parse_xmp(prefix);
            let _ = patch_xmp(prefix, &[(XmpField::Title, "x".into())]);
        }
    }
    assert!(parse_xmp("").is_empty());
    assert!(parse_xmp("<rdf:Description").is_empty());
    assert!(parse_xmp("<rdf:Description xmlns:dc=").is_empty());
}

#[test]
fn a_self_closing_description_gets_inserted_properties() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/>",
    );
    let edited = patch_xmp(&packet, &[(XmpField::Title, "New".into())]).unwrap();
    assert_eq!(parse_xmp(&edited).title.as_deref(), Some("New"));
}

#[test]
fn scalar_value_governs_the_noop_decision() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:title><rdf:Alt>\
         <rdf:li xml:lang=\"fr\">Titre</rdf:li>\
         <rdf:li xml:lang=\"x-default\">Title</rdf:li>\
         </rdf:Alt></dc:title>\
         <dc:creator><rdf:Seq><rdf:li>Ada</rdf:li><rdf:li>Grace</rdf:li></rdf:Seq></dc:creator>\
         </rdf:Description>",
    );
    assert_eq!(parse_xmp(&packet).title.as_deref(), Some("Title"));
    assert_eq!(parse_xmp(&packet).creator, vec!["Ada", "Grace"]);

    let same = patch_xmp(&packet, &[(XmpField::Title, "Title".into())]).unwrap();
    assert_eq!(same, packet, "the x-default value is already held");
    assert!(same.contains("xml:lang=\"fr\""), "siblings survive a no-op");

    let changed = patch_xmp(&packet, &[(XmpField::Title, "Titre".into())]).unwrap();
    assert_eq!(parse_xmp(&changed).title.as_deref(), Some("Titre"));
    assert!(
        !changed.contains("xml:lang=\"fr\""),
        "a non-primary alternative collapses the list"
    );

    let seq = patch_xmp(&packet, &[(XmpField::Creator, "Grace".into())]).unwrap();
    assert_eq!(parse_xmp(&seq).creator, vec!["Grace"]);
}

#[test]
fn a_comment_inside_a_managed_property_fails_closed() {
    let packet = packet(
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <photoshop:Credit>A<!--keep-->B</photoshop:Credit></rdf:Description>",
    );
    assert!(
        patch_xmp(&packet, &[(XmpField::Credit, "New".into())]).is_none(),
        "the rewrite would drop the comment"
    );
}

#[test]
fn an_over_cap_packet_fails_closed() {
    let mut big = String::from(HEAD);
    big.push_str(
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <photoshop:Credit>Old</photoshop:Credit></rdf:Description>",
    );
    big.push_str(TAIL);
    big.push_str(&"z".repeat(MAX_PACKET_BYTES));
    let _ = parse_xmp(&big);
    assert!(patch_xmp(&big, &[(XmpField::Credit, "New".into())]).is_none());
}

#[test]
fn clearing_a_property_removes_its_bytes() {
    let source = element_packet();
    let edited = patch_xmp(&source, &[(XmpField::Title, String::new())]).unwrap();
    assert!(!edited.contains("<dc:title>"), "the property is removed");
    assert_eq!(parse_xmp(&edited).title, None);
    assert!(edited.contains("<photoshop:Credit>Old Credit</photoshop:Credit>"));
    assert!(edited.contains("keep me"), "unmanaged bytes survive");

    let only_credit = packet(
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <photoshop:Credit>C</photoshop:Credit></rdf:Description>",
    );
    assert_eq!(
        patch_xmp(&only_credit, &[(XmpField::Title, String::new())]).unwrap(),
        only_credit,
        "clearing an absent property is a no-op"
    );
}

#[test]
fn insertion_requires_an_in_scope_prefix() {
    let packet = packet(
        "<rdf:Description>\
         <acme:Wrap xmlns:dc=\"http://purl.org/dc/elements/1.1/\"/>\
         </rdf:Description>\
         <rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\"/>",
    );
    assert!(
        patch_xmp(&packet, &[(XmpField::Title, "New".into())]).is_none(),
        "dc is only declared on a nested child / a sibling, so it is out of scope"
    );
}

#[test]
fn a_managed_element_with_an_attribute_fails_closed() {
    let packet = packet(
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <photoshop:Credit xml:lang=\"fr\">Old</photoshop:Credit>\
         </rdf:Description>",
    );
    assert!(
        patch_xmp(&packet, &[(XmpField::Credit, "New".into())]).is_none(),
        "the rewrite would drop the element's attribute"
    );
    assert_eq!(parse_xmp(&packet).credit.as_deref(), Some("Old"));
}

#[test]
fn a_self_closing_description_edit_and_insertion_fails_closed() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" dc:title=\"Old\"/>",
    );
    assert!(patch_xmp(&packet, &[(XmpField::Title, "New".into())]).is_some());
    assert!(patch_xmp(&packet, &[(XmpField::Credit, "C".into())]).is_some());
    assert!(
        patch_xmp(
            &packet,
            &[
                (XmpField::Title, "New".into()),
                (XmpField::Credit, "C".into()),
            ]
        )
        .is_none(),
        "the insertion rewrites the start tag that holds the attribute edit"
    );
}

#[test]
fn caps_fail_closed_without_panicking() {
    let mut many_tags = String::from(HEAD);
    many_tags.push_str("<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">");
    for _ in 0..(MAX_TOKENS + 100) {
        many_tags.push_str("<acme:x/>");
    }
    many_tags.push_str("</rdf:Description>");
    many_tags.push_str(TAIL);
    let _ = parse_xmp(&many_tags);
    assert!(patch_xmp(&many_tags, &[(XmpField::Title, "x".into())]).is_none());

    let mut many_attrs = String::from(HEAD);
    many_attrs.push_str("<rdf:Description");
    for index in 0..(MAX_ATTRS + 100) {
        many_attrs.push_str(&format!(" a{index}=\"x\""));
    }
    many_attrs.push_str("/>");
    many_attrs.push_str(TAIL);
    let _ = parse_xmp(&many_attrs);
    assert!(patch_xmp(&many_attrs, &[(XmpField::Title, "x".into())]).is_none());

    let big = packet(&format!(
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
         <photoshop:Credit>{}</photoshop:Credit></rdf:Description>",
        "a".repeat(MAX_VALUE_BYTES + 100)
    ));
    let _ = parse_xmp(&big);
    assert!(patch_xmp(&big, &[(XmpField::Credit, "x".into())]).is_none());
}

#[test]
fn serializes_and_parses_a_mixed_set() {
    let props = XmpProperties {
        title: Some("A & B".into()),
        creator: vec!["Ada".into(), "Grace".into()],
        subject: vec!["one".into(), "two".into()],
        credit: Some("Cred".into()),
        marked: Some(true),
        ..Default::default()
    };
    let packet = to_xmp_packet(&props);
    assert_eq!(parse_xmp(&packet), props);
}

#[test]
fn an_empty_set_serializes_to_an_empty_packet() {
    let packet = to_xmp_packet(&XmpProperties::default());
    assert_eq!(parse_xmp(&packet), XmpProperties::default());
    assert!(parse_xmp(&packet).is_empty());
}

#[test]
fn patch_xmp_values_writes_full_lists_and_matches_them() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:subject><rdf:Bag><rdf:li>one</rdf:li></rdf:Bag></dc:subject>\
         </rdf:Description>",
    );
    let edited = patch_xmp_values(
        &packet,
        &[(XmpField::Subject, vec!["one".into(), "two".into()])],
    )
    .unwrap();
    assert_eq!(parse_xmp(&edited).subject, vec!["one", "two"]);

    let same = patch_xmp_values(
        &edited,
        &[(XmpField::Subject, vec!["one".into(), "two".into()])],
    )
    .unwrap();
    assert_eq!(same, edited, "the full list already matches");

    let changed = patch_xmp_values(
        &edited,
        &[(XmpField::Subject, vec!["one".into(), "three".into()])],
    )
    .unwrap();
    assert_eq!(parse_xmp(&changed).subject, vec!["one", "three"]);
}

#[test]
fn patch_xmp_values_single_list_item_replaces_the_list() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:creator><rdf:Seq><rdf:li>Ada</rdf:li><rdf:li>Grace</rdf:li></rdf:Seq></dc:creator>\
         </rdf:Description>",
    );
    let shrunk = patch_xmp_values(&packet, &[(XmpField::Creator, vec!["Ada".into()])]).unwrap();
    assert_eq!(
        parse_xmp(&shrunk).creator,
        vec!["Ada"],
        "a shorter list shrinks"
    );
    let cleared = patch_xmp_values(&packet, &[(XmpField::Creator, Vec::new())]).unwrap();
    assert_eq!(parse_xmp(&cleared).creator, Vec::<String>::new());
}

#[test]
fn a_simple_element_list_is_rewritten_as_a_seq() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:creator>Ada</dc:creator>\
         </rdf:Description>",
    );
    let edited = patch_xmp_values(
        &packet,
        &[(XmpField::Creator, vec!["Ada".into(), "Grace".into()])],
    )
    .unwrap();
    assert!(edited.contains("<rdf:Seq>"), "{edited}");
    assert_eq!(parse_xmp(&edited).creator, vec!["Ada", "Grace"]);
}

#[test]
fn an_empty_property_parses_as_absent() {
    let packet = packet(
        "<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
         <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\"></rdf:li></rdf:Alt></dc:title>\
         <dc:creator><rdf:Seq><rdf:li></rdf:li></rdf:Seq></dc:creator>\
         </rdf:Description>",
    );
    let props = parse_xmp(&packet);
    assert_eq!(props.title, None);
    assert!(props.creator.is_empty());
    assert_eq!(parse_xmp(&to_xmp_packet(&props)), props);
}

#[test]
fn serializer_round_trips_special_and_non_ascii_values() {
    let props = XmpProperties {
        title: Some("Titre «café» & <x> \"q\"".into()),
        description: Some("café — naïve ✓".into()),
        credit: Some("<a>&\"b\"".into()),
        ..Default::default()
    };
    let packet = to_xmp_packet(&props);
    assert!(packet.contains("&amp;") && packet.contains("&lt;"));
    assert_eq!(parse_xmp(&packet), props);
}

#[test]
fn metadata_description_fields_parse() {
    let packet = element_packet().replace(
        "<acme:Note",
        "<photoshop:AuthorsPosition>Engineer</photoshop:AuthorsPosition>\
         <photoshop:CaptionWriter>Grace</photoshop:CaptionWriter>\
         <photoshop:City>Paris</photoshop:City>\
         <photoshop:State>Ile-de-France</photoshop:State>\
         <photoshop:Country>France</photoshop:Country>\
         <photoshop:Instructions>Careful</photoshop:Instructions>\
         <xmpRights:Marked>True</xmpRights:Marked>\
         <xmpRights:WebStatement>https://example.com/rights</xmpRights:WebStatement>\
         <xmpRights:UsageTerms><rdf:Alt><rdf:li xml:lang=\"x-default\">Editorial</rdf:li></rdf:Alt></xmpRights:UsageTerms>\
         <acme:Note",
    );
    let props = parse_xmp(&packet);
    assert_eq!(props.authors_position.as_deref(), Some("Engineer"));
    assert_eq!(props.caption_writer.as_deref(), Some("Grace"));
    assert_eq!(props.city.as_deref(), Some("Paris"));
    assert_eq!(props.state.as_deref(), Some("Ile-de-France"));
    assert_eq!(props.country.as_deref(), Some("France"));
    assert_eq!(props.instructions.as_deref(), Some("Careful"));
    assert_eq!(props.marked, Some(true));
    assert_eq!(
        props.web_statement.as_deref(),
        Some("https://example.com/rights")
    );
    assert_eq!(props.usage_terms.as_deref(), Some("Editorial"));
}
