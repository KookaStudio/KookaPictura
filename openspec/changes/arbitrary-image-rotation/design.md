## Context

The Image menu calls the existing cancellable whole-document rotation bridge. Crop already uses the same renderer and keeps a fixed bilinear engine entry point. Free Transform supplies the six interpolation IDs and the existing cubic kernels.

## Goals / Non-Goals

- Support two-decimal signed angles and an independent clockwise or counterclockwise direction. Prefill a positive magnitude with the corresponding direction.
- Read the ruler endpoints from the active PictureView, including when another tool is selected. Reuse Crop's nearest-axis correction, measured in document coordinates.
- Persist an Image Interpolation choice in the General preferences store. Applying it to Image Size or Free Transform is outside #567.

## Decisions

The dedicated dialog uses standard Qt Widgets and a localized numeric field. The final clockwise angle equals the signed field value for CW and its negation for CCW. The dialog refuses blank and invalid text, and the handler treats zero as a no-op.

The renderer takes the typed interpolation on a new entry point and carries it through the existing tiled inverse map. Bilinear keeps its existing arithmetic. Nearest and cubic reuse the transform kernel on the tile read window. The window already includes padding for the cubic footprint. Out-of-source pixels keep the existing zero fill. Bicubic Automatic resolves to Bicubic for a rotation at unit scale. For exact centre quarter turns without vector masks, retain the existing exact remap. When a vector mask requires the existing geometry path to transform its vertices, use nearest typed sampling for the raster planes so their pixels receive the same exact index mapping, independent of the preference. Keep the fixed-angle commands unchanged and keep angle-only arbitrary calls bilinear for non-right angles.

General preferences use the default Bicubic when the field is absent or unsupported. The menu snapshots the preference for each command, and stores the ID alongside the signed angle when recording. Existing playback or direct command callers that provide only the angle retain bilinear. Explicit unrecognized interpolation IDs are refused. New recorded steps persist the ID in the existing Pictura JSON sidecar of the Adobe Rtte descriptor, so a Pictura .atn save and reload preserves it. Imported Adobe angle-only steps have no ID and retain bilinear behavior; the sidecar does not claim Adobe will honor the chosen kernel.

## Risks / Trade-offs

Exact CS6 interpolation and the default for Arbitrary are inferred from the docs. The cubic kernels follow Pictura's existing approximations. Existing mode and layer-kind limitations and fill rules stay in effect.
