//! Advanced-blending layer model: knockout mode and the Blend If view.

/// PSD `knko` knockout mode. Values match psd-tools `Knockout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Knockout {
    #[default]
    None = 0,
    Shallow = 1,
    Deep = 2,
}

impl Knockout {
    /// Map a `knko` payload byte; any value outside 0/1/2 is [`Knockout::None`].
    pub fn from_byte(v: u8) -> Knockout {
        match v {
            1 => Knockout::Shallow,
            2 => Knockout::Deep,
            _ => Knockout::None,
        }
    }

    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// A typed view of a layer's blending-ranges body: composite source and
/// destination `(black, white)` ranges (first 8 bytes), then zero or more
/// 8-byte channel groups of source + destination pairs. The raw
/// [`crate::Layer::blending_ranges`] field remains the serialization source
/// when unmodified.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct BlendIf {
    pub composite_source: (u16, u16),
    pub composite_dest: (u16, u16),
    pub channel_ranges: Vec<((u16, u16), (u16, u16))>,
}

impl BlendIf {
    /// True when every range is the full `(0, 65535)` default, making the view
    /// a no-op gate.
    pub fn is_default(&self) -> bool {
        const FULL: (u16, u16) = (0, 65535);
        self.composite_source == FULL
            && self.composite_dest == FULL
            && self
                .channel_ranges
                .iter()
                .all(|(source, dest)| *source == FULL && *dest == FULL)
    }
}
