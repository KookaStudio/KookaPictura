# Design: smart-filter-apply

## The generic record (codec)

`pictura_codec::pictura_filter` follows the Lens Correction module. The
`filterID` is `0x5054_464C` ("PTFL"), Pictura-only: no cross-app interop is
claimed. The `Fltr` descriptor (class `Pictura Filter`) holds:

| Key | Type | Meaning |
|---|---|---|
| `Vrsn` | Long | record version, 1 |
| `Glry` | Bool | the entry is a Filter Gallery stack |
| `Stck` | List of Object | the steps in apply order |
| `Stck[].Kind` | Text | the catalogue kind id |
| `Stck[].Prms` | List of Double | the kind's full-resolution `commit_params` slots |

A missing key, a wrong type, or an empty stack decodes to `None`. The entry
name is the catalogue label without its ellipsis, or "Filter Gallery".

## Eligibility

`catalogue::smart_capable(id)` is false for `liquify`, `vanishing-point`,
`lens-blur`, `displace`, `adaptive-wide-angle` (CS6 refuses the first two on a
Smart Object; the other three read a session resource that a stored record
cannot name) and for `hdr-toning` (an Image > Adjustments command). A guard
test builds every kind with a recording `FilterResources` and fails when a kind
that touches a resource is smart-capable.

## Render

`SmartFilterOp::Filters(Vec<Filter>)` is decoded from a PTFL record with
`catalogue::build(.., &NullResources)` only when every step is smart-capable,
so an excluded or unknown kind leaves the record undecodable (the layer falls
back to its proxy). Each filter runs on a scratch layer through
`crate::filter::apply_filter` on the CPU, so the alpha handling matches the
destructive path; GPU is not an oracle (determinism rule).

`document_ops::smart_filter_apply`:

- `smart_filter_record(steps, gallery) -> Option<SmartFilter>` builds the
  record and validates it through `decode_smart_filter`.
- `apply_smart_filter(doc, path, filter, selection) -> bool` refuses an object
  that fails `can_replace_smart_object_contents`, appends the filter, and when
  the object has no mask and a selection is given, makes the selection the
  mask (`filter_mask_enabled = true`). An existing mask is kept (inferred: the
  CS6 Help does not say). The proxy is not touched.

## Codec append and mask flag

`attach_smart_filter` and `insert_into_config` share `attach(.., replace)`.
`append_smart_filter` always pushes to both the preserved `filterFXList` and
the typed `smart_filters`, so their indices stay aligned for the panel
operations. `set_filter_mask_enabled` rewrites the preserved
`filterFX.filterMaskEnable` and the typed flag.

## The filter mask on save

`filter_effects::author_filter_masks(extra, layers, color_channels, psb)` runs
at save time, because a converted object's uuid is `author_uuid(so)`, which
only exists once its `SoLd` is authored. For each smart object with a
`filter_mask`:

- the key is `author_uuid` when its `SoLd` is authored this save, else
  `placed_uuid`, falling back to `uuid`;
- an existing record with the same rect and coverage is kept verbatim;
- a changed or new record replaces or appends one version-3 entry: rect, depth
  8, the colour channels unwritten, and the coverage in the last channel with
  ZIP-with-prediction, matching the reader.

An `FMsk` block is added only when none exists; its colour and opacity are
unverified defaults. This also restores a saved mask after an undone Delete
Filter Mask, which drops the `FEid` entry but not the typed mask on undo.

ponytail: the authored `FEid` has no rendered RGB cache, and Photoshop has not
been asked to open it; interop is unverified (a follow-up issue).

## App routing

`filter_tools/smart_apply.rs` holds the routing so the near-cap files only gain
a hook each:

- `smart_filter_target(rust)`: the single active layer, no mask target, and
  `can_replace_smart_object_contents`. Lens Correction's target and apply move
  onto it (Lens Correction keeps its replace semantics for re-edit).
- `smart_refusal(kind, rust)`: on a Smart Object an excluded kind is refused
  with "Rasterize the Smart Object to apply X".
- `smart_preview(..)`: keeps `PreviewBaseline::Layer(layer.clone())`, then
  pushes the record (and a mask from the selection) onto the typed view only;
  Cancel restores the whole layer, and the compositor's `chain_hash` key
  invalidates the cached chain.
- `commit_smart(view, path, record, label)`: restore the preview →
  `apply_smart_filter` → one `Edit::Structural` state → `last_filter`.

Hooks: the `apply_filter_active_region` preview branch (covers
`filter_preview`, `filter_preview_section`), `apply_filter_params`,
`filter_commit_start_inner` (synchronous on a Smart Object), the gallery commit
and replay (`filter_stack_commit_start_inner`, `run_stack`), and the control
server's `PictureView::apply_filter` (the kind's defaults).
`canvas_proxy_buffer` and `exact_preview_start` decline on a Smart Object, so
the dialog falls back to the section preview.

Safety net: `pixel_filter_target` refuses any layer with a smart object ("the
smart object is not directly editable"), covering linked and legacy objects,
HDR Toning, and the adjustments.

Menus: `filter_document_takes` also requires no `smart_refusal`, greying the
excluded filters; a new `filter_target_smart(view)` greys Image > Adjustments.

## Risks

- The chain renders synchronously on the UI thread, so a large Smart Object is
  slow; Lens Correction accepts the same cost.
- The panel lists filters in apply order (index 0 first) and the existing
  panel already draws them that way; this change does not reorder.
