//! Guides: non-printing horizontal and vertical lines placed from the rulers.

/// Which way a guide runs. A horizontal guide sits at a document row, a
/// vertical guide at a document column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

/// One guide at `position` document pixels along the axis across it (a row
/// for a horizontal guide, a column for a vertical one). It may lie outside
/// the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub orientation: GuideOrientation,
    pub position: f64,
}

/// The guide nearest `(x, y)` within `radius` document pixels, measured
/// across the guide; ties go to the topmost (last placed).
pub fn guide_near(guides: &[Guide], x: f64, y: f64, radius: f64) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, guide) in guides.iter().enumerate() {
        let along = match guide.orientation {
            GuideOrientation::Horizontal => y,
            GuideOrientation::Vertical => x,
        };
        let distance = (along - guide.position).abs();
        if distance <= radius && best.is_none_or(|(_, d)| distance <= d) {
            best = Some((i, distance));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guide(orientation: GuideOrientation, position: f64) -> Guide {
        Guide {
            orientation,
            position,
        }
    }

    #[test]
    fn nearest_guide_across_its_axis() {
        let guides = [
            guide(GuideOrientation::Vertical, 10.0),
            guide(GuideOrientation::Horizontal, 20.0),
            guide(GuideOrientation::Vertical, 13.0),
        ];
        assert_eq!(guide_near(&guides, 11.0, 90.0, 2.0), Some(0));
        assert_eq!(guide_near(&guides, 12.0, 90.0, 2.0), Some(2));
        assert_eq!(guide_near(&guides, 50.0, 21.5, 2.0), Some(1));
        assert_eq!(guide_near(&guides, 50.0, 90.0, 2.0), None);
        // Equidistant: the later guide wins.
        assert_eq!(guide_near(&guides, 11.5, 90.0, 2.0), Some(2));
        assert_eq!(guide_near(&[], 0.0, 0.0, 5.0), None);
    }
}
