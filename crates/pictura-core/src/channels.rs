//! Document-level channel budget.
//!
//! A document holds at most [`MAX_CHANNELS`] channels in total, counting its
//! colour channels and its saved-selection/alpha channels. The PSD codec uses
//! the same constant when it writes or reads the header channel count.

use crate::{ColorMode, Document};

/// The most channels a document may hold, colour and extra channels together.
pub const MAX_CHANNELS: u16 = 56;

impl Document {
    /// Whether one more document-level extra channel fits the 56-channel budget.
    ///
    /// The colour count comes from the opened mode (`source_mode` when the file
    /// was converted for editing), because that is what a save emits: a CMYK
    /// source writes four colour channels even though the working composite is
    /// RGB. Channel-owned Multichannel counts its document channels alone.
    /// Legacy Multichannel uses the post-normalization composite count.
    /// An RGBA composite's display alpha is dropped by `save_view` once a
    /// document extra exists, so it counts as three colour channels. A converted
    /// Indexed/Bitmap/Duotone document may save back at the working RGB count;
    /// taking the larger of the two is conservative.
    pub fn can_add_channel(&self) -> bool {
        if self.source.multichannel_plates() {
            return self.channels.len() < MAX_CHANNELS as usize;
        }
        let opened = self.source.mode().unwrap_or(self.mode);
        let colour = if opened == ColorMode::Multichannel {
            let channels = self.composite.channels as usize;
            if channels == 0 {
                return false;
            }
            channels
        } else if self.mode == ColorMode::Rgb
            && self.source.mode().is_none()
            && self.composite.channels == 4
        {
            3
        } else {
            (opened.color_channels() as usize).max(self.composite.channels as usize)
        };
        colour + self.channels.len() < MAX_CHANNELS as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BitDepth, Channel};

    fn document(mode: ColorMode) -> Document {
        Document::new(2, 2, mode, BitDepth::Eight)
    }

    fn add(doc: &mut Document, count: usize) {
        for i in 0..count {
            doc.channels.push(Channel {
                id: i as i16,
                data: vec![0u8; 4].into(),
            });
        }
    }

    #[test]
    fn channel_budget_counts_colour_and_extra_channels() {
        let mut doc = document(ColorMode::Rgb);
        add(&mut doc, 52);
        assert!(doc.can_add_channel(), "3 colour + 52 extras = 55");
        add(&mut doc, 1);
        assert!(!doc.can_add_channel(), "3 colour + 53 extras = 56");
    }

    #[test]
    fn channel_budget_refuses_at_56() {
        let mut doc = document(ColorMode::Rgb);
        add(&mut doc, 53);
        assert_eq!(doc.composite.channels as usize + doc.channels.len(), 56);
        assert!(!doc.can_add_channel());
        let before = doc.channels.len();
        assert_eq!(before, 53);
    }

    #[test]
    fn channel_budget_multichannel_uses_composite() {
        let mut doc = document(ColorMode::Multichannel);
        doc.composite
            .replace(crate::PixelBuffer::new(doc.width, doc.height, 3))
            .expect("three color planes");
        add(&mut doc, 52);
        assert!(doc.can_add_channel(), "3 header channels + 52 extras = 55");
        add(&mut doc, 1);
        assert!(!doc.can_add_channel(), "3 header channels + 53 extras = 56");
    }

    #[test]
    fn channel_budget_counts_the_trimmed_rgba_alpha() {
        let mut doc = Document::from_rgba("rgba", 2, 2, &[0u8; 16]);
        add(&mut doc, 52);
        assert!(
            doc.can_add_channel(),
            "3 colour + 52 extras = 55 after save_view trims the display alpha"
        );
        add(&mut doc, 1);
        assert!(!doc.can_add_channel(), "3 colour + 53 extras = 56");
    }

    #[test]
    fn channel_budget_rejects_a_constructed_multichannel() {
        let doc = document(ColorMode::Multichannel);
        assert_eq!(doc.composite.channels, 0);
        assert!(!doc.can_add_channel());
    }

    #[test]
    fn channel_budget_uses_the_source_mode() {
        let mut doc = document(ColorMode::Rgb);
        doc.source
            .set_encoding(Some(ColorMode::Cmyk), None)
            .expect("valid source encoding");
        add(&mut doc, 51);
        assert!(
            doc.can_add_channel(),
            "4 CMYK source channels + 51 extras = 55"
        );
        add(&mut doc, 1);
        assert!(
            !doc.can_add_channel(),
            "4 CMYK source channels + 52 extras = 56; three working RGB planes must not allow 53"
        );
    }
}
