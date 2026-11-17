use std::mem::size_of;

use crate::budget::DecodeBudget;
use crate::common::Reader;
use crate::{DescValue, PsdError};

pub(crate) struct ContentAlloc<'a> {
    budget: Option<&'a mut DecodeBudget>,
    error: Option<PsdError>,
}

impl<'a> ContentAlloc<'a> {
    pub(crate) fn new(budget: Option<&'a mut DecodeBudget>) -> Self {
        Self {
            budget,
            error: None,
        }
    }

    pub(crate) fn vec<T>(&mut self, count: usize, what: &str) -> Option<Vec<T>> {
        if self.error.is_some() {
            return None;
        }
        match self.budget.as_deref_mut() {
            None => Some(Vec::with_capacity(count)),
            Some(budget) => match vec_with_capacity(count, budget, what) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.error = Some(error);
                    None
                }
            },
        }
    }

    pub(crate) fn push<T>(&mut self, values: &mut Vec<T>, value: T, what: &str) -> Option<()> {
        if values.len() == values.capacity() {
            let Some(capacity) = values.capacity().checked_mul(2) else {
                self.error = Some(PsdError::Unsupported(format!(
                    "{what} exceeds decode budget address space"
                )));
                return None;
            };
            let capacity = capacity.max(4);
            let mut replacement = self.vec(capacity, what)?;
            replacement.append(values);
            *values = replacement;
        }
        values.push(value);
        Some(())
    }

    pub(crate) fn string(&mut self, value: &str, what: &str) -> Option<String> {
        if self.error.is_some() {
            return None;
        }
        match self.budget.as_deref_mut() {
            None => Some(value.to_string()),
            Some(budget) => match copy_string(value, budget, what) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.error = Some(error);
                    None
                }
            },
        }
    }

    pub(crate) fn cube(&mut self, bytes: &[u8]) -> Option<CubePoints> {
        if self.error.is_some() {
            return None;
        }
        match self.budget.as_deref_mut() {
            None => parse_cube_unbudgeted(bytes),
            Some(budget) => match parse_cube(bytes, budget) {
                Ok(value) => value,
                Err(error) => {
                    self.error = Some(error);
                    None
                }
            },
        }
    }

    // Keep resource errors separate while reusing optional field parsers.
    pub(crate) fn finish<T>(self, value: Option<T>) -> Result<Option<T>, PsdError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(value),
        }
    }
}

pub(crate) fn vec_with_capacity<T>(
    count: usize,
    budget: &mut DecodeBudget,
    what: &str,
) -> Result<Vec<T>, PsdError> {
    budget.charge(what, &[count, size_of::<T>()])?;
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| {
        PsdError::Unsupported(format!(
            "{what} allocation refused after decode budget admission"
        ))
    })?;
    Ok(values)
}

#[cfg(test)]
pub(crate) fn copy_payload(bytes: &[u8], budget: &mut DecodeBudget) -> Result<Vec<u8>, PsdError> {
    let mut copy = vec_with_capacity(bytes.len(), budget, "content payload copy")?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

pub(crate) fn copy_string(
    value: &str,
    budget: &mut DecodeBudget,
    what: &str,
) -> Result<String, PsdError> {
    budget.charge(what, &[value.len()])?;
    let mut copy = String::new();
    copy.try_reserve_exact(value.len()).map_err(|_| {
        PsdError::Unsupported(format!(
            "{what} allocation refused after decode budget admission"
        ))
    })?;
    copy.push_str(value);
    Ok(copy)
}

pub(crate) fn read_descriptor(
    bytes: &[u8],
    budget: &mut DecodeBudget,
) -> Result<Option<DescValue>, PsdError> {
    crate::descriptor::read_descriptor_with_budget(&mut Reader::new(bytes), budget)
}

type CubePoints = (usize, Vec<[f32; 3]>);

pub(crate) fn parse_cube(
    bytes: &[u8],
    budget: &mut DecodeBudget,
) -> Result<Option<CubePoints>, PsdError> {
    let Some((size, count)) = walk_cube(bytes, |_| {}) else {
        return Ok(None);
    };
    let mut points = vec_with_capacity(count, budget, "content LUT points")?;
    if walk_cube(bytes, |point| points.push(point)).is_none() {
        return Ok(None);
    }
    Ok(Some((size, points)))
}

pub(crate) fn parse_cube_unbudgeted(bytes: &[u8]) -> Option<CubePoints> {
    let (size, count) = walk_cube(bytes, |_| {})?;
    let mut points = Vec::with_capacity(count);
    walk_cube(bytes, |point| points.push(point))?;
    Some((size, points))
}

fn walk_cube(bytes: &[u8], mut point: impl FnMut([f32; 3])) -> Option<(usize, usize)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut size: Option<usize> = None;
    let mut count = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("TITLE") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
            let n = rest.trim().parse().ok()?;
            if !(2..=64).contains(&n) {
                return None;
            }
            size = Some(n);
            continue;
        }
        if line.starts_with("DOMAIN_MIN") || line.starts_with("DOMAIN_MAX") {
            continue;
        }
        size?;
        if count == 64 * 64 * 64 {
            return None;
        }
        let mut parts = line.split_whitespace();
        let mut rgb = [0.0f32; 3];
        for component in &mut rgb {
            *component = parts.next()?.parse().ok()?;
            if !component.is_finite() {
                return None;
            }
        }
        if parts.next().is_some() {
            return None;
        }
        count += 1;
        point(rgb);
    }
    let size = size?;
    (count == size * size * size).then_some((size, count))
}

#[cfg(test)]
mod tests;
