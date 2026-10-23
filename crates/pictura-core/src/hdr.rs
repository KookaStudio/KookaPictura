//! Shared HDR Toning controls.
//!
//! The 8-bit HDR Toning filter and the f32 HDR Conversion path use the same
//! nine Local Adaptation controls. Keeping the type and its ranges here stops
//! the two surfaces from drifting.

/// Local Adaptation controls, in dialog order. `Default` is the `Default`
/// preset the HDR Toning dialog opens with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrToningParams {
    pub radius: f64,
    pub strength: f64,
    pub gamma: f64,
    pub exposure: f64,
    pub detail: f64,
    pub shadow: f64,
    pub highlight: f64,
    pub vibrance: f64,
    pub saturation: f64,
}

impl Default for HdrToningParams {
    fn default() -> Self {
        Self {
            radius: 187.0,
            strength: 4.0,
            gamma: 0.99,
            exposure: 0.0,
            detail: 30.0,
            shadow: 0.0,
            highlight: 0.0,
            vibrance: 0.0,
            saturation: 20.0,
        }
    }
}

fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}

/// Validate the nine controls. The caller maps the reason to its own error
/// type (`FilterError` for the 8-bit filter, `AdjustError` for conversion).
pub fn validate_hdr_toning_params(p: &HdrToningParams) -> Result<(), String> {
    let checks = [
        ("radius", p.radius, 1.0, 500.0),
        ("strength", p.strength, 0.01, 4.0),
        ("gamma", p.gamma, 0.01, 9.99),
        ("exposure", p.exposure, -5.0, 5.0),
        ("detail", p.detail, -100.0, 300.0),
        ("shadow", p.shadow, -100.0, 100.0),
        ("highlight", p.highlight, -100.0, 100.0),
        ("vibrance", p.vibrance, -100.0, 100.0),
        ("saturation", p.saturation, -100.0, 100.0),
    ];
    for (name, value, lo, hi) in checks {
        if !in_range(value, lo, hi) {
            return Err(format!("hdr toning {name} {value} must be in {lo}..={hi}"));
        }
    }
    Ok(())
}
