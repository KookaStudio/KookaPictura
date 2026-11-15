use super::*;

use super::super::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_color::{proof, Intent, Profile, ProofPreview};

/// The view's proof setup and display toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofState {
    pub device: i32,
    pub intent: i32,
    pub bpc: bool,
    pub simulate_paper: bool,
    pub simulate_black: bool,
    pub colors: bool,
}

impl Default for ProofState {
    fn default() -> Self {
        Self {
            device: 0,
            intent: 1,
            bpc: true,
            simulate_paper: false,
            simulate_black: false,
            colors: false,
        }
    }
}

fn device_profile(device: i32) -> Profile {
    match device {
        1 => Profile::gray_gamma22(),
        2 => Profile::srgb(),
        3 => Profile::adobe_rgb(),
        4 => Profile::pro_photo(),
        _ => Profile::coated_cmyk(),
    }
}

pub(in crate::cxxqt_object) fn proof_device_icc(device: i32) -> Vec<u8> {
    if device == 0 {
        Profile::coated_cmyk_icc().to_vec()
    } else {
        device_profile(device).to_icc()
    }
}

pub(in crate::cxxqt_object) fn proof_device_components(device: i32) -> u8 {
    device_profile(device).channels().unwrap_or(3)
}

pub(in crate::cxxqt_object) fn proof_intent_from_code(code: i32) -> Intent {
    intent_from_code(code)
}

fn device_name(device: i32) -> &'static str {
    match device {
        1 => "Working Gray",
        2 => "sRGB IEC61966-2.1",
        3 => "Adobe RGB (1998)",
        4 => "ProPhoto RGB",
        _ => "Working CMYK",
    }
}

fn intent_from_code(code: i32) -> Intent {
    match code {
        0 => Intent::Perceptual,
        2 => Intent::Saturation,
        3 => Intent::AbsoluteColorimetric,
        _ => Intent::RelativeColorimetric,
    }
}

fn setup(state: &ProofState) -> proof::ProofSetup {
    proof::ProofSetup {
        device: device_profile(state.device),
        intent: intent_from_code(state.intent),
        bpc: state.bpc,
        simulate_paper: state.simulate_paper,
        simulate_black: state.simulate_black,
    }
}

fn normalize(device: i32, intent: i32) -> (i32, i32) {
    (device.clamp(0, 4), intent.clamp(0, 3))
}

pub(super) fn set_proof_setup(
    mut view: Pin<&mut PictureView>,
    device: i32,
    intent: i32,
    bpc: bool,
    simulate_paper: bool,
    simulate_black: bool,
) {
    let (device, intent) = normalize(device, intent);
    let mut rust = view.as_mut().rust_mut();
    let next = ProofState {
        device,
        intent,
        bpc,
        simulate_paper,
        simulate_black,
        colors: rust.proof.colors,
    };
    if next != rust.proof {
        rust.proof = next;
        rust.proof_preview.borrow_mut().take();
    }
}

pub(super) fn set_proof_colors(mut view: Pin<&mut PictureView>, on: bool) -> bool {
    let mut rust = view.as_mut().rust_mut();
    if rust.proof.colors == on {
        return false;
    }
    rust.proof.colors = on;
    true
}

pub(super) fn proof_setup_name(view: &PictureView) -> QString {
    QString::from(device_name(view.rust().proof.device))
}

pub(super) fn proof_colors_active(view: &PictureView) -> bool {
    view.rust().proof.colors
}

pub(super) fn proof_device(view: &PictureView) -> i32 {
    view.rust().proof.device
}

pub(super) fn proof_intent(view: &PictureView) -> i32 {
    view.rust().proof.intent
}

pub(super) fn proof_bpc(view: &PictureView) -> bool {
    view.rust().proof.bpc
}

pub(super) fn proof_simulate_paper(view: &PictureView) -> bool {
    view.rust().proof.simulate_paper
}

pub(super) fn proof_simulate_black(view: &PictureView) -> bool {
    view.rust().proof.simulate_black
}

impl PictureViewRust {
    /// Proof a premultiplied RGBA8888 crop in place: unpremultiply, run the
    /// cached lcms2 proofing transform, and premultiply again. The transform is
    /// rebuilt only when the setup changes.
    pub(in crate::cxxqt_object) fn proof_rgba(&self, data: &mut [u8]) {
        if !self.proof.colors || !data.len().is_multiple_of(4) {
            return;
        }
        for pixel in data.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha == 0 {
                pixel[..3].fill(0);
            } else if alpha < 255 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha) as u8;
                }
            }
        }
        let key = (
            self.proof.device,
            self.proof.intent,
            self.proof.bpc,
            self.proof.simulate_paper,
            self.proof.simulate_black,
        );
        let mut cache = self.proof_preview.borrow_mut();
        if cache.as_ref().map(|(cached, _)| *cached) != Some(key) {
            *cache = ProofPreview::new(&setup(&self.proof))
                .ok()
                .map(|preview| (key, preview));
        }
        if let Some((_, preview)) = cache.as_ref() {
            preview.apply(data);
        }
        for pixel in data.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha < 255 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
        }
    }
}
