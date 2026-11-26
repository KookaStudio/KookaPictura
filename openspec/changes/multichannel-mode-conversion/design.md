# Design

## Context

The engine displays an 8-bit working composite and retains native source planes separately. The Channels panel already reads named document channels and spot metadata. Legacy one- and three-plane 8-bit Multichannel files use a fixed grayscale or CMY preview.

## Goals / Non-Goals

Produce named independent plates, flatten visible artwork in the conversion transaction, preserve existing extra channels, and round-trip the resulting mode at 8 or 16 bits. Duotone curve interpretation, delete-to-convert, and DCS output are separate workflows.

## Decisions

- Store authored plates in the existing document channel list so rows read their real samples. Put the process plates before existing extras, keeping names and display metadata aligned. Use Cyan/Magenta/Yellow for RGB, add Black for CMYK, use Lightness/a/b for Lab, and use Black for Grayscale. Authored plate samples use low values for ink and high values for no ink; retain the white/no-ink RGB component values and matching native precision.
- Use the source composite when already flat and composite the visible layer tree over white otherwise. For CMYK and Lab, use valid retained samples when they match the working image and otherwise use the existing encoders. Keep matching native samples at 16 bits, and widen rebuilt 8-bit samples consistently with the existing engine precision ceiling.
- Write authored documents as flat mode-7 PSD/PSB. Write the document channel list directly and reuse the native store when its samples still match the channel view. Record an explicit SourceState distinction between legacy composite-backed Multichannel and channel-owned Multichannel. A valid mode-7 DisplayInfo or channel-name resource identifies channel-owned files; retain every plane as a document channel with aligned names and display metadata. Without either resource accept only legacy 8-bit one- and three-plane files, retaining their old rendering and write fallback; refuse other unnamed layouts.
- Reconstruct an opaque display preview from the stored plate values and spot colours. Spot display is an approximation, and the stored plate samples are authoritative for interchange. Report unsupported input before changing the document.
- Route both the menu and action playback through the mode command so flattening and source-state replacement occupy one undo step. Channel-owned plates use the existing extra-channel rows and alpha/spot selection handling, with no fabricated colour rows; Load Selection uses each stored channel once. Reject layer insertion while channel-owned Multichannel is active. Conversion back to RGB safely materializes the display preview as the RGB composite, clears the channel-owned state, and retains every named plate and every pre-existing extra exactly once as a normal extra channel; fabricated mode-7 rows disappear.

- Rebuild the preview after spot coverage changes and after geometry operations that remap the plates. Canvas growth pads spot plates with no ink and saved selections with zero coverage. Arbitrary-angle rotation and perspective crop require a layer-based path and are unavailable while the document stores its image as plates.

## Risks / Trade-offs

The current working compositor is 8-bit, so visibly recomposed regions without corresponding native source samples widen from the working result. Displayed spot colours are a preview without a press simulation.
