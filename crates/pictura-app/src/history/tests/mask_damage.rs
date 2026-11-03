//! A layer-mask edit restores as a bounded region, in document coordinates.

use crate::history::{History, Restored};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerMask, PsdRect};
use pictura_select::Selection;

const MASK: PsdRect = PsdRect {
    top: 200,
    left: 100,
    bottom: 264,
    right: 228,
};

fn masked_doc() -> Document {
    let mut doc = Document::new(512, 512, ColorMode::Rgb, BitDepth::Eight);
    let rgba = vec![128u8; 512 * 512 * 4];
    let _ = pictura_render::add_raster_layer_from_rgba(&mut doc, "px", 512, 512, &rgba);
    doc.layers[0].mask = Some(LayerMask {
        rect: MASK,
        default_color: 255,
        data: Some(vec![255u8; (MASK.width() * MASK.height()) as usize].into()),
        density: 255,
        ..Default::default()
    });
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

fn paint_mask(doc: &mut Document, x: usize, y: usize) {
    let mask = doc.layers[0].mask.as_mut().expect("mask");
    let stride = mask.rect.width() as usize;
    mask.data.as_mut().expect("mask data").as_mut_slice()[y * stride + x] = 0;
    doc.composite = pictura_render::composite_rgba(doc);
}

struct Stack {
    history: History,
    selection: Option<Selection>,
    target: LayerTarget,
}

impl Stack {
    fn opened(doc: &mut Document) -> Self {
        let mut stack = Self {
            history: History::default(),
            selection: None,
            target: LayerTarget::default(),
        };
        stack.capture(doc, "Open");
        stack
    }

    fn capture(&mut self, doc: &mut Document, label: &str) {
        self.history
            .capture_live(doc, &self.selection, &self.target, label);
    }

    fn undo(&mut self, doc: &mut Document) -> Option<Restored> {
        self.history
            .undo_live(doc, &mut self.selection, &mut self.target)
    }

    fn redo(&mut self, doc: &mut Document) -> Option<Restored> {
        self.history
            .redo_live(doc, &mut self.selection, &mut self.target)
    }
}

fn assert_bounds_the_tile(restored: Option<Restored>) {
    let Some(Restored::Region(rect)) = restored else {
        panic!("a mask edit is bounded, got {restored:?}");
    };
    let (x, y) = (MASK.left + 70, MASK.top + 10);
    assert!(
        rect.left <= x && x < rect.right && rect.top <= y && y < rect.bottom,
        "{rect:?} misses the edited mask sample"
    );
    assert!(
        rect.width() <= 128 && rect.height() <= 128,
        "{rect:?} exceeds a composite tile and a mask tile, whose grids are offset"
    );
}

#[test]
fn undoing_a_mask_edit_reports_its_tiles_in_document_coordinates() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    let before = live.clone();
    paint_mask(&mut live, 70, 10);
    stack.capture(&mut live, "Mask");
    let after = live.clone();

    assert_bounds_the_tile(stack.undo(&mut live));
    assert!(live == before, "undo restores the mask byte for byte");
    assert_bounds_the_tile(stack.redo(&mut live));
    assert!(live == after, "redo restores the mask byte for byte");
}

#[test]
fn a_moved_mask_reports_the_composite() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    let before = live.clone();
    let mask = live.layers[0].mask.as_mut().expect("mask");
    mask.rect.left += 8;
    mask.rect.right += 8;
    live.composite = pictura_render::composite_rgba(&live);
    stack.capture(&mut live, "Move Mask");
    paint_mask(&mut live, 70, 10);
    stack.capture(&mut live, "Mask");
    assert!(matches!(stack.undo(&mut live), Some(Restored::Region(_))));

    assert!(matches!(
        stack.undo(&mut live),
        Some(Restored::Composite(_))
    ));
    assert!(live == before, "undo restores the mask and its position");
}

#[test]
fn a_walk_across_a_mask_move_reports_the_composite() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    for delta in [8, -8] {
        let mask = live.layers[0].mask.as_mut().expect("mask");
        mask.rect.left += delta;
        mask.rect.right += delta;
        live.composite = pictura_render::composite_rgba(&live);
        stack.capture(&mut live, "Move Mask");
    }
    let start = live.clone();
    paint_mask(&mut live, 70, 10);
    stack.capture(&mut live, "Mask");

    let restored = stack
        .history
        .jump_live(0, &mut live, &mut stack.selection, &mut stack.target);
    assert!(matches!(restored, Some(Restored::Composite(_))));
    let _ = stack
        .history
        .jump_live(2, &mut live, &mut stack.selection, &mut stack.target);
    assert!(live == start, "the walk lands on the state it named");
}

#[test]
fn a_document_channel_edit_reports_no_rectangle() {
    let mut live = masked_doc();
    live.channels.push(Channel {
        id: 0,
        data: vec![0u8; 512 * 512].into(),
    });
    let mut stack = Stack::opened(&mut live);
    live.channels[0].data.as_mut_slice()[3] = 9;
    stack.capture(&mut live, "Channel");
    let edited = live.clone();
    assert_eq!(
        stack.undo(&mut live),
        Some(Restored::Region(PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        }))
    );
    assert!(live != edited, "undo restores the channel");
}

#[test]
fn undoing_a_masked_delete_stays_bounded() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    let before = live.clone();
    assert!(live.layers.remove(&[0]).is_some());
    live.composite = pictura_render::composite_rgba(&live);
    stack.capture(&mut live, "Delete Layer");
    assert!(matches!(
        stack.undo(&mut live),
        Some(Restored::Composite(_))
    ));
    assert!(live == before, "undo restores the masked layer");
}

#[test]
fn a_jump_lands_on_an_exact_stored_composite() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    paint_mask(&mut live, 70, 10);
    stack.capture(&mut live, "Mask");
    assert!(pictura_render::add_layer(&mut live, -1, "added") >= 0);
    stack.capture(&mut live, "New Layer");
    pictura_render::resize_document(&mut live, 256, 256, pictura_render::Resample::Nearest)
        .expect("resize");
    live.composite = pictura_render::composite_rgba(&live);
    stack.capture(&mut live, "Image Size");
    for state in [0, 3, 1, 3, 2, 0] {
        stack
            .history
            .jump_live(state, &mut live, &mut stack.selection, &mut stack.target)
            .expect("a state to jump to");
        assert!(
            live.composite == pictura_render::composite_rgba(&live),
            "state {state} holds its exact composite"
        );
    }
}

#[test]
fn an_undo_after_a_whole_jump_stays_bounded() {
    let mut live = masked_doc();
    let mut stack = Stack::opened(&mut live);
    pictura_render::resize_document(&mut live, 256, 256, pictura_render::Resample::Nearest)
        .expect("resize");
    live.composite = pictura_render::composite_rgba(&live);
    stack.capture(&mut live, "Image Size");
    let restored = stack
        .history
        .jump_live(0, &mut live, &mut stack.selection, &mut stack.target);
    assert_eq!(restored, Some(Restored::Everywhere));
    paint_mask(&mut live, 70, 10);
    stack.capture(&mut live, "Mask");
    assert!(matches!(stack.undo(&mut live), Some(Restored::Region(_))));
}
