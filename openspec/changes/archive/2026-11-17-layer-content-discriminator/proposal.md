# Proposal

## Why

Issue [#1344](https://github.com/KookaStudio/KookaPictura/issues/1344) identifies independently writable layer-content fields and repeated precedence decisions across decoding, rendering, rasterization, and the app. Generative fills also inhabit the destructive `Adjustment` enum, making valid content fail `apply` as unsupported. The user selected an authoritative content model rather than a classification-only facade.

## What Changes

- **BREAKING**: Give each layer one authoritative `LayerContent` value covering Pixel, Adjustment, Fill, Text, Shape, SmartObject, and Group. Replace independently writable active-content options with typed access and content transitions.
- Preserve opaque PSD blocks, original adjustment bytes, channels, and raw channel streams separately from active content; normalize conflicting imported candidates without discarding their saved data.
- Make one import/content resolver own the existing compositing precedence. Derive render routes, command eligibility, and row facts from active content; preserve legitimate masks, raster proxies, and opaque fallbacks. Define the existing Fill↔Shape changes during vector-mask edits as atomic content transitions, including file/history/UI state.
- Admit newly introduced raw-copy, descriptor/parser temporary, and retained parameter allocations under the import caller's existing per-allocation/document limits, including hidden layers. Keep malformed-payload preservation distinct from typed resource refusal, and bound cube point accumulation before growth beyond the supported maximum.
- **BREAKING**: Introduce `FillContent` for Solid, Gradient, and Pattern, and remove those variants from `Adjustment`. Every remaining valid adjustment is supported by the 8-bit destructive application contract.
- Keep core content data independent of execution and PSD parsing. Move plain parameter types to core and pure PSD content decoding to codec; retain execution in adjust/render and compatibility re-exports where they do not recreate ambiguous types.
- Preserve existing rendered output, serialized goldens, native-depth support limits, and UI command protocols. Test normalized conflicts, content transitions, history restoration, and save/reopen behavior.

## Capabilities

### New Capabilities

- `document/layer-content`: Authoritative content, centralized normalization and route selection, atomic mask/content transitions, bounded derived-content import, compatible PSD preservation, and shared consumer classification.

### Modified Capabilities

- `imaging/image-adjustments`: Destructive-only `Adjustment`, alpha preservation and validation without generative variants, and relocation of fill models.
- `imaging/adjustment-layers`: Keep opaque adjustment preservation while replacing the public optional active-adjustment slot with authoritative typed content.
- `imaging/adjustment-layer-rendering`: Separate destructive and fill decoding/results without changing existing descriptor layouts, generated samples, or fill encoders.
- `compositing/layer-compositing`: Decode vector fill blocks into `FillContent` and preserve strict adjustment-block precedence and CPU fallback.
- `compositing/layer-management`: Express fill rasterization through the active fill content type while preserving the existing Rasterize subset and bake behavior.
- `tools/type-layer-kind`: Project type identity from normalized active content while retaining opaque `TySh` detection during import, saved bytes, and existing type-lock rules.

## Impact

The Rust model is intentionally source-breaking: `Layer` construction and content mutations throughout the workspace must migrate together. Affected areas include core/layer stacks and history, adjust types/application, codec read/write and content decoders, renderer plans/GPU eligibility/effects/source caches/layer operations, and app row facts and command eligibility. Stable C++ bridge strings and command IDs remain compatibility projections; no GUI redesign is proposed. The mandatory test-plan parser also needs a focused correction so an untagged scenario ending in an inline-code name can retain that name when matched.

No external dependencies, new PSD tags, fixture regeneration, golden replacement, pixel algorithms, or expansion of supported adjustment/filter/depth kinds are proposed. The long-form `docs/` contract remains unchanged. Implementation will update `ROADMAP.md`, validate the test plan, and archive this change in the shipping PR; this proposal leaves the change open for review.
