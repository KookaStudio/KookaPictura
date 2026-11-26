//! Pictura's HDR display settings, carried in a named Photoshop image resource.

use pictura_core::Document;

use crate::image_resources::{
    decode_image_resources_with_len, encode_image_resources, frame_image_resource, ImageResource,
};

const ID: u16 = 4901;
const NAME: &str = "Kooka HDR Preview";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewOptions {
    pub method: i32,
    pub exposure: f32,
    pub gamma: f32,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            method: 0,
            exposure: 0.0,
            gamma: 1.0,
        }
    }
}

impl PreviewOptions {
    pub fn new(method: i32, exposure: f32, gamma: f32) -> Option<Self> {
        (matches!(method, 0 | 1)
            && (-20.0..=20.0).contains(&exposure)
            && (0.1..=9.99).contains(&gamma))
        .then_some(Self {
            method,
            exposure,
            gamma,
        })
    }

    pub fn map(self, rgb: [f32; 3], view_ev: f32) -> [f32; 3] {
        let gain = f64::from(self.exposure + view_ev).exp2();
        let rgb = rgb.map(|channel| {
            if channel.is_finite() {
                f64::from(channel.max(0.0)) * gain
            } else {
                0.0
            }
        });
        if self.method == 1 {
            let light = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
            let shoulder = (light + 0.5).max(1.0).recip();
            return rgb.map(|channel| (channel * shoulder).clamp(0.0, 1.0) as f32);
        }
        let power = f64::from(self.gamma).recip();
        rgb.map(|channel| channel.clamp(0.0, 1.0).powf(power) as f32)
    }
}

pub(crate) fn is_preview(resource: &ImageResource) -> bool {
    resource.id == ID && resource.name == NAME
}

pub fn get(doc: &Document) -> PreviewOptions {
    decode_image_resources_with_len(doc)
        .0
        .iter()
        .filter(|resource| is_preview(resource))
        .find_map(|resource| {
            let [1, method, e0, e1, e2, e3, g0, g1, g2, g3] = resource.data.as_slice() else {
                return None;
            };
            PreviewOptions::new(
                i32::from(*method),
                f32::from_be_bytes([*e0, *e1, *e2, *e3]),
                f32::from_be_bytes([*g0, *g1, *g2, *g3]),
            )
        })
        .unwrap_or_default()
}

pub fn set(doc: &mut Document, options: PreviewOptions) {
    let (mut resources, consumed) = decode_image_resources_with_len(doc);
    resources.retain(|resource| !is_preview(resource));
    if options != PreviewOptions::default() {
        let mut data = vec![1, options.method as u8];
        data.extend_from_slice(&options.exposure.to_be_bytes());
        data.extend_from_slice(&options.gamma.to_be_bytes());
        resources.push(frame_image_resource(ID, NAME, &data));
    }
    let mut bytes = encode_image_resources(&resources);
    bytes.extend_from_slice(&doc.image_resources[consumed..]);
    doc.image_resources = bytes.into();
}

#[cfg(test)]
mod tests {
    use super::PreviewOptions;

    #[test]
    fn highlight_compression_handles_finite_extremes() {
        let options = PreviewOptions::new(1, 20.0, 1.0).unwrap();
        let mapped = options.map([f32::MAX, f32::MAX / 2.0, 0.0], 10.0);
        assert!(mapped.iter().all(|value| value.is_finite()));
        assert!(mapped[0] > mapped[1] && mapped[1] > 0.0);
    }
}
