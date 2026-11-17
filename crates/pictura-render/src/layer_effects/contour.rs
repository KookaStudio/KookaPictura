//! A bevel contour: the PSD `ShpC` point list and its 256-step lookup table.
//!
//! ponytail: Photoshop's exact contour interpolation is unpublished, so this
//! uses piecewise linear between the stored points. The contour editor uses the
//! same rule, which is what keeps its preview and the canvas in agreement.

use pictura_codec::DescValue;

use crate::composite::desc_item;

const LINEAR_POINTS: [(f32, f32); 2] = [(0.0, 0.0), (255.0, 255.0)];

/// A contour curve, input `x` to output `y` on Photoshop's 0–255 scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Contour {
    points: Vec<(f32, f32)>,
}

impl Default for Contour {
    fn default() -> Self {
        Self::linear()
    }
}

impl Contour {
    pub fn linear() -> Self {
        Self {
            points: LINEAR_POINTS.to_vec(),
        }
    }

    /// A contour from an already-ordered point list, or `None` when it is not
    /// usable: fewer than two points, a non-finite or out-of-range value, or a
    /// non-rising input.
    pub fn from_points(points: Vec<(f32, f32)>) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }
        let mut previous = f32::NEG_INFINITY;
        for &(x, y) in &points {
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..=255.0).contains(&x)
                || !(0.0..=255.0).contains(&y)
                || x <= previous
            {
                return None;
            }
            previous = x;
        }
        Some(Self { points })
    }

    pub fn points(&self) -> &[(f32, f32)] {
        &self.points
    }

    /// Decode a `ShpC` object's `Crv` list. A missing or malformed contour is
    /// the caller's to replace with [`Contour::linear`]; this never panics.
    pub fn decode(value: &DescValue) -> Option<Self> {
        let DescValue::Object { items, .. } = value else {
            return None;
        };
        let crv = items
            .iter()
            .find(|(key, _)| key.as_slice() == b"Crv ")
            .map(|(_, value)| value)?;
        let DescValue::List(items) = crv else {
            return None;
        };
        let mut points = Vec::with_capacity(items.len());
        for item in items {
            let x = coord(item, b"Hrzn")?.clamp(0.0, 255.0);
            let y = coord(item, b"Vrtc")?.clamp(0.0, 255.0);
            points.push((x, y));
        }
        Self::from_points(points)
    }

    /// The `ShpC` descriptor for a fresh contour.
    pub fn descriptor(&self, name: &str) -> DescValue {
        let crv = self
            .points
            .iter()
            .map(|&(x, y)| DescValue::Object {
                name: String::new(),
                class_id: b"CrPt".to_vec(),
                items: vec![
                    (b"Hrzn".to_vec(), DescValue::Double(f64::from(x))),
                    (b"Vrtc".to_vec(), DescValue::Double(f64::from(y))),
                ],
            })
            .collect();
        DescValue::Object {
            name: String::new(),
            class_id: b"ShpC".to_vec(),
            items: vec![
                (b"Nm  ".to_vec(), DescValue::Text(name.into())),
                (b"Crv ".to_vec(), DescValue::List(crv)),
            ],
        }
    }

    /// The 256-step lookup table, output normalized to 0–1.
    pub fn lut(&self) -> [f32; 256] {
        let mut out = [0.0f32; 256];
        let n = self.points.len();
        for (i, slot) in out.iter_mut().enumerate() {
            let x = i as f32;
            let y = if x <= self.points[0].0 {
                self.points[0].1
            } else if x >= self.points[n - 1].0 {
                self.points[n - 1].1
            } else {
                self.points
                    .windows(2)
                    .find(|segment| x >= segment[0].0 && x <= segment[1].0)
                    .map_or(self.points[n - 1].1, |segment| {
                        let (x0, y0) = segment[0];
                        let (x1, y1) = segment[1];
                        if x1 > x0 {
                            y0 + (y1 - y0) * (x - x0) / (x1 - x0)
                        } else {
                            y1
                        }
                    })
            };
            *slot = (y / 255.0).clamp(0.0, 1.0);
        }
        out
    }

    /// Whether the curve is the identity. Checked on the LUT, so a hand-edited
    /// straight line also takes the byte-identical fast path.
    pub fn is_linear(&self) -> bool {
        if self.points.len() == 2
            && self.points[0] == LINEAR_POINTS[0]
            && self.points[1] == LINEAR_POINTS[1]
        {
            return true;
        }
        let lut = self.lut();
        (0..256).all(|i| (lut[i] - i as f32 / 255.0).abs() < 1e-6)
    }
}

fn coord(item: &DescValue, key: &[u8]) -> Option<f32> {
    match desc_item(item, key)? {
        DescValue::Double(value) if value.is_finite() => Some(*value as f32),
        DescValue::Long(value) => Some(*value as f32),
        DescValue::UnitFloat { value, .. } if value.is_finite() => Some(*value as f32),
        _ => None,
    }
}
