use super::*;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::print_resources::{
    read_print_settings, update_print_settings, PrintProof, PrintProofProfile, PrintResourceEdits,
    PrintSettings,
};
use pictura_color::Intent;
use serde_json::{json, Map, Value};

pub(super) fn intent_from_number(value: i64) -> Intent {
    match value {
        0 => Intent::Perceptual,
        2 => Intent::Saturation,
        3 => Intent::AbsoluteColorimetric,
        _ => Intent::RelativeColorimetric,
    }
}

fn intent_number(intent: Intent) -> i64 {
    match intent {
        Intent::Perceptual => 0,
        Intent::RelativeColorimetric => 1,
        Intent::Saturation => 2,
        Intent::AbsoluteColorimetric => 3,
    }
}

fn portable_options(settings: &PrintSettings) -> Value {
    let mut fields = Map::new();
    macro_rules! put {
        ($id:literal, $value:expr) => {
            if let Some(value) = $value {
                fields.insert($id.into(), json!(value));
            }
        };
    }
    put!("managed", settings.manage_colors);
    put!("profile", settings.printer_profile.as_deref());
    put!("components", settings.printer_components);
    put!("intent", settings.intent.map(intent_number));
    put!("bpc", settings.black_point);
    put!("hardProof", settings.hard_proof);
    put!("calibration", settings.calibration);
    put!("registration", settings.registration);
    put!("cornerCrop", settings.corner_crop);
    put!("centerCrop", settings.center_crop);
    put!("filename", settings.filename);
    put!("description", settings.description);
    put!("background", settings.background);
    put!("border", settings.border);
    put!("bleed", settings.bleed);
    if let Some(proof) = &settings.proof {
        let (kind, name) = match &proof.profile {
            PrintProofProfile::Builtin(name) => ("builtin", name),
            PrintProofProfile::Named(name) => ("named", name),
        };
        fields.insert(
            "proof".into(),
            json!({ "kind": kind, "name": name,
            "intent": proof.intent.map(intent_number), "bpc": proof.black_point,
            "paper": proof.paper_white }),
        );
    }
    Value::Object(fields)
}

fn to_portable(value: &Value) -> PrintSettings {
    let proof = &value["proof"];
    let proof = proof["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .map(|name| {
            let profile = match proof["kind"].as_str() {
                Some("builtin") => PrintProofProfile::Builtin(name.into()),
                _ => PrintProofProfile::Named(name.into()),
            };
            PrintProof {
                profile,
                intent: proof["intent"].as_i64().map(intent_from_number),
                black_point: proof["bpc"].as_bool(),
                paper_white: proof["paper"].as_bool(),
            }
        });
    PrintSettings {
        manage_colors: value["managed"].as_bool(),
        printer_profile: value["profile"]
            .as_str()
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        printer_components: value["components"]
            .as_u64()
            .and_then(|n| u8::try_from(n).ok()),
        intent: value["intent"].as_i64().map(intent_from_number),
        black_point: value["bpc"].as_bool(),
        hard_proof: value["hardProof"].as_bool(),
        proof,
        calibration: value["calibration"].as_bool(),
        registration: value["registration"].as_bool(),
        corner_crop: value["cornerCrop"].as_bool(),
        center_crop: value["centerCrop"].as_bool(),
        filename: value["filename"].as_bool(),
        description: value["description"].as_bool(),
        background: serde_json::from_value(value["background"].clone()).ok(),
        border: value["border"].as_f64(),
        bleed: value["bleed"].as_f64(),
    }
}

pub(super) fn parse(text: &QString) -> Value {
    serde_json::from_str(&text.to_string()).unwrap_or(Value::Null)
}

pub(super) fn print_document_settings(view: &PictureView) -> QString {
    let session = &view.rust().session;
    let Some(doc) = &session.doc else {
        return QString::default();
    };
    let mut copy = doc.clone();
    session.print_resources.apply(&mut copy);
    QString::from(portable_options(&read_print_settings(&copy)).to_string())
}

pub(super) fn print_metadata_pending(view: &PictureView) -> bool {
    let session = &view.rust().session;
    session.print_revision != session.saved_print_revision
}

pub(super) fn print_document_settings_update(
    mut view: Pin<&mut PictureView>,
    old: &QString,
    new: &QString,
    commit: bool,
) -> QString {
    let old = to_portable(&parse(old));
    let new = to_portable(&parse(new));
    let binding = view.as_ref();
    let session = &binding.rust().session;
    let Some(doc) = &session.doc else {
        return QString::from("There is no document to print.");
    };
    let mut copy = doc.clone();
    session.print_resources.apply(&mut copy);
    let mut old = old;
    let managed_changed = old.manage_colors != new.manage_colors
        || old.printer_profile != new.printer_profile
        || old.printer_components != new.printer_components
        || old.intent != new.intent
        || old.black_point != new.black_point
        || old.hard_proof != new.hard_proof
        || old.proof != new.proof;
    if new.manage_colors == Some(true) && managed_changed {
        let stored = read_print_settings(&copy);
        if stored.printer_profile.is_none() {
            old.printer_profile = None;
        }
        if stored.printer_components.is_none() {
            old.printer_components = None;
        }
        if stored.intent.is_none() {
            old.intent = None;
        }
        if stored.black_point.is_none() {
            old.black_point = None;
        }
        if stored.manage_colors.is_none() {
            old.manage_colors = None;
        }
        if stored.hard_proof.is_none() {
            old.hard_proof = None;
        }
        if new.hard_proof == Some(true) && stored.proof.is_none() {
            old.proof = None;
        }
    }
    match update_print_settings(&mut copy, &old, &new) {
        Ok(true) if commit => {
            let edits = PrintResourceEdits::between(doc, &copy);
            view.as_mut()
                .rust_mut()
                .session
                .stage_print_resources(edits);
            view.as_mut().changed();
        }
        Ok(_) => {}
        Err(error) => return QString::from(error.to_string()),
    }
    QString::default()
}
