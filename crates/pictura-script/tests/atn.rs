//! Lossless `.atn` conversion tests: raw descriptors, names, app-native
//! payloads, and Adobe event disambiguation.

use pictura_codec::action_descriptor::{
    write_action_descriptor, ActionClass, ActionDescriptor, ActionKey, ActionReferenceItem,
    ActionValue, ReferenceForm, ReferenceValue,
};
use pictura_codec::action_file::{
    read_atn as codec_read_atn, write_atn as codec_write_atn, RawAction, RawActionSet, RawAtn,
    RawEventId, RawStep,
};
use pictura_script::atn::{read_atn, write_atn};
use pictura_script::{
    command::{self, CommandParams},
    Action, ActionLibrary, ActionSet, ActionStep, DialogOptions, EventKey, StepKind, ToolStroke,
};
use serde_json::json;

fn code(name: [u8; 4], value: ActionValue) -> (ActionKey, ActionValue) {
    (ActionKey::Code(name), value)
}

fn descriptor(name: &str, items: Vec<(ActionKey, ActionValue)>) -> ActionDescriptor {
    ActionDescriptor {
        name: name.to_string(),
        class: ActionClass {
            name: name.to_string(),
            id: ActionKey::Code(*b"null"),
        },
        items,
    }
}

fn json_item(value: serde_json::Value) -> (ActionKey, ActionValue) {
    (
        ActionKey::Code(*b"jsn "),
        ActionValue::Text(value.to_string()),
    )
}

fn raw_step(
    event: RawEventId,
    descriptor: Option<ActionDescriptor>,
    display_name: &str,
) -> RawStep {
    RawStep {
        expanded: true,
        enabled: true,
        with_dialog: false,
        dialog_options: 0,
        event_id: event,
        display_name: display_name.as_bytes().to_vec(),
        descriptor,
    }
}

fn raw_set(name: Vec<u16>, steps: Vec<RawStep>) -> RawActionSet {
    RawActionSet {
        name,
        expanded: true,
        actions: vec![RawAction {
            function_key: 0,
            shift: false,
            ctrl: false,
            color: 0,
            name: "Imported".encode_utf16().chain([0]).collect(),
            expanded: true,
            steps,
        }],
    }
}

fn raw_bytes(set: RawActionSet) -> Vec<u8> {
    codec_write_atn(&RawAtn {
        set,
        trailing: Vec::new(),
    })
}

#[test]
fn mapped_steps_reemit_duplicate_unknown_and_unit_items() {
    let step = raw_step(
        RawEventId::Long(*b"GsnB"),
        Some(descriptor(
            "Gaussian Blur",
            vec![
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 4.0,
                    },
                ),
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 8.0,
                    },
                ),
                code(*b"Xtra", ActionValue::Text("keep me".into())),
                json_item(json!({ "radius": 8.0 })),
            ],
        )),
        "Gaussian Blur",
    );
    let bytes = raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    ));

    let set = read_atn(&bytes).expect("decode");
    let step = &set.actions[0].steps[0];
    assert!(matches!(
        step.kind(),
        StepKind::Command { command, .. } if command.as_str() == "filter.blur.gaussian.blur"
    ));
    assert!(step.imported().is_some(), "imported form must be kept");

    let reencoded = write_atn(&set);
    assert_eq!(bytes, reencoded, "unedited step must re-emit byte-for-byte");
}

#[test]
fn stop_play_and_menu_item_keep_their_imported_descriptors() {
    let extra = code(*b"Xtra", ActionValue::Text("unknown".into()));
    let steps = vec![
        {
            let mut item = raw_step(
                RawEventId::Long(*b"Stop"),
                Some(descriptor(
                    "Stop",
                    vec![
                        code(*b"Msge", ActionValue::Text("hello".into())),
                        code(*b"Cntn", ActionValue::Boolean(true)),
                        extra.clone(),
                    ],
                )),
                "Stop",
            );
            item.dialog_options = 4;
            item
        },
        raw_step(
            RawEventId::Long(*b"Ply "),
            Some(descriptor(
                "Play",
                vec![
                    code(*b"Nm  ", ActionValue::Text("Nested".into())),
                    extra.clone(),
                ],
            )),
            "Play",
        ),
        raw_step(
            RawEventId::Long(*b"slct"),
            Some(descriptor(
                "Insert Menu Item",
                vec![code(*b"null", ActionValue::Text("edit.copy".into())), extra],
            )),
            "menu",
        ),
    ];
    let bytes = raw_bytes(raw_set("Set".encode_utf16().chain([0]).collect(), steps));
    let set = read_atn(&bytes).expect("decode");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Stop { .. }
    ));
    assert!(matches!(
        set.actions[0].steps[1].kind(),
        StepKind::Play { .. }
    ));
    assert!(matches!(
        set.actions[0].steps[2].kind(),
        StepKind::MenuItem { .. }
    ));
    let normalized = write_atn(&set);
    let second = read_atn(&normalized).expect("decode normalized");
    assert_eq!(normalized, write_atn(&second));
    assert_eq!(
        second.actions[0].steps[2].dialog_options,
        DialogOptions::SPECIFIC
    );
}

#[test]
fn unknown_steps_round_trip_with_their_descriptor() {
    let descriptor = descriptor(
        "Mystery",
        vec![code(*b"Data", ActionValue::RawData(vec![1, 2, 3]))],
    );
    let step = {
        let mut step = ActionStep::new(StepKind::Unknown {
            event: EventKey::text("AdobeMystery"),
            descriptor: Some(write_action_descriptor(&descriptor, true)),
        });
        step.flags = Default::default();
        step.dialog_options = DialogOptions::OPTIONAL;
        step.display_name = "Mystery".into();
        step
    };
    let mut set = ActionSet::new("Set");
    let mut action = Action::new("Action");
    action.steps.push(step.clone());
    set.actions.push(action);
    let library = ActionLibrary::from_sets(vec![set]);
    let bytes = write_atn(&library.sets()[0]);
    let loaded = read_atn(&bytes).expect("load");
    let loaded_step = &loaded.actions[0].steps[0];
    assert!(matches!(
        loaded_step.kind(),
        StepKind::Unknown { event, .. } if event.display() == "AdobeMystery"
    ));
    let reencoded = write_atn(&loaded);
    let second = pictura_codec::action_file::read_atn(&reencoded).expect("codec");
    let raw_descriptor = second.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor");
    assert_eq!(raw_descriptor.items.len(), 1);
    assert_eq!(raw_descriptor.items[0].0, ActionKey::Code(*b"Data"));
    assert_eq!(loaded_step.kind(), step.kind());
}

#[test]
fn imported_names_and_display_names_are_byte_preserved() {
    let mut step = raw_step(RawEventId::Text(vec![0xFF]), None, "ignored");
    step.display_name = vec![0xFF, 0xFE];
    let set = RawActionSet {
        name: vec![0xD800, 0x0041],
        expanded: false,
        actions: vec![RawAction {
            function_key: 0,
            shift: false,
            ctrl: false,
            color: 0,
            name: vec![0x0042],
            expanded: true,
            steps: vec![step],
        }],
    };
    let bytes = raw_bytes(set);
    let decoded = read_atn(&bytes).expect("decode");
    assert_eq!(decoded.raw_name.as_deref(), Some(&[0xD800, 0x0041][..]));
    assert_eq!(decoded.actions[0].raw_name.as_deref(), Some(&[0x0042][..]));
    assert_eq!(write_atn(&decoded), bytes);

    let raw = codec_read_atn(&write_atn(&decoded)).expect("codec");
    assert_eq!(raw.set.name, vec![0xD800, 0x0041]);
    assert!(!raw.set.name.contains(&0), "no-NUL names must stay no-NUL");
    assert_eq!(raw.set.actions[0].name, vec![0x0042]);
    assert_eq!(raw.set.actions[0].steps[0].display_name, vec![0xFF, 0xFE]);
}

#[test]
fn unknown_conditional_json_is_an_unknown_step() {
    let step = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![json_item(json!({
                "v": 1,
                "condition": "DocumentIsPortrait",
                "then": "A",
                "else": null,
            }))],
        )),
        "Conditional",
    );
    let bytes = raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    ));
    let set = read_atn(&bytes).expect("decode");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Unknown { event, .. } if event.display() == "pictura.conditional"
    ));
    assert_eq!(write_atn(&set), bytes);

    let missing = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![json_item(json!({ "v": 1 }))],
        )),
        "Conditional",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![missing],
    )))
    .expect("decode missing");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Unknown { .. }
    ));
}

#[test]
fn tool_strokes_enforce_the_version_and_payload_bound() {
    let valid = ToolStroke::new("brush", CommandParams::new(), vec![]);
    let valid_step = raw_step(
        RawEventId::Text(b"pictura.toolStroke".to_vec()),
        Some(descriptor(
            "pictura.toolStroke",
            vec![json_item(serde_json::to_value(valid).expect("stroke"))],
        )),
        "Stroke",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![valid_step],
    )))
    .expect("decode valid");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::ToolStroke { .. }
    ));

    for payload in [
        json!({
            "version": 0,
            "tool": "brush",
            "settings": {},
            "samples": [],
        }),
        json!({
            "version": ToolStroke::VERSION,
            "tool": "brush",
            "settings": { "blob": "x".repeat(pictura_script::recorder::MAX_TOOL_PAYLOAD_BYTES) },
            "samples": [],
        }),
    ] {
        let step = raw_step(
            RawEventId::Text(b"pictura.toolStroke".to_vec()),
            Some(descriptor("pictura.toolStroke", vec![json_item(payload)])),
            "Stroke",
        );
        let bytes = raw_bytes(raw_set(
            "Set".encode_utf16().chain([0]).collect(),
            vec![step],
        ));
        let set = read_atn(&bytes).expect("decode invalid");
        assert!(matches!(
            set.actions[0].steps[0].kind(),
            StepKind::Unknown { event, descriptor }
                if event.display() == "pictura.toolStroke" && descriptor.is_some()
        ));
        assert_eq!(write_atn(&set), bytes, "rejected payload stays raw");
    }
}

#[test]
fn adobe_when_parameter_is_written_and_used_for_mapping() {
    let mut set = ActionSet::new("Set");
    let mut action = Action::new("Mode");
    action.steps.push(ActionStep::command(
        "image.mode.grayscale",
        CommandParams::new(),
    ));
    set.actions.push(action);
    let bytes = write_atn(&set);
    let raw = codec_read_atn(&bytes).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Md  ")
            && value
                == &ActionValue::Enumerated {
                    kind: ActionKey::Code(*b"Md  "),
                    value: ActionKey::Code(*b"Grys"),
                }
    }));

    let loaded = read_atn(&bytes).expect("decode");
    let step = &loaded.actions[0].steps[0];
    assert!(matches!(
        step.kind(),
        StepKind::Command { command, params }
            if command.as_str() == "image.mode.grayscale"
                && params.get_str("mode") == Some("grayscale")
    ));

    let imported = raw_step(
        RawEventId::Long(*b"CnvM"),
        Some(descriptor(
            "mode",
            vec![
                code(
                    *b"Md  ",
                    ActionValue::Enumerated {
                        kind: ActionKey::Code(*b"Md  "),
                        value: ActionKey::Code(*b"Grys"),
                    },
                ),
                json_item(json!({})),
            ],
        )),
        "Grayscale",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![imported],
    )))
    .expect("decode imported mode");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Command { command, .. } if command.as_str() == "image.mode.grayscale"
    ));
}

#[test]
fn dialog_options_are_validated_and_menu_items_are_specific() {
    let mut step = raw_step(
        RawEventId::Long(*b"slct"),
        Some(descriptor(
            "menu",
            vec![code(*b"null", ActionValue::Text("edit.copy".into()))],
        )),
        "menu",
    );
    step.dialog_options = 0;
    step.with_dialog = true;
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode menu");
    let decoded = &set.actions[0].steps[0];
    assert!(matches!(decoded.kind(), StepKind::MenuItem { .. }));
    assert_eq!(decoded.dialog_options, DialogOptions::SPECIFIC);
    assert!(!decoded.flags.with_dialog);

    let mut invalid = raw_step(
        RawEventId::Long(*b"Stop"),
        Some(descriptor("Stop", vec![])),
        "Stop",
    );
    invalid.dialog_options = 9;
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![invalid],
    )))
    .expect("decode invalid");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Unknown { .. }
    ));
}

#[test]
fn rotation_atn_round_trip() {
    let mut set = ActionSet::new("Rotation");
    let mut action = Action::new("Rotate");
    action.steps.push(ActionStep::command(
        "image.rotate.arbitrary",
        CommandParams::new()
            .with("angle", -24.5)
            .with("interpolation", "nearest"),
    ));
    set.actions.push(action);
    let loaded = read_atn(&write_atn(&set)).expect("Pictura round trip");
    assert!(matches!(
        loaded.actions[0].steps[0].kind(),
        StepKind::Command { command, params }
            if command.as_str() == "image.rotate.arbitrary"
                && params.get("angle").and_then(|value| value.as_f64()) == Some(-24.5)
                && params.get_str("interpolation") == Some("nearest")
    ));

    let imported = raw_step(
        RawEventId::Long(*b"Rtte"),
        Some(descriptor(
            "Rotate",
            vec![code(
                *b"Angl",
                ActionValue::UnitFloat {
                    unit: *b"#Ang",
                    value: -24.5,
                },
            )],
        )),
        "Rotate",
    );
    let bytes = raw_bytes(raw_set(
        "Adobe".encode_utf16().chain([0]).collect(),
        vec![imported],
    ));
    let loaded = read_atn(&bytes).expect("import Adobe");
    assert!(matches!(
        loaded.actions[0].steps[0].kind(),
        StepKind::Command { command, params }
            if command.as_str() == "image.rotate.arbitrary"
                && params.get("angle").and_then(|value| value.as_f64()) == Some(-24.5)
                && params.get("interpolation").is_none()
    ));
}

#[test]
fn rotate_and_flip_events_map_by_angle_axis_and_family() {
    let angle = |value: f64| CommandParams::new().with("angle", value);
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(180.0)),
        Some("image.rotate180")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(90.0)),
        Some("image.rotate90cw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(-90.0)),
        Some("image.rotate90ccw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(270.0)),
        Some("image.rotate90ccw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(45.0)),
        Some("image.rotate.arbitrary")
    );
    let layer = CommandParams::new().with("angle", 90.0).with("layer", true);
    assert_eq!(
        command::command_id_for_event("Rtte", &layer),
        Some("edit.transform.rotate90cw")
    );
    let layer180 = CommandParams::new()
        .with("angle", 180.0)
        .with("layer", true);
    assert_eq!(
        command::command_id_for_event("Rtte", &layer180),
        Some("edit.transform.rotate180")
    );

    let horizontal = CommandParams::new().with("axis", "horizontal");
    assert_eq!(
        command::command_id_for_event("Flp ", &horizontal),
        Some("image.flipHorizontal")
    );
    let vertical = CommandParams::new().with("axis", "vertical");
    assert_eq!(
        command::command_id_for_event("Flp ", &vertical),
        Some("image.flipVertical")
    );
    let layer_vertical = CommandParams::new()
        .with("axis", "Vrtc")
        .with("layer", true);
    assert_eq!(
        command::command_id_for_event("Flp ", &layer_vertical),
        Some("edit.transform.flipVertical")
    );
    assert_eq!(
        command::command_id_for_event("Flp ", &CommandParams::new()),
        None
    );
}

#[test]
fn a_layer_reference_selects_the_transform_family() {
    let reference = ActionValue::Reference(vec![ActionReferenceItem {
        form: ReferenceForm::Class,
        class: ActionClass {
            name: "layer".into(),
            id: ActionKey::Code(*b"Lyr "),
        },
        key: None,
        value: ReferenceValue::None,
    }]);
    let step = raw_step(
        RawEventId::Long(*b"Rtte"),
        Some(descriptor(
            "rotate",
            vec![
                (ActionKey::Code(*b"null"), reference),
                code(
                    *b"Angl",
                    ActionValue::UnitFloat {
                        unit: *b"#Ang",
                        value: 90.0,
                    },
                ),
            ],
        )),
        "Rotate 90° Clockwise",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode");
    assert!(matches!(
        set.actions[0].steps[0].kind(),
        StepKind::Command { command, .. } if command.as_str() == "edit.transform.rotate90cw"
    ));
}

#[test]
fn editing_a_parameter_clears_the_imported_descriptor() {
    let step = raw_step(
        RawEventId::Long(*b"GsnB"),
        Some(descriptor(
            "Gaussian Blur",
            vec![
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 4.0,
                    },
                ),
                code(*b"Xtra", ActionValue::Text("drop me when edited".into())),
                json_item(json!({ "radius": 4.0 })),
            ],
        )),
        "Gaussian Blur",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode");
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.set_step_params(0, 0, 0, CommandParams::new().with("radius", 2.0)));
    let bytes = write_atn(&library.sets()[0]);
    let raw = codec_read_atn(&bytes).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Rds ")
            && value
                == &ActionValue::UnitFloat {
                    unit: *b"#Pxl",
                    value: 2.0,
                }
    }));
    assert!(!items
        .iter()
        .any(|(key, _)| key == &ActionKey::Code(*b"Xtra")));
}

#[test]
fn renaming_an_imported_name_replaces_stale_units() {
    let mut library = ActionLibrary::from_sets(vec![read_atn(&raw_bytes(raw_set(
        vec![0x0041],
        vec![raw_step(RawEventId::Text(vec![0x41]), None, "Step")],
    )))
    .expect("decode")]);
    assert!(library.rename_set(0, "New Set"));
    assert!(library
        .rename_action(0, 0, "New Action")
        .expect("rename action"));
    let raw = codec_read_atn(&library.set_bytes(0).expect("write")).expect("codec");
    assert_eq!(
        raw.set.name,
        "New Set".encode_utf16().chain([0]).collect::<Vec<_>>()
    );
    assert_eq!(
        raw.set.actions[0].name,
        "New Action".encode_utf16().chain([0]).collect::<Vec<_>>()
    );
}

fn round_trip_single_step(step: ActionStep) -> ActionStep {
    let mut set = ActionSet::new("Batch");
    let mut action = Action::new("A");
    action.steps.push(step);
    set.actions.push(action);
    let back = read_atn(&write_atn(&set)).expect("decode");
    back.actions[0].steps[0].clone()
}

#[test]
fn atn_round_trips_an_open_step() {
    let step = ActionStep::command(
        "file.open",
        CommandParams::from_value(json!({"path": "/home/u/pictures/a.png"})),
    );
    let raw = codec_read_atn(&write_atn(&{
        let mut set = ActionSet::new("Batch");
        let mut action = Action::new("A");
        action.steps.push(step.clone());
        set.actions.push(action);
        set
    }))
    .expect("codec");
    assert_eq!(
        raw.set.actions[0].steps[0].event_id,
        RawEventId::Long(*b"Opn ")
    );
    let back = round_trip_single_step(step);
    let StepKind::Command { command, params } = back.kind() else {
        panic!("not a command step: {:?}", back.kind());
    };
    assert_eq!(command.as_str(), "file.open");
    assert_eq!(params.get_str("path"), Some("/home/u/pictures/a.png"));
}

#[test]
fn atn_round_trips_save_as_options() {
    let params = json!({
        "as": "JPG",
        "in": "/tmp/out.jpg",
        "options": {"quality": 4, "format": 2, "scans": 3, "matte": -1, "preview": true}
    });
    let back = round_trip_single_step(ActionStep::command(
        "file.saveAs",
        CommandParams::from_value(params.clone()),
    ));
    let StepKind::Command {
        command,
        params: read,
    } = back.kind()
    else {
        panic!("not a command step: {:?}", back.kind());
    };
    assert_eq!(command.as_str(), "file.saveAs");
    assert_eq!(read.get("options"), params.get("options"));
    assert_eq!(read.get_str("in"), Some("/tmp/out.jpg"));
}

fn raw_action(name: &str, steps: Vec<RawStep>) -> RawAction {
    RawAction {
        function_key: 0,
        shift: false,
        ctrl: false,
        color: 0,
        name: name.encode_utf16().chain([0]).collect(),
        expanded: true,
        steps,
    }
}

fn raw_set_actions(name: &str, actions: Vec<RawAction>) -> RawActionSet {
    RawActionSet {
        name: name.encode_utf16().chain([0]).collect(),
        expanded: true,
        actions,
    }
}

fn imported_set(source: RawActionSet) -> ActionSet {
    read_atn(&raw_bytes(source)).expect("decode imported set")
}

fn reload(library: &ActionLibrary) -> ActionLibrary {
    let bytes = library.set_bytes(0).expect("one set");
    let mut loaded = ActionLibrary::empty();
    loaded.load_bytes("test.atn", &bytes).expect("reload");
    loaded
}

fn first_step(library: &ActionLibrary) -> &ActionStep {
    &library.sets()[0].actions[0].steps[0]
}

fn play_kind(step: &ActionStep) -> &str {
    match step.kind() {
        StepKind::Play { action } => action,
        other => panic!("expected Play, got {other:?}"),
    }
}

fn imported_blur() -> RawStep {
    raw_step(
        RawEventId::Long(*b"GsnB"),
        Some(descriptor(
            "Gaussian Blur",
            vec![
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 4.0,
                    },
                ),
                json_item(json!({ "radius": 4.0 })),
            ],
        )),
        "Gaussian Blur",
    )
}

#[test]
fn renaming_an_action_updates_an_imported_play_reference() {
    let play = raw_step(
        RawEventId::Long(*b"Ply "),
        Some(descriptor(
            "Play",
            vec![code(*b"Nm  ", ActionValue::Text("Target".into()))],
        )),
        "Play",
    );
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![play]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let reloaded = reload(&library);
    assert_eq!(play_kind(first_step(&reloaded)), "Renamed");
}

#[test]
fn renaming_an_action_updates_imported_conditional_branches() {
    let mut conditional = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![
                json_item(json!({
                    "v": 1,
                    "condition": "DocumentHasSelection",
                    "then": "Target",
                    "else": "Target",
                })),
                code(*b"Xtra", ActionValue::Text("keep me".into())),
            ],
        )),
        "Conditional",
    );
    conditional.display_name = vec![0xFF, 0xFE];
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![conditional]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let bytes = library.set_bytes(0).expect("set");
    let raw = codec_read_atn(&bytes).expect("codec");
    let step = &raw.set.actions[0].steps[0];
    let items = &step.descriptor.as_ref().expect("descriptor").items;
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Xtra")
            && matches!(value, ActionValue::Text(text) if text == "keep me")
    }));
    let saved_jsn = items
        .iter()
        .find_map(|(key, value)| {
            if key != &ActionKey::Code(*b"jsn ") {
                return None;
            }
            match value {
                ActionValue::Text(text) => serde_json::from_str::<serde_json::Value>(text).ok(),
                _ => None,
            }
        })
        .expect("jsn item");
    assert_eq!(saved_jsn.get("v"), Some(&json!(1)));
    assert_eq!(step.display_name, vec![0xFF, 0xFE]);
    let reloaded = reload(&library);
    match first_step(&reloaded).kind() {
        StepKind::Conditional {
            then_action,
            else_action,
            ..
        } => {
            assert_eq!(then_action.as_deref(), Some("Renamed"));
            assert_eq!(else_action.as_deref(), Some("Renamed"));
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
}

#[test]
fn renaming_a_jsn_only_play_reference_updates_it() {
    let play = raw_step(
        RawEventId::Long(*b"Ply "),
        Some(descriptor(
            "Play",
            vec![json_item(json!({ "action": "Target" }))],
        )),
        "Play",
    );
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![play]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let reloaded = reload(&library);
    assert_eq!(play_kind(first_step(&reloaded)), "Renamed");
}

#[test]
fn renaming_keeps_unknown_imported_descriptor_items() {
    let mut play = raw_step(
        RawEventId::Long(*b"Ply "),
        Some(descriptor(
            "Play",
            vec![
                code(*b"Nm  ", ActionValue::Text("Target".into())),
                code(*b"Nm  ", ActionValue::Text("Target".into())),
                code(*b"Xtra", ActionValue::Text("keep me".into())),
                json_item(json!({ "action": "Target" })),
            ],
        )),
        "Play",
    );
    play.display_name = vec![0xFF, 0xFE];
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![play]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let bytes = library.set_bytes(0).expect("set");
    let raw = codec_read_atn(&bytes).expect("codec");
    let step = &raw.set.actions[0].steps[0];
    let items = &step.descriptor.as_ref().expect("descriptor").items;
    let renamed = items
        .iter()
        .filter(|(key, value)| {
            key == &ActionKey::Code(*b"Nm  ")
                && matches!(value, ActionValue::Text(text) if text == "Renamed")
        })
        .count();
    assert_eq!(renamed, 2);
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Xtra")
            && matches!(value, ActionValue::Text(text) if text == "keep me")
    }));
    let saved_jsn = items
        .iter()
        .find_map(|(key, value)| {
            if key != &ActionKey::Code(*b"jsn ") {
                return None;
            }
            match value {
                ActionValue::Text(text) => serde_json::from_str::<serde_json::Value>(text).ok(),
                _ => None,
            }
        })
        .expect("jsn item");
    assert_eq!(saved_jsn.get("action"), Some(&json!("Renamed")));
    assert_eq!(step.display_name, vec![0xFF, 0xFE]);
    assert_eq!(play_kind(first_step(&reload(&library))), "Renamed");
}

#[test]
fn renaming_a_play_preserves_unrelated_json_bytes() {
    let source = r#" { "precision" : 1.234567890123456789, "action" : "Target", "dup": 1, "dup": 2, "nested": { "action": "Target" }, "action" : "Target" } "#;
    let expected = r#" { "precision" : 1.234567890123456789, "action" : "Renamed", "dup": 1, "dup": 2, "nested": { "action": "Target" }, "action" : "Renamed" } "#;
    let play = raw_step(
        RawEventId::Long(*b"Ply "),
        Some(descriptor(
            "Play",
            vec![code(*b"jsn ", ActionValue::Text(source.to_string()))],
        )),
        "Play",
    );
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![play]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let raw = codec_read_atn(&library.set_bytes(0).expect("set")).expect("codec");
    let saved = raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items
        .iter()
        .find_map(|(key, value)| {
            if key != &ActionKey::Code(*b"jsn ") {
                return None;
            }
            match value {
                ActionValue::Text(text) => Some(text.as_str()),
                _ => None,
            }
        })
        .expect("jsn item");
    assert_eq!(saved, expected);
    assert_eq!(play_kind(first_step(&reload(&library))), "Renamed");
}

#[test]
fn renaming_a_conditional_leaves_unrelated_jsn_sidecars_untouched() {
    let conditional = r#" { "v":1, "condition":"DocumentHasSelection", "then":"Target", "else":null, "precision":1.234567890123456789 } "#;
    let expected = r#" { "v":1, "condition":"DocumentHasSelection", "then":"Renamed", "else":null, "precision":1.234567890123456789 } "#;
    let unrelated = r#"{"then":"Target","else":null,"note":"keep"}"#;
    let matching_copy =
        r#"{ "condition": "DocumentHasSelection", "then": "Target", "else": null, "copy": true }"#;
    let expected_copy =
        r#"{ "condition": "DocumentHasSelection", "then": "Renamed", "else": null, "copy": true }"#;
    let other_condition = r#"{"condition":"DocumentIsSquare","then":"Target","else":null}"#;
    let other_branches = r#"{"condition":"DocumentHasSelection","then":"Other","else":null}"#;
    let step = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![
                code(*b"jsn ", ActionValue::Text(conditional.to_string())),
                code(*b"jsn ", ActionValue::Text(unrelated.to_string())),
                code(*b"jsn ", ActionValue::Text(matching_copy.to_string())),
                code(*b"jsn ", ActionValue::Text(other_condition.to_string())),
                code(*b"jsn ", ActionValue::Text(other_branches.to_string())),
            ],
        )),
        "Conditional",
    );
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![step]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let raw = codec_read_atn(&library.set_bytes(0).expect("set")).expect("codec");
    let saved: Vec<_> = raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items
        .iter()
        .filter_map(|(key, value)| {
            if key != &ActionKey::Code(*b"jsn ") {
                return None;
            }
            match value {
                ActionValue::Text(text) => Some(text.as_str()),
                _ => None,
            }
        })
        .collect();
    assert_eq!(
        saved,
        vec![
            expected,
            unrelated,
            expected_copy,
            other_condition,
            other_branches
        ]
    );
    match first_step(&reload(&library)).kind() {
        StepKind::Conditional {
            then_action,
            else_action,
            ..
        } => {
            assert_eq!(then_action.as_deref(), Some("Renamed"));
            assert_eq!(else_action.as_deref(), None);
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
}

#[test]
fn renaming_a_conditional_with_conflicting_duplicate_branches_reauthors() {
    let decoded =
        r#"{"condition":"DocumentHasSelection","then":"Other","then":"Target","else":null}"#;
    let later = r#"{"condition":"DocumentHasSelection","then":"Target","else":null}"#;
    let step = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![
                code(*b"jsn ", ActionValue::Text(decoded.to_string())),
                code(*b"jsn ", ActionValue::Text(later.to_string())),
                code(*b"Xtra", ActionValue::Text("drop me".into())),
            ],
        )),
        "Conditional",
    );
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![step]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    assert!(first_step(&library).imported().is_none());
    match first_step(&reload(&library)).kind() {
        StepKind::Conditional {
            then_action,
            else_action,
            ..
        } => {
            assert_eq!(then_action.as_deref(), Some("Renamed"));
            assert_eq!(else_action.as_deref(), None);
        }
        other => panic!("expected Conditional, got {other:?}"),
    }
    let raw = codec_read_atn(&library.set_bytes(0).expect("set")).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert_eq!(items.len(), 1);
    assert!(!items
        .iter()
        .any(|(key, _)| key == &ActionKey::Code(*b"Xtra")));
}

#[test]
fn renaming_an_action_leaves_unrelated_imported_steps_untouched() {
    let play = raw_step(
        RawEventId::Long(*b"Ply "),
        Some(descriptor(
            "Play",
            vec![code(*b"Nm  ", ActionValue::Text("Target".into()))],
        )),
        "Play",
    );
    let mut unknown = raw_step(
        RawEventId::Text(b"AdobeMystery".to_vec()),
        Some(descriptor(
            "Mystery",
            vec![code(*b"Data", ActionValue::RawData(vec![1, 2, 3]))],
        )),
        "Mystery",
    );
    unknown.display_name = vec![0xFF, 0xFE];
    let set = imported_set(raw_set_actions(
        "Set",
        vec![
            raw_action("Caller", vec![play, unknown]),
            raw_action("Target", Vec::new()),
        ],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    let before = library.set_bytes(0).expect("set");
    assert!(library.rename_action(0, 1, "Renamed").expect("rename"));
    let after = library.set_bytes(0).expect("set");
    let before_raw = codec_read_atn(&before).expect("codec");
    let after_raw = codec_read_atn(&after).expect("codec");
    let before_step = &before_raw.set.actions[0].steps[1];
    let after_step = &after_raw.set.actions[0].steps[1];
    assert_eq!(after_step, before_step);
    assert_eq!(after_step.display_name, vec![0xFF, 0xFE]);
}

#[test]
fn structural_step_edits_keep_the_imported_form() {
    let other = raw_step(RawEventId::Long(*b"slct"), None, "menu");
    let set = imported_set(raw_set_actions(
        "Set",
        vec![raw_action("A", vec![imported_blur(), other])],
    ));
    let original = set.actions[0].steps[0]
        .imported()
        .cloned()
        .expect("imported blur");
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.move_step(0, 0, 0, 2));
    assert!(library.duplicate_step(0, 0, 1));
    let reloaded = reload(&library);
    let steps = &reloaded.sets()[0].actions[0].steps;
    assert_eq!(steps.len(), 3);
    let preserved: Vec<_> = steps
        .iter()
        .filter_map(|step| step.imported())
        .filter(|form| form.descriptor.is_some())
        .collect();
    assert_eq!(preserved.len(), 2);
    for form in preserved {
        assert_eq!(form.descriptor, original.descriptor);
        assert_eq!(form.event, original.event);
        assert_eq!(form.display_name, original.display_name);
    }
}

#[test]
fn no_op_parameter_edit_keeps_the_imported_form() {
    let set = imported_set(raw_set_actions(
        "Set",
        vec![raw_action("A", vec![imported_blur()])],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    let before = library.set_bytes(0).expect("set");
    let params = match first_step(&library).kind() {
        StepKind::Command { params, .. } => params.clone(),
        other => panic!("expected Command, got {other:?}"),
    };
    assert!(library.set_step_params(0, 0, 0, params));
    assert_eq!(library.set_bytes(0).expect("set"), before);
}

#[test]
fn repeated_insert_path_keeps_the_imported_form() {
    let path = raw_step(
        RawEventId::Text(b"pictura.insertPath".to_vec()),
        Some(descriptor(
            "pictura.insertPath",
            vec![
                json_item(json!({ "path": "Work Path" })),
                code(*b"Xtra", ActionValue::Text("keep me".into())),
            ],
        )),
        "Insert Path",
    );
    let set = imported_set(raw_set_actions("Set", vec![raw_action("A", vec![path])]));
    let mut library = ActionLibrary::from_sets(vec![set]);
    let imported = first_step(&library)
        .imported()
        .cloned()
        .expect("imported path");
    let before = library.set_bytes(0).expect("set");
    assert_eq!(library.insert_path(0, 0, "Work Path").expect("insert"), 0);
    let after = library.set_bytes(0).expect("set");
    assert_eq!(after, before);
    assert_eq!(first_step(&library).imported(), Some(&imported));
    let before_raw = codec_read_atn(&before).expect("codec");
    let after_raw = codec_read_atn(&after).expect("codec");
    assert_eq!(
        before_raw.set.actions[0].steps[0],
        after_raw.set.actions[0].steps[0]
    );
}

#[test]
fn changing_an_imported_insert_path_reauthors_the_step() {
    let path = raw_step(
        RawEventId::Text(b"pictura.insertPath".to_vec()),
        Some(descriptor(
            "pictura.insertPath",
            vec![
                json_item(json!({ "path": "Work Path" })),
                code(*b"Xtra", ActionValue::Text("drop me".into())),
            ],
        )),
        "Insert Path",
    );
    let set = imported_set(raw_set_actions("Set", vec![raw_action("A", vec![path])]));
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert_eq!(library.insert_path(0, 0, "New Path").expect("insert"), 0);
    assert!(first_step(&library).imported().is_none());
    let reloaded = reload(&library);
    assert!(matches!(
        first_step(&reloaded).kind(),
        StepKind::InsertPath { path } if path == "New Path"
    ));
    let raw = codec_read_atn(&library.set_bytes(0).expect("set")).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert!(!items
        .iter()
        .any(|(key, _)| key == &ActionKey::Code(*b"Xtra")));
    let saved = items
        .iter()
        .find_map(|(key, value)| {
            if key != &ActionKey::Code(*b"jsn ") {
                return None;
            }
            match value {
                ActionValue::Text(text) => serde_json::from_str::<serde_json::Value>(text).ok(),
                _ => None,
            }
        })
        .expect("jsn item");
    assert_eq!(saved.get("path"), Some(&json!("New Path")));
}

#[test]
fn flag_only_edits_keep_the_imported_form() {
    let set = imported_set(raw_set_actions(
        "Set",
        vec![raw_action("A", vec![imported_blur()])],
    ));
    let mut library = ActionLibrary::from_sets(vec![set]);
    let before = library.set_bytes(0).expect("set");
    assert!(library.set_step_enabled(0, 0, 0, false));
    assert!(library.set_step_with_dialog(0, 0, 0, true));
    let after = library.set_bytes(0).expect("set");
    let before_raw = codec_read_atn(&before).expect("codec");
    let after_raw = codec_read_atn(&after).expect("codec");
    assert_eq!(
        before_raw.set.actions[0].steps[0].descriptor,
        after_raw.set.actions[0].steps[0].descriptor
    );
}
