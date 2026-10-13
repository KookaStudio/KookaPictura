//! Typed view of the fixed `crs:` XMP property set of an embedded raw source.

/// The typed `crs:` XMP tone/colour scalars of an embedded raw source.
///
/// Only the fixed PV2012 property set is lifted; every other `crs:` key stays
/// inside the raw packet. A field is `None` when the packet omits it or its
/// value is non-finite/unparsable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CrsSettings {
    pub exposure: Option<f64>,
    pub contrast: Option<f64>,
    pub highlights: Option<f64>,
    pub shadows: Option<f64>,
    pub whites: Option<f64>,
    pub blacks: Option<f64>,
    pub clarity: Option<f64>,
    pub vibrance: Option<f64>,
    pub saturation: Option<f64>,
    pub temperature: Option<f64>,
    pub tint: Option<f64>,
}
