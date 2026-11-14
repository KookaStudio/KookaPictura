use pictura_codec::{read_psd, write_psd};
use pictura_core::{BitDepth, Channel, ColorMode, Document};

fn add_extras(doc: &mut Document, count: usize) {
    for i in 0..count {
        doc.channels.push(Channel {
            id: i as i16,
            data: vec![0u8; 4].into(),
        });
    }
}

#[test]
fn extra_channels_write_at_the_56_channel_limit() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    add_extras(&mut doc, 53);

    let bytes = write_psd(&doc).expect("3 colour + 53 extras = 56 channels");
    let back = read_psd(&bytes).expect("read the 56-channel file");
    assert_eq!(back.channels.len(), 53);

    add_extras(&mut doc, 1);
    assert!(
        write_psd(&doc).is_err(),
        "3 colour + 54 extras = 57 channels must be rejected"
    );
}
