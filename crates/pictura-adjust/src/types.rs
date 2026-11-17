#[derive(Debug, thiserror::Error)]
pub enum AdjustError {
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}


pub use pictura_core::*;

pub(crate) fn validate_levels(record: &LevelsChannel) -> Result<(), AdjustError> {
    if !record.gamma.is_finite() || record.gamma <= 0.0 {
        return Err(AdjustError::InvalidParams("levels gamma must be > 0".into()));
    }
    if record.input_black >= record.input_white {
        return Err(AdjustError::InvalidParams("levels input black must be below input white".into()));
    }
    Ok(())
}

fn eval_levels(record: &LevelsChannel, x: f64) -> f64 {
    let (ib, iw) = (record.input_black as f64, record.input_white as f64);
    let (ob, ow) = (record.output_black as f64, record.output_white as f64);
    let t = ((x - ib) / (iw - ib)).clamp(0.0, 1.0).powf(1.0 / record.gamma);
    ob + t * (ow - ob)
}

pub(crate) fn levels_map(record: LevelsChannel) -> Result<impl Fn(f64) -> f64, AdjustError> {
    validate_levels(&record)?;
    Ok(move |u: f64| eval_levels(&record, u * 255.0) / 255.0)
}

pub(crate) fn levels_lut(record: &LevelsChannel) -> Result<[u8;256], AdjustError> {
    validate_levels(record)?;
    let mut lut = [0u8;256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = eval_levels(record, i as f64).round().clamp(0.0,255.0) as u8;
    }
    Ok(lut)
}
