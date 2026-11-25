use super::*;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn view() -> PictureViewRust {
    let rgba = vec![200u8; 8 * 8 * 4];
    let doc = Document::from_rgba("Layer 0", 8, 8, &rgba);
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(doc));
    rust.session.dirty = true;
    rust
}

fn path(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "pictura-save-job-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

fn wait(rust: &PictureViewRust) {
    while save_running_rust(rust) {
        std::thread::yield_now();
    }
}

fn save_compatible(doc: Document, name: &str, include: bool, dirty: bool) -> Vec<u8> {
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(doc));
    rust.session.dirty = dirty;
    let target = path(name);
    assert!(save_start_with_compatibility(
        &mut rust,
        &target,
        -1,
        false,
        Some(include)
    ));
    assert!(save_finish_rust(&mut rust));
    let bytes = std::fs::read(&target).unwrap();
    std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
    bytes
}

fn colored_document() -> Document {
    let rgba: Vec<u8> = (0..4096)
        .flat_map(|i| [(i % 251) as u8, (i % 173) as u8, 84, 255])
        .collect();
    let mut doc = Document::from_rgba("Color", 64, 64, &rgba);
    let mut hidden = doc.layers[0].clone();
    hidden.name = "Hidden".into();
    hidden.visible = false;
    doc.layers = LayerStack::from_layers(vec![doc.layers[0].clone(), hidden]);
    doc
}

#[test]
fn native_save_respects_merged_output_choice() {
    for name in ["color.psd", "color.psb"] {
        let mut source = colored_document();
        let expected = pictura_render::composite_rgba(&source);
        source.composite.detach_plane();
        let include = save_compatible(source.clone(), name, true, false);
        let omit = save_compatible(source, name, false, false);
        assert!(omit.len() < include.len());
        let include = pictura_codec::read_psd(&include).unwrap();
        let omit = pictura_codec::read_psd(&omit).unwrap();
        assert!(include.composite.merged_output());
        assert!(!omit.composite.merged_output());
        assert_eq!(omit.is_psb, name.ends_with(".psb"));
        assert_eq!(include.layers, omit.layers);
        assert_eq!(
            &include.composite.data[..64 * 64 * 3],
            &expected.data[..64 * 64 * 3]
        );
    }
}

#[test]
fn native_save_restores_merged_image() {
    for name in ["restore.psd", "restore.psb"] {
        let omitted = save_compatible(colored_document(), name, false, false);
        let mut doc = pictura_codec::read_psd(&omitted).unwrap();
        doc.layers[0].channels_mut()[0].data.as_mut_slice().fill(88);
        let expected = pictura_render::composite_rgba(&doc);
        let restored = save_compatible(doc, name, true, true);
        let restored = pictura_codec::read_psd(&restored).unwrap();
        assert!(restored.composite.merged_output());
        assert!(restored.channels.is_empty());
        assert!(can_omit_merged(&restored));
        assert_eq!(
            &restored.composite.data[..64 * 64 * 3],
            &expected.data[..64 * 64 * 3]
        );
        assert_eq!(restored.layers.len(), 2);
    }
}

#[test]
fn native_restore_keeps_merged_transparency_separate() {
    for name in ["transparent.psd", "transparent.psb"] {
        let rgba = [40, 90, 150, 80].repeat(64);
        let source = Document::from_rgba("Transparent", 8, 8, &rgba);
        let omitted = save_compatible(source, name, false, false);
        let doc = pictura_codec::read_psd(&omitted).unwrap();
        let expected = pictura_render::composite_rgba(&doc);
        let restored = pictura_codec::read_psd(&save_compatible(doc, name, true, true)).unwrap();
        assert!(restored.channels.is_empty());
        assert!(can_omit_merged(&restored));
        assert_eq!(restored.composite.channels, 4);
        assert_eq!(restored.composite.data.as_ref(), expected.data.as_ref());
    }
}

#[test]
fn signed_alpha_survives_direct_and_noninteractive_resave() {
    for name in ["signed.psd", "signed.psb"] {
        let source = Document::from_rgba("Transparent", 8, 8, &[40, 90, 150, 80].repeat(64));
        let encoded = save_compatible(source, name, true, true);
        let mut signed = pictura_codec::read_psd(&encoded).unwrap();
        assert!(signed.composite.signed_merged_alpha());
        signed.channels.push(pictura_core::Channel {
            id: 0,
            data: vec![37; 64].into(),
        });
        let direct = if name.ends_with(".psb") {
            pictura_codec::write_psb(&signed).unwrap()
        } else {
            pictura_codec::write_psd(&signed).unwrap()
        };
        let lazy = pictura_codec::read_psd_with_options(
            &direct,
            &pictura_codec::OpenOptions {
                skip_layered_merged: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(lazy.composite.signed_merged_alpha());
        assert_eq!(lazy.channels.len(), 1);
        assert_eq!(lazy.channels[0].data.as_ref(), &[37; 64]);
        let mut reopened = pictura_codec::read_psd(&direct).unwrap();
        assert!(reopened.composite.signed_merged_alpha());
        assert_eq!(reopened.composite.data[3 * 64], 80);
        assert_eq!(reopened.channels.len(), 1);
        assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64]);
        reopened.composite.detach_plane();
        let mut rust = PictureViewRust::default();
        rust.session.doc.set(Some(reopened));
        let target = path(name);
        assert!(save_start_rust(&mut rust, &target, -1, false));
        assert!(save_finish_rust(&mut rust));
        let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
        assert!(reopened.composite.signed_merged_alpha());
        assert_eq!(reopened.composite.data[3 * 64], 80);
        assert_eq!(reopened.channels.len(), 1);
        assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64]);
        std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
    }
}

#[test]
fn legacy_save_stages_detached_omitted_composite() {
    let encoded = save_compatible(colored_document(), "omitted.psd", false, false);
    let mut doc = pictura_codec::read_psd(&encoded).unwrap();
    doc.composite.detach_plane();
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(doc));
    let target = path("legacy.psd");
    assert!(save_start_rust(&mut rust, &target, -1, false));
    assert!(save_finish_rust(&mut rust));
    let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
    assert!(!reopened.composite.merged_output());
    assert_eq!(reopened.layers.len(), 2);
    std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
}

#[test]
fn legacy_save_preserves_channels_from_marked_placeholder() {
    use pictura_codec::{encode_image_resources, frame_image_resource};
    let mut source = colored_document();
    source.channels.push(pictura_core::Channel {
        id: 0,
        data: vec![37; 64 * 64].into(),
    });
    let version_info = [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let marker = frame_image_resource(1057, "", &version_info);
    source.image_resources = encode_image_resources(std::slice::from_ref(&marker)).into();
    let mut encoded = pictura_codec::write_psd(&pictura_render::save_view(&source)).unwrap();
    let offset = encoded
        .windows(marker.raw.len())
        .position(|bytes| bytes == marker.raw)
        .unwrap();
    let flag = marker.raw.len() - marker.data.len() - marker.data.len() % 2 + 4;
    encoded[offset + flag] = 0;
    let mut doc = pictura_codec::read_psd(&encoded).unwrap();
    assert!(!doc.composite.merged_output());
    assert_eq!(doc.channels[0].data.as_ref(), &[37; 64 * 64]);
    doc.composite.detach_plane();
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(doc));
    let target = path("with-channel.psd");
    assert!(save_start_rust(&mut rust, &target, -1, false));
    assert!(save_finish_rust(&mut rust));
    let reopened = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
    assert!(reopened.composite.merged_output());
    assert_eq!(reopened.channels[0].data.as_ref(), &[37; 64 * 64]);
    std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
}

#[test]
fn direct_writer_synchronizes_existing_version_info() {
    use pictura_codec::{decode_image_resources, encode_image_resources, frame_image_resource};
    let mut doc = colored_document();
    let marker = frame_image_resource(1057, "", &[0, 0, 0, 1, 0]);
    let other = frame_image_resource(1005, "", &[1, 2, 3, 4]);
    doc.image_resources = encode_image_resources(&[marker, other.clone()]).into();
    for bytes in [
        pictura_codec::write_psd(&doc).unwrap(),
        pictura_codec::write_psb(&doc).unwrap(),
    ] {
        let back = pictura_codec::read_psd(&bytes).unwrap();
        let resources = decode_image_resources(&back);
        assert_eq!(resources.iter().find(|r| r.id == 1057).unwrap().data[4], 1);
        assert_eq!(
            resources.iter().find(|r| r.id == 1005).unwrap().raw,
            other.raw
        );
        assert!(back.composite.merged_output());
    }
}

#[test]
fn native_save_preserves_required_image_data() {
    for name in ["protected.psd", "protected.psb"] {
        let flat =
            Document::with_uniform_composite(8, 8, ColorMode::Grayscale, BitDepth::Eight, 44);
        let flat = pictura_codec::read_psd(&save_compatible(flat, name, false, false)).unwrap();
        assert!(flat.composite.merged_output());
        assert_eq!(flat.composite.data.as_ref(), &[44; 64]);

        let mut layered = colored_document();
        layered.channels.push(pictura_core::Channel {
            id: 0,
            data: vec![37; 64 * 64].into(),
        });
        let layered =
            pictura_codec::read_psd(&save_compatible(layered, name, false, false)).unwrap();
        assert!(layered.composite.merged_output());
        assert_eq!(layered.layers.len(), 2);
        assert_eq!(layered.channels[0].data.as_ref(), &[37; 64 * 64]);
    }
}

#[test]
fn native_save_restores_adobe_omitted_fixture() {
    use pictura_codec::{decode_image_resources, encode_image_resources, frame_image_resource};

    // Generated from the observed structure of the local Photoshop 2021
    // reference: resource 1057 with false, plus a white 3-channel RLE image.
    let mut doc = colored_document();
    doc.composite
        .replace(pictura_core::PixelBuffer {
            width: 64,
            height: 64,
            channels: 3,
            data: vec![255; 64 * 64 * 3].into(),
        })
        .unwrap();
    let version_info = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let marker = frame_image_resource(1057, "", &version_info);
    doc.image_resources = encode_image_resources(std::slice::from_ref(&marker)).into();
    let mut encoded = pictura_codec::write_psd(&doc).unwrap();
    let mut real_marker = marker.raw.clone();
    let flag = real_marker.len() - marker.data.len() - marker.data.len() % 2 + 4;
    real_marker[flag] = 1;
    let offset = encoded
        .windows(real_marker.len())
        .position(|bytes| bytes == real_marker)
        .unwrap();
    encoded[offset + flag] = 0;
    let mut doc = pictura_codec::read_psd(&encoded).unwrap();
    assert!(!doc.composite.merged_output());
    doc.layers[0].channels_mut()[0].data.as_mut_slice().fill(80);
    let output = save_compatible(doc, "adobe.psd", true, true);
    let back = pictura_codec::read_psd(&output).unwrap();
    assert!(back.composite.merged_output());
    assert_eq!(back.composite.data[0], 80);
    let resource = decode_image_resources(&back)
        .into_iter()
        .find(|resource| resource.id == 1057)
        .unwrap();
    assert_eq!(resource.data[4], 1);
}

fn native_layered(mode: ColorMode, depth: BitDepth) -> Document {
    use pictura_core::{PixelBuffer, Samples, SourceChannels};

    let color_count = mode.color_channels();
    let mut doc = Document::from_rgba("Native", 8, 8, &[100, 100, 100, 255].repeat(64));
    doc.mode = mode;
    let layer = &mut doc.layers[0];
    layer
        .channels_mut()
        .retain(|channel| channel.id < i16::from(color_count));
    let planes = layer
        .channels()
        .iter()
        .map(|channel| {
            let samples = match (depth, channel.id) {
                (BitDepth::Sixteen, -1) => Samples::U16(vec![u16::MAX; 64].into()),
                (BitDepth::Sixteen, _) => Samples::U16(vec![25_701; 64].into()),
                (BitDepth::ThirtyTwo, -1) => Samples::F32(vec![1.0; 64].into()),
                _ => Samples::F32(vec![0.393; 64].into()),
            };
            (channel.id, samples)
        })
        .collect();
    layer.source_channels = Some(SourceChannels::new(depth, layer.rect, planes));
    doc.source.set_native(Some(depth), None).unwrap();
    doc.composite
        .replace(PixelBuffer {
            width: 8,
            height: 8,
            channels: color_count,
            data: vec![100; 64 * usize::from(color_count)].into(),
        })
        .unwrap();
    doc
}

#[test]
fn native_save_restores_missing_native_composite() {
    for mode in [ColorMode::Rgb, ColorMode::Grayscale] {
        for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
            for name in ["native.psd", "native.psb"] {
                let source = native_layered(mode, depth);
                let encoded = save_compatible(source, name, false, false);
                let omitted = pictura_codec::read_psd(&encoded).unwrap();
                assert!(!omitted.composite.merged_output());
                assert!(omitted.source.planes().is_none());
                for dirty in [false, true] {
                    let mut doc = omitted.clone();
                    if dirty {
                        doc.layers[0].name = "Edited".into();
                    }
                    let expected = doc.authoritative_plane(&doc.layers[0], 0).unwrap().clone();
                    let encoded = save_compatible(doc, name, true, dirty);
                    let restored = pictura_codec::read_psd(&encoded).unwrap();
                    assert!(restored.composite.merged_output());
                    assert_eq!(restored.source.depth(), Some(depth));
                    let native = restored.source.planes().as_ref().unwrap();
                    assert_eq!(native.samples.slice(0..64).unwrap(), expected);
                }
            }
        }
    }
}

#[test]
fn native_depth_save_regressions() {
    use pictura_core::{Channel, Samples, SourcePlanes};

    fn psd_layer_section_and_image(bytes: &[u8]) -> (&[u8], &[u8]) {
        let end = |start: usize| {
            start + 4 + u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize
        };
        let layers = end(end(26));
        let image = end(layers);
        (&bytes[layers..image], &bytes[image..])
    }

    let mut doc = native_layered(ColorMode::Rgb, BitDepth::Sixteen);
    assert!(pictura_render::restore_native_composite(&mut doc));
    let unchanged =
        pictura_codec::read_psd(&save_compatible(doc.clone(), "native.psd", true, false)).unwrap();
    assert!(unchanged.channels.is_empty());
    assert_eq!(unchanged.source.depth(), Some(BitDepth::Sixteen));
    let edited = save_compatible(doc.clone(), "native.psd", true, true);
    let native = pictura_codec::read_psd(&edited).unwrap();
    assert_eq!(
        native
            .source
            .planes()
            .as_ref()
            .unwrap()
            .samples
            .slice(0..64),
        doc.source.planes().as_ref().unwrap().samples.slice(0..64)
    );

    let mut with_channel = doc.clone();
    with_channel.channels.push(Channel {
        id: 0,
        data: vec![98; 64].into(),
    });
    let native =
        pictura_codec::read_psd(&save_compatible(with_channel, "channel.psd", true, true)).unwrap();
    assert_eq!(native.channels.len(), 1);
    assert_eq!(native.channels[0].data.as_ref(), &[98; 64]);

    let mut converted = native_layered(ColorMode::Rgb, BitDepth::Sixteen);
    converted
        .source
        .set_encoding(Some(ColorMode::Cmyk), None)
        .unwrap();
    converted
        .source
        .replace_planes(Some(SourcePlanes {
            depth: BitDepth::Sixteen,
            width: 8,
            height: 8,
            samples: Samples::U16(vec![25_701; 4 * 64].into()),
        }))
        .unwrap();
    let expected = pictura_codec::write_psd(&converted).unwrap();
    let saved = save_compatible(converted, "converted.psd", true, true);
    assert_eq!(&saved[..26], &expected[..26]);
    assert_eq!(
        psd_layer_section_and_image(&saved),
        psd_layer_section_and_image(&expected)
    );

    doc.layers = LayerStack::default();
    let expected = pictura_codec::write_psd(&doc).unwrap();
    let saved = save_compatible(doc, "native.psd", true, true);
    assert_eq!(&saved[..26], &expected[..26]);
    assert_eq!(
        psd_layer_section_and_image(&saved).1,
        psd_layer_section_and_image(&expected).1
    );
    let reopened = pictura_codec::read_psd(&saved).unwrap();
    assert!(reopened.composite.signed_merged_alpha());
    assert!(reopened.channels.is_empty());
}

#[test]
fn native_save_codec_preconditions() {
    let mut doc = colored_document();
    doc.composite.set_merged_output(false);
    let bytes = pictura_codec::write_psd(&doc).unwrap();
    let omitted = pictura_codec::read_psd(&bytes).unwrap();
    assert!(!omitted.composite.merged_output());
    assert!(pictura_codec::decode_image_resources(&omitted)
        .iter()
        .all(|resource| resource.id != 1057));
    doc.composite.detach_plane();
    assert!(pictura_codec::write_psd(&doc).is_err());
}

#[test]
fn explicit_native_format_selects_the_container() {
    use pictura_codec::SaveFormat;

    for (format, name, version) in [
        (SaveFormat::Psd, "explicit-psd.png", 1),
        (SaveFormat::Psb, "explicit-psb.png", 2),
    ] {
        let mut rust = view();
        let target = path(name);
        assert!(save_start_rust(&mut rust, &target, format.id(), false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let bytes = std::fs::read(&target).unwrap();
        assert_eq!(bytes[5], version);
        assert_eq!(
            pictura_codec::read_psd(&bytes).unwrap().is_psb,
            format == SaveFormat::Psb
        );
        std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
    }

    let mut rust = view();
    let target = path("refused.psd");
    assert!(!save_start_rust(
        &mut rust,
        &target,
        SaveFormat::Png.id(),
        false
    ));
    assert!(!save_start_rust(&mut rust, &target, i32::MAX, false));
    assert!(!std::path::Path::new(&target).exists());
    std::fs::remove_dir_all(std::path::Path::new(&target).parent().unwrap()).unwrap();
}

#[test]
fn native_save_stamps_document_versions() {
    for (name, native_version) in [("version.psd", 1), ("version.psb", 2)] {
        let mut rust = view();
        let target = path(name);
        assert!(save_start_rust(&mut rust, &target, -1, false));
        wait(&rust);
        assert!(save_finish_rust(&mut rust));
        let bytes = std::fs::read(&target).unwrap();
        assert_eq!(bytes[5], native_version);
        let doc = pictura_codec::read_psd(&bytes).unwrap();
        let read = pictura_codec::document_version(&doc);
        let stamp = read.stamp.unwrap();
        assert_eq!(
            stamp.op_document_version,
            pictura_codec::CURRENT_DOCUMENT_VERSION
        );
        assert_eq!(stamp.min_reader_version, stamp.op_document_version);
        assert_eq!(stamp.writer_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(read.access, pictura_codec::VersionAccess::Editable);
        assert!(read.version_info.unwrap().has_real_merged_data);
        let packet = pictura_codec::read_metadata(&doc).xmp;
        assert!(packet.contains("opDocumentVersion"));
        assert!(packet.contains("minReaderVersion"));
        assert!(packet.contains("writerVersion"));
        let _ = std::fs::remove_file(target);
    }
}

#[test]
fn save_absent_composite_preserves_merged_output_policy() {
    let mut rust = view();
    let expected = pictura_render::composite_rgba(rust.session.doc.as_ref().unwrap());
    rust.session.doc.as_mut().unwrap().composite.detach_plane();
    assert!(pictura_codec::write_psd(rust.session.doc.as_ref().unwrap()).is_err());
    let target = path("absent.psd");
    assert!(save_start_rust(&mut rust, &target, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    let saved = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
    assert!(saved.composite.merged_output());
    assert!(saved.composite.signed_merged_alpha());
    assert_eq!(saved.composite.buffer(), &expected);
    assert!(saved.channels.is_empty());
    let doc = rust.session.doc.as_mut().unwrap();
    doc.composite.replace(expected).unwrap();
    doc.composite.set_merged_output(false);
    assert!(save_start_rust(&mut rust, &target, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    let saved = pictura_codec::read_psd(&std::fs::read(&target).unwrap()).unwrap();
    assert!(!saved.composite.merged_output());
    let _ = std::fs::remove_file(target);
}

#[test]
fn a_started_save_finishes_writes_and_clears_modified() {
    let mut rust = view();
    let path = path("save.psd");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    assert_eq!(rust.path.as_deref(), Some(path.as_str()));
    assert!(!rust.session.dirty);
    assert!(std::path::Path::new(&path).is_file());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn print_metadata_survives_undo_and_concurrent_save() {
    use pictura_codec::print_resources::{
        read_print_settings, update_print_settings, PrintResourceEdits,
    };

    let mut rust = view();
    let target = crate::layer_target::LayerTarget::default();
    rust.session.record(&target, "Open");
    rust.session.mark_saved();
    rust.session.record(&target, "Image change");
    let stage = |rust: &mut PictureViewRust, border| {
        let doc = rust.session.doc.as_ref().unwrap();
        let mut edited = doc.clone();
        rust.session.print_resources.apply(&mut edited);
        let old = read_print_settings(&edited);
        let mut new = old.clone();
        new.border = Some(border);
        update_print_settings(&mut edited, &old, &new).unwrap();
        let changes = PrintResourceEdits::between(doc, &edited);
        rust.session.stage_print_resources(changes);
    };
    let image_history = rust.session.history.current_id();
    stage(&mut rust, 4.0);
    assert_eq!(rust.session.history.current_id(), image_history);
    let restored = rust.session.history.undo().unwrap();
    rust.session.doc.set(Some(restored.doc));
    assert!(rust.session.derived_dirty());
    let path = path("print.psb");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    stage(&mut rust, 6.0);
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    assert!(rust.session.dirty);
    let first = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(read_print_settings(&first).border, Some(4.0));
    assert!(save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    assert!(!rust.session.dirty);
    let second = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(read_print_settings(&second).border, Some(6.0));
    let _ = std::fs::remove_file(path);
}

#[test]
fn save_refuses_unreadable_tiles() {
    use crate::cxxqt_object::scratch_rollback::tests::{dense_doc, lose_tiles};
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(dense_doc()));
    rust.session.dirty = true;
    lose_tiles(rust.session.doc.as_ref().unwrap());
    let path = path("unreadable.psd");
    assert!(!save_start_rust(&mut rust, &path, -1, false));
    assert!(crate::cxxqt_object::helpers::memory_gate::edit_refused_for_scratch());
    assert!(!std::path::Path::new(&path).exists(), "nothing was written");
}

#[test]
fn a_second_start_while_running_is_refused() {
    let mut rust = view();
    // Keep the worker provably alive after its encode; an 8×8 document can
    // otherwise finish inside `thread::spawn`'s window and the second start
    // legitimately succeeds.
    rust.save_delay_for_test = 200;
    let path = path("twice.psd");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    assert!(!save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_edit_during_the_save_keeps_the_document_modified() {
    let mut rust = view();
    let path = path("stale.psd");
    let snapshot = rust.session.doc.snapshot().expect("document");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    assert!(rust.session.dirty);
    assert_eq!(rust.path.as_deref(), Some(path.as_str()));
    let written = std::fs::read(&path).expect("saved file");
    let expected =
        pictura_codec::write_versioned(&snapshot, false, None, env!("CARGO_PKG_VERSION"), false)
            .expect("snapshot bytes")
            .bytes;
    assert_eq!(written, expected, "the file holds the pre-edit snapshot");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_replaced_document_keeps_its_path_and_dirty_state() {
    let mut rust = view();
    rust.path = Some("original.psd".to_string());
    let path = path("replaced.psd");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    rust.session
        .doc
        .set(Some(Document::from_rgba("Other", 4, 4, &[1u8; 4 * 4 * 4])));
    rust.session.document_generation = rust.session.document_generation.wrapping_add(1);
    wait(&rust);
    assert!(!save_finish_rust(&mut rust));
    assert_eq!(rust.path.as_deref(), Some("original.psd"));
    assert!(rust.session.dirty);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_failed_write_leaves_the_document_untouched() {
    let mut rust = view();
    let path = path("save.psd");
    std::fs::create_dir_all(&path).unwrap();
    assert!(save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(!save_finish_rust(&mut rust));
    assert!(rust.path.is_none());
    assert!(rust.session.dirty);
    let dir = std::path::Path::new(&path).parent().expect("parent");
    let leftovers = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(leftovers, 0, "a failed save leaves no temp file");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_save_distinguishes_publication_failures() {
    let mut rust = view();
    let target = path("publication.psd");
    rust.path = Some("prior.psd".into());
    std::fs::write(&target, b"prior snapshot").unwrap();
    let bad_target = format!("{target}/missing.psd");
    assert!(save_start_rust(&mut rust, &bad_target, -1, false));
    wait(&rust);
    assert!(!save_finish_rust(&mut rust));
    assert!(!rust.session.save_publication_uncertain);
    assert_eq!(std::fs::read(&target).unwrap(), b"prior snapshot");
    rust.save_sync_failure_for_test = true;
    assert!(save_start_rust(&mut rust, &target, -1, false));
    wait(&rust);
    assert!(!save_finish_rust(&mut rust));
    assert!(rust.session.save_publication_uncertain);
    assert_eq!(rust.path.as_deref(), Some("prior.psd"));
    assert!(rust.session.dirty);
    assert!(pictura_codec::read_psd(&std::fs::read(&target).unwrap()).is_ok());
    let _ = std::fs::remove_file(target);
}

#[test]
fn a_synchronous_save_writes_the_worker_bytes() {
    let mut rust = view();
    let path = path("sync.psd");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    let written = std::fs::read(&path).expect("saved file");
    let doc = rust.session.doc.as_ref().expect("document");
    let expected =
        pictura_codec::write_versioned(doc, false, None, env!("CARGO_PKG_VERSION"), false)
            .expect("fresh encode")
            .bytes;
    assert_eq!(written, expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_layered_document_round_trips_through_the_job() {
    let rgba = vec![10u8; 4 * 4 * 4];
    let mut doc = Document::from_rgba("Layer 0", 4, 4, &rgba);
    let mut layer = doc.layers.get(0).expect("layer").clone();
    layer.name = "Layer 1".to_string();
    layer.channels_mut()[1].data.as_mut_slice().fill(9);
    let original = doc.layers.get(0).expect("layer").clone();
    doc.layers = LayerStack::from_layers(vec![layer, original]);
    doc.composite
        .replace(pictura_core::PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: vec![10u8; 4 * 4 * 4].into(),
        })
        .expect("valid composite");
    doc.mode = ColorMode::Rgb;
    doc.depth = BitDepth::Eight;
    let mut rust = PictureViewRust::default();
    rust.session.doc.set(Some(doc));
    rust.session.dirty = true;
    let path = path("layers.psd");
    assert!(save_start_rust(&mut rust, &path, -1, false));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    let back = pictura_codec::read_psd(&std::fs::read(&path).unwrap()).expect("reads");
    assert_eq!(back.layers.len(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn copy_intent_skips_path_and_dirty() {
    let mut rust = view();
    rust.path = Some("original.psd".to_string());
    let path = path("copy.psd");
    assert!(save_start_rust(&mut rust, &path, -1, true));
    wait(&rust);
    assert!(save_finish_rust(&mut rust));
    assert_eq!(rust.path.as_deref(), Some("original.psd"));
    assert!(rust.session.dirty);
    assert!(std::path::Path::new(&path).is_file());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn native_save_replaced_session_panic() {
    let mut rust = view();
    let target = path("replaced-panic.psd");
    rust.save_fault_for_test = 4;
    assert!(save_start_rust(&mut rust, &target, -1, false));
    let generation = rust.session.document_generation;
    rust.session = crate::cxxqt_object::document_session::DocumentSession::with_document(
        Document::from_rgba("Replacement", 4, 4, &[1; 64]),
    );
    rust.session.document_generation = generation.wrapping_add(1);
    rust.path = Some("replacement.psd".to_owned());
    let dirty = rust.session.dirty;
    let history = rust.session.history.current_id();
    wait(&rust);
    let result = finish(&mut rust);
    assert_eq!(result.code, 6);
    assert_eq!(result.publication, 2);
    assert!(result.replaced);
    assert_eq!(rust.path.as_deref(), Some("replacement.psd"));
    assert_eq!(rust.session.dirty, dirty);
    assert_eq!(rust.session.history.current_id(), history);
    assert!(!rust.session.save_publication_uncertain);
    assert!(pictura_codec::read_psd(&std::fs::read(&target).unwrap()).is_ok());
    let _ = std::fs::remove_file(target);
}

#[test]
fn admission_status_is_owned_by_the_save_attempt() {
    use crate::cxxqt_object::helpers::memory_gate::{clear, set_cgroup_for_test};
    let mut rust = view();
    let target = path("admission.psd");
    let limit = 1_i64 << 30;
    set_cgroup_for_test(limit, 768_i64 << 20);
    assert!(!save_start_rust(&mut rust, &target, -1, false));
    set_cgroup_for_test(-1, 0);
    clear();
    assert_eq!(rust.last_native_save.code, ErrorCode::Host.status().raw());
    assert_eq!(rust.last_native_save.admission, 1);
}
