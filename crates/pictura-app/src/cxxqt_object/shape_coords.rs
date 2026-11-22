//! The flat anchor/handle form the shell and the shape/path bridge exchange:
//! six numbers per point (`ax, ay, ix, iy, ox, oy`, a missing handle equal to
//! its anchor), preceded by a `NaN, closed` marker for every subpath.

use pictura_core::path::{PathPoint, Subpath};

pub(in crate::cxxqt_object) fn encode(subpaths: &[Subpath]) -> Vec<f64> {
    let mut out = Vec::new();
    for subpath in subpaths {
        out.extend([
            f64::NAN,
            if subpath.closed { 1.0 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            0.0,
        ]);
        out.extend(subpath.points.iter().flat_map(|p| {
            let (ix, iy) = p.in_handle.unwrap_or(p.anchor);
            let (ox, oy) = p.out_handle.unwrap_or(p.anchor);
            [p.anchor.0, p.anchor.1, ix, iy, ox, oy]
        }));
    }
    out
}

pub(in crate::cxxqt_object) fn decode(coords: &[f64]) -> Option<Vec<Subpath>> {
    let mut subpaths: Vec<Subpath> = Vec::new();
    let mut current: Option<Subpath> = None;
    let mut i = 0;
    while i < coords.len() {
        if coords[i].is_nan() {
            if let Some(subpath) = current.take() {
                subpaths.push(subpath);
            }
            let closed = *coords.get(i + 1)? != 0.0;
            current = Some(Subpath {
                points: Vec::new(),
                closed,
            });
            i += 6;
            continue;
        }
        let values = coords.get(i..i + 6)?;
        if !values.iter().all(|v| v.is_finite()) {
            return None;
        }
        let anchor = (values[0], values[1]);
        let in_handle = (values[2], values[3]);
        let out_handle = (values[4], values[5]);
        let point = PathPoint {
            anchor,
            in_handle: (in_handle != anchor).then_some(in_handle),
            out_handle: (out_handle != anchor).then_some(out_handle),
            smooth: in_handle != anchor && out_handle != anchor,
        };
        current
            .get_or_insert_with(|| Subpath {
                points: Vec::new(),
                closed: true,
            })
            .points
            .push(point);
        i += 6;
    }
    if let Some(subpath) = current {
        subpaths.push(subpath);
    }
    subpaths.retain(|subpath| !subpath.points.is_empty());
    (!subpaths.is_empty()).then_some(subpaths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_trip_keeps_closedness_and_handles() {
        let paths = vec![
            Subpath {
                points: vec![
                    PathPoint {
                        anchor: (1.0, 2.0),
                        in_handle: None,
                        out_handle: Some((3.0, 4.0)),
                        smooth: false,
                    },
                    PathPoint {
                        anchor: (5.0, 6.0),
                        in_handle: Some((7.0, 8.0)),
                        out_handle: None,
                        smooth: false,
                    },
                ],
                closed: true,
            },
            Subpath {
                points: vec![PathPoint {
                    anchor: (9.0, 10.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                }],
                closed: false,
            },
        ];
        assert_eq!(decode(&encode(&paths)), Some(paths));
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[f64::NAN, 1.0, 0.0]), None);
    }
}
