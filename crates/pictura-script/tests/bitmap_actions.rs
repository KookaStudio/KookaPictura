use pictura_codec::preset_library::{encode_native, Kind, Library, Payload, Preset};
use pictura_codec::PatternPixels;
use pictura_script::{
    atn::{read_atn, write_atn}, command::{CommandId, CommandParams}, Action, ActionLibrary,
    ActionSet, DialogOptions, DocumentFacts, PlaybackCommand, PlaybackOptions, PlaybackSelection,
    Player, Recorder, Selection, StepKind,
};
use serde_json::json;

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for part in bytes.chunks(3) {
        let a = part[0];
        let b = part.get(1).copied().unwrap_or(0);
        let c = part.get(2).copied().unwrap_or(0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if part.len() > 1 { TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char } else { '=' });
        out.push(if part.len() > 2 { TABLE[(c & 63) as usize] as char } else { '=' });
    }
    out
}

#[test]
fn bitmap_actions_record_modal_and_roundtrip_large_pattern() {
    let native = encode_native(&Library {
        kind: Kind::Patterns,
        entries: vec![Preset {
            name: "Large".into(),
            payload: Payload::Pattern(PatternPixels {
                pattern_id: "large".into(), width: 512, height: 512,
                rgba: vec![127; 512 * 512 * 4],
            }),
            opaque: vec![],
        }],
    }).unwrap();
    let encoded = base64(&native);
    assert!(encoded.len() > 1 << 20);
    let params = CommandParams::from_value(json!({
        "mode": "bitmap", "method": 4, "patternId": "large", "patternNative": encoded,
    }));
    let mut set = ActionSet::new("Bitmap");
    set.actions.push(Action::new("Convert"));
    let mut library = ActionLibrary::from_sets(vec![set]);
    let mut recorder = Recorder::new();
    recorder.start(&library, Selection::Action { set: 0, action: 0 }).unwrap();
    assert!(recorder.record(&mut library, &CommandId::new("image.mode.bitmap"), &params).unwrap());
    assert_eq!(library.sets()[0].actions[0].steps[0].dialog_options, DialogOptions::OPTIONAL);
    assert!(library.set_step_with_dialog(0, 0, 0, true));
    let bytes = write_atn(&library.sets()[0]);
    let restored = read_atn(&bytes).expect("large pattern must survive ATN decoding");
    let StepKind::Command { params: restored_params, .. } = restored.actions[0].steps[0].kind() else {
        panic!("expected Bitmap command");
    };
    assert_eq!(restored_params, &params);
    let loaded = ActionLibrary::from_sets(vec![restored]);
    let facts = DocumentFacts { present: true, square: true, mode: "grayscale".into(), has_selection: false };
    let mut player = Player::start(&loaded, PlaybackSelection::Action { set: 0, action: 0 }, &facts, PlaybackOptions::default()).unwrap();
    match player.next().unwrap() {
        PlaybackCommand::Execute { command, params: replayed, modal, .. } => {
            assert_eq!(command.as_str(), "image.mode.bitmap");
            assert_eq!(replayed, params);
            assert!(modal);
        }
        other => panic!("unexpected playback: {other:?}"),
    }
}

#[test]
fn bitmap_action_reads_legacy_json_native_pattern() {
    use pictura_codec::action_descriptor::{ActionClass, ActionDescriptor, ActionKey, ActionValue};
    use pictura_codec::action_file::{write_atn as write_raw, RawAction, RawActionSet, RawAtn, RawEventId, RawStep};
    let descriptor = ActionDescriptor {
        name: "Bitmap".into(),
        class: ActionClass { name: "null".into(), id: ActionKey::Code(*b"null") },
        items: vec![
            (ActionKey::Code(*b"cmid"), ActionValue::Text("image.mode.bitmap".into())),
            (ActionKey::Code(*b"jsn "), ActionValue::Text(json!({"mode":"bitmap", "method":4,
                "patternId":"old", "patternNative":"YWJj"}).to_string())),
        ],
    };
    let bytes = write_raw(&RawAtn {
        set: RawActionSet { name: vec![83, 0], expanded: true, actions: vec![RawAction {
            function_key: 0, shift: false, ctrl: false, color: 0, name: vec![65, 0], expanded: true,
            steps: vec![RawStep { expanded: true, enabled: true, with_dialog: false,
                dialog_options: 0, event_id: RawEventId::Long(*b"CnvM"),
                display_name: b"Bitmap".to_vec(), descriptor: Some(descriptor) }],
        }] }, trailing: vec![],
    });
    let restored = read_atn(&bytes).unwrap();
    let StepKind::Command { params, .. } = restored.actions[0].steps[0].kind() else {
        panic!("expected Bitmap command");
    };
    assert_eq!(params.get_str("patternNative"), Some("YWJj"));
}
