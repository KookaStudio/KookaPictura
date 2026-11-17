use super::*;
use crate::history::History;

#[test]
fn quick_mask_coverage_keeps_temporary_clip() {
    for selected in [false, true] {
        for constant in [true, false] {
            let mut rust = fixture();
            assert!(rust.set_quick_mask_mode(true));
            rust.session.quick_mask.selected = selected;
            if !constant {
                rust.session.quick_mask.coverage = Some(Selection {
                    width: 8,
                    height: 8,
                    data: pictura_core::Plane::filled(8, 8, 1, 192),
                });
            }
            let baseline = rust.session.quick_mask.coverage.clone();
            let mut clip = Selection::none(8, 8);
            clip.data.write_rect_from(0..4, 0..8, 8, &[128; 32]);
            rust.session.set_selection(Some(clip.clone()));
            rust.session.record(&rust.layer_target, "Initial");
            let cfg = StrokeConfig {
                diameter: 8,
                hardness: 100,
                aliased: true,
                color: pictura_paint::Rgba {
                    r: if selected { 255 } else { 0 },
                    g: if selected { 255 } else { 0 },
                    b: if selected { 255 } else { 0 },
                    a: 255,
                },
                ..Default::default()
            };
            pictura_core::assert_no_materialization(|| {
                assert!(rust.begin_quick_mask_paint(cfg, "Brush"));
                assert!(rust.sample_quick_mask(4.0, 4.0, 1.0, false));
                assert!(rust.finish_quick_mask_paint());
            });
            let edited = rust.session.quick_mask.coverage.clone();
            assert!(rust.quick_mask_coverage(2, 4) < if constant { 255 } else { 192 });
            assert_eq!(
                rust.quick_mask_coverage(6, 4),
                if constant { 255 } else { 192 }
            );
            assert_eq!(rust.session.selection, Some(clip.clone()));
            rust.session.record(&rust.layer_target, "Brush");
            rust.session
                .restore(&mut rust.layer_target, History::undo_live, History::undo)
                .unwrap();
            assert_eq!(rust.session.quick_mask.coverage, baseline);
            assert_eq!(rust.session.selection, Some(clip.clone()));
            rust.session
                .restore(&mut rust.layer_target, History::redo_live, History::redo)
                .unwrap();
            assert_eq!(rust.session.quick_mask.coverage, edited);
            assert_eq!(rust.session.selection, Some(clip));
        }
    }
}
