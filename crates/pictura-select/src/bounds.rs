use pictura_core::mask_ops::coverage_bounds;
use pictura_core::PsdRect;

use super::Selection;

impl Selection {
    /// The bounding box of the covered pixels (coverage `> 0`), or `None` when
    /// the plane covers nothing.
    ///
    /// A tiled mask skips its `Empty` tiles and takes a covered `Uniform` tile
    /// whole, so only the tiles along the edge are scanned; a flat one is
    /// searched row by row for each row's first and last covered pixel.
    pub fn bounds(&self) -> Option<PsdRect> {
        let (w, h) = (self.width as usize, self.height as usize);
        coverage_bounds(&self.data, w, h).map(|(x0, y0, x1, y1)| PsdRect {
            top: y0 as i32,
            left: x0 as i32,
            bottom: y1 as i32,
            right: x1 as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: i32, h: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        }
    }

    /// The pre-change per-pixel scan, kept as the oracle for the row scan.
    fn reference_bounds(sel: &Selection) -> Option<PsdRect> {
        let mut found: Option<(i32, i32, i32, i32)> = None;
        for (i, &v) in sel.data.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let x = (i as u32 % sel.width) as i32;
            let y = (i as u32 / sel.width) as i32;
            found = Some(match found {
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                None => (x, y, x, y),
            });
        }
        found.map(|(x0, y0, x1, y1)| PsdRect {
            top: y0,
            left: x0,
            bottom: y1 + 1,
            right: x1 + 1,
        })
    }

    fn fill(sel: &mut Selection, x0: u32, y0: u32, x1: u32, y1: u32, value: u8) {
        for y in y0..y1 {
            let row = y as usize * sel.width as usize;
            sel.data[row + x0 as usize..row + x1 as usize].fill(value);
        }
    }

    #[test]
    fn bounds_matches_a_full_scan_with_padding() {
        let mut sel = Selection::none(200, 160);
        fill(&mut sel, 20, 10, 60, 40, 255);
        assert_eq!(sel.bounds(), reference_bounds(&sel));
        assert_eq!(
            sel.bounds(),
            Some(PsdRect {
                top: 10,
                left: 20,
                bottom: 40,
                right: 60
            })
        );
    }

    #[test]
    fn bounds_uses_partial_coverage() {
        let mut sel = Selection::none(8, 8);
        fill(&mut sel, 2, 3, 6, 5, 1);
        fill(&mut sel, 4, 6, 5, 7, 128);
        assert_eq!(sel.bounds(), reference_bounds(&sel));
        assert_eq!(
            sel.bounds(),
            Some(PsdRect {
                top: 3,
                left: 2,
                bottom: 7,
                right: 6
            })
        );
    }

    #[test]
    fn bounds_of_a_single_pixel() {
        let mut sel = Selection::none(5, 5);
        sel.data[3 * 5 + 4] = 255;
        assert_eq!(sel.bounds(), reference_bounds(&sel));
        assert_eq!(
            sel.bounds(),
            Some(PsdRect {
                top: 3,
                left: 4,
                bottom: 4,
                right: 5
            })
        );
    }

    #[test]
    fn bounds_of_an_empty_plane_is_none() {
        let sel = Selection::none(16, 9);
        assert_eq!(sel.bounds(), None);
        assert_eq!(reference_bounds(&sel), None);
    }

    #[test]
    fn bounds_of_a_zero_sized_plane_is_none() {
        let sel = Selection {
            width: 0,
            height: 0,
            data: Vec::new().into(),
        };
        assert_eq!(sel.bounds(), None);
    }

    #[test]
    fn bounds_of_a_full_plane_is_the_canvas() {
        let sel = Selection::all(7, 3);
        assert_eq!(sel.bounds(), Some(rect(7, 3)));
        assert_eq!(sel.bounds(), reference_bounds(&sel));
    }

    #[test]
    fn translated_bounds_clip_at_the_canvas() {
        let mut sel = Selection::none(64, 48);
        fill(&mut sel, 40, 30, 64, 48, 255);
        let moved = sel.translate(10, 10);
        assert_eq!(moved.bounds(), reference_bounds(&moved));
        assert_eq!(
            moved.bounds(),
            Some(PsdRect {
                top: 40,
                left: 50,
                bottom: 48,
                right: 64
            })
        );
    }

    #[test]
    fn translated_bounds_follow_the_offset() {
        let mut sel = Selection::none(64, 64);
        fill(&mut sel, 10, 12, 30, 32, 255);
        let moved = sel.translate(7, -5);
        assert_eq!(moved.bounds(), reference_bounds(&moved));
        let origin = sel.bounds().expect("origin bounds");
        assert_eq!(
            moved.bounds(),
            Some(PsdRect {
                top: origin.top - 5,
                left: origin.left + 7,
                bottom: origin.bottom - 5,
                right: origin.right + 7,
            })
        );
    }
}
