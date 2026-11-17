use super::*;
use pictura_core::{BitDepth, SourceState};

#[test]
fn stored_composite_queries_agree_with_document() {
    let mut doc = Document::from_rgba("pixel", 1, 1, &[1, 2, 3, 255]);
    for source in [
        SourceState::default(),
        SourceState::new(None, Some(BitDepth::Sixteen), None, None).unwrap(),
    ] {
        doc.source = source;
        assert_eq!(crate::document_color_mode(&doc), doc.document_color_mode());
        assert_eq!(crate::document_bit_depth(&doc), doc.document_bit_depth());
        assert!(DocumentComposite::is_stored(&doc));
        let data = doc.composite.detach_plane();
        assert_eq!(
            DocumentComposite::is_stored(&doc),
            doc.has_stored_composite()
        );
        assert!(!doc.has_stored_composite());
        doc.composite.attach_plane(data).unwrap();
        doc.width = 2;
        assert_eq!(
            DocumentComposite::is_stored(&doc),
            doc.has_stored_composite()
        );
        assert!(!doc.has_stored_composite());
        doc.width = 1;
    }
}
