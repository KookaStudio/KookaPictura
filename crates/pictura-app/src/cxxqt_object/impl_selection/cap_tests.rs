use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::state::PictureViewRust;
use pictura_core::{BitDepth, Channel, ColorMode, Document};

#[test]
fn save_selection_refuses_at_cap() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for i in 0..53 {
        doc.channels.push(Channel {
            id: i,
            data: vec![0u8; 16].into(),
        });
    }
    let mut rust = PictureViewRust {
        session: DocumentSession::with_document(doc),
        ..Default::default()
    };
    rust.session.selection = Some(Selection::all(4, 4));
    assert!(!rust
        .session
        .doc
        .as_ref()
        .expect("document")
        .can_add_channel());

    let outcome = save_selection_channel(&mut rust);
    assert_eq!(outcome, Edit::None);
    assert_eq!(
        rust.session.doc.as_ref().expect("document").channels.len(),
        53
    );
    assert_eq!(rust.session.history.count(), 0);
}
