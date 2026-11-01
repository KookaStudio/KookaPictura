use crate::error::PsdError;

pub(crate) const SIGNATURE: u32 = 0x3842_5053; // "8BPS"
pub(crate) const VERSION_PSD: u16 = 1;
pub(crate) const VERSION_PSB: u16 = 2;

pub(crate) const MODE_BITMAP: u16 = 0;
pub(crate) const MODE_GRAYSCALE: u16 = 1;
pub(crate) const MODE_INDEXED: u16 = 2;
pub(crate) const MODE_RGB: u16 = 3;
pub(crate) const MODE_CMYK: u16 = 4;
pub(crate) const MODE_MULTICHANNEL: u16 = 7;
pub(crate) const MODE_DUOTONE: u16 = 8;
pub(crate) const MODE_LAB: u16 = 9;

pub(crate) const MAX_CHANNELS: u16 = 56;
pub(crate) const MAX_DIM_PSD: u32 = 30_000;
pub(crate) const MAX_DIM_PSB: u32 = 300_000;

pub(crate) const SECTION_DIVIDER: u32 = 3;
pub(crate) const SECTION_OPEN_FOLDER: u32 = 1;
pub(crate) const SECTION_CLOSED_FOLDER: u32 = 2;

pub(crate) const COMPRESSION_RAW: u16 = 0;
pub(crate) const COMPRESSION_RLE: u16 = 1;
pub(crate) const COMPRESSION_ZIP: u16 = 2;
pub(crate) const COMPRESSION_ZIP_PREDICTION: u16 = 3;

pub(crate) const DIVIDER_NAME: &str = "</Layer group>";

/// The additional-layer-information keys whose length field widens to `u64` in
/// a version-2 PSB. This is the exact psd-tools `TaggedBlock._BIG_KEYS` set
/// (`psd/tagged_blocks.py`): its `_length_format` returns `Q` when
/// `version == 2 and key in _BIG_KEYS`, `I` otherwise.
const PSB_BIG_KEYS: [[u8; 4]; 21] = [
    *b"Alph", *b"FELS", *b"FEid", *b"FMsk", *b"FXid", *b"LMsk", *b"Layr", *b"Lr16", *b"Lr32",
    *b"Mt16", *b"Mt32", *b"Mtrn", *b"PxSD", *b"artd", *b"cinf", *b"extd", *b"extn", *b"lnk2",
    *b"lnk3", *b"lnkE", *b"pths",
];

pub(crate) fn is_psb_big_key(key: &[u8; 4]) -> bool {
    PSB_BIG_KEYS.contains(key)
}

/// The document-level block Photoshop stores layer info in at `depth`, in place
/// of the layer-info subsection; `None` at depths that use the subsection.
pub(crate) fn tagged_layer_key(depth: u16) -> Option<&'static [u8; 4]> {
    match depth {
        16 => Some(b"Lr16"),
        32 => Some(b"Lr32"),
        _ => None,
    }
}

/// Additional-layer-info keys that carry an adjustment.
///
/// The brief's list plus the spellings the reference actually writes: Invert is
/// `nvrt` (not `invr`) and the legacy Hue/Saturation key is `hue ` alongside
/// `hue2`. Both spellings are accepted on read. `SoCo` is solid-color fill
/// content, `GdFl` is gradient fill content, and `PtFl` is pattern fill content:
/// all three are preserved verbatim here, and only `pictura-render` decodes the
/// subsets it understands.
pub(crate) const ADJUSTMENT_KEYS: [[u8; 4]; 22] = [
    *b"levl", *b"curv", *b"brit", *b"expA", *b"vibA", *b"hue2", *b"hue ", *b"blwh", *b"phfl",
    *b"blnc", *b"mixr", *b"grdm", *b"gdrm", *b"invr", *b"nvrt", *b"post", *b"thrs", *b"selc",
    *b"clrL", *b"SoCo", *b"GdFl", *b"PtFl",
];

pub(crate) fn is_adjustment_key(key: &[u8; 4]) -> bool {
    ADJUSTMENT_KEYS.contains(key)
}

/// Cursor over the file bytes. Every read is bounds-checked, so malformed input
/// yields [`PsdError::Truncated`] instead of an index panic.
pub(crate) struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], PsdError> {
        if self.remaining() < n {
            return Err(PsdError::Truncated);
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, PsdError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, PsdError> {
        let s = self.take(2)?;
        Ok(u16::from_be_bytes(
            s.try_into().map_err(|_| PsdError::Truncated)?,
        ))
    }

    pub(crate) fn i16(&mut self) -> Result<i16, PsdError> {
        Ok(self.u16()? as i16)
    }

    pub(crate) fn u32(&mut self) -> Result<u32, PsdError> {
        let s = self.take(4)?;
        Ok(u32::from_be_bytes(
            s.try_into().map_err(|_| PsdError::Truncated)?,
        ))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, PsdError> {
        Ok(self.u32()? as i32)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, PsdError> {
        let s = self.take(8)?;
        Ok(u64::from_be_bytes(
            s.try_into().map_err(|_| PsdError::Truncated)?,
        ))
    }

    pub(crate) fn skip(&mut self, n: usize) -> Result<(), PsdError> {
        self.take(n).map(|_| ())
    }
}
