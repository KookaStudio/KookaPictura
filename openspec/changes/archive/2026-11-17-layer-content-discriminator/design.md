# Design

## Context

See `proposal.md` for motivation and the confirmed scope. The existing seams are:

- `pictura-core/src/lib.rs`: `Layer` exposes `adjustment`, `is_group`, `children`, `smart_object`, and `type_tool` independently. `channels` can be editable pixels or a proxy. `vector_mask` is an attribute; its presence alone is not a content identity. Raw adjustment bytes, `extra_blocks`, and `raw_channels` are serialization inputs.
- `pictura-adjust/src/types.rs` and `apply.rs`: `Adjustment` is documented as destructive but includes three refused generative fills. Most plain parameters live here; Match Color and Replace Color declare their parameters alongside their kernels. `LevelsChannel::validate` currently returns `AdjustError`, so moving data also requires separating that validation implementation.
- `pictura-render/src/composite_route.rs`: one private precedence chain already coordinates `CompositePlan` and GPU eligibility. Its `Fill(Adjustment)` arm includes destructive adjustments. Its `Pixels` route can describe a raster-backed Text or SmartObject. Smart filters, stack modes, and the channel-0 proxy predicate matter to routing.
- `pictura-render/src/fill.rs` and `composite.rs`: PSD fill and destructive decoders currently return the same enum. A decodable `vscg` is considered before smart/text sources when the primary adjustment slot is absent. A present, undecodable primary block blocks fallback.
- Codec read assembles records/channels/tree, then resolves smart objects and type tools from preserved descriptors. Codec write re-emits original adjustment and extra-block bytes; type and smart views are currently derived rather than independent serialization sources.
- Rasterization, shape helpers, and `pictura-app/src/cxxqt_object/helpers.rs` implement additional classifications. Legacy row strings group several kinds together; the snapshot carries separate shape/smart-object facts. `tools/type-layer-kind` intentionally recognizes even malformed `TySh` by presence.

The graph index does not cover this worktree and reports unavailable freshness; the observations above were checked against the current source. The issue's 146-call and 64-fill-match counts are historical scope estimates, not verified acceptance counts.

## Goals / Non-Goals

**Goals:** Enforce a single active value through the public model; preserve all existing render routes and unchanged PSD serialization; make fill versus destructive execution a type distinction; put content knowledge behind a small public interface; migrate producers and consumers together.

**Non-Goals:** A new node arena, layer identifier scheme, content registry, trait hierarchy, renderer, PSD format, UI protocol, supported adjustment/filter set, or native-depth expansion. No pixel math or fixture/golden regeneration. Long-form `docs/` stays untouched.

## Decisions

### 1. The active enum owns payloads; it is not a cached kind flag

Add core's `layer_content` module and store one private content value in `Layer`. The proposed data shape is:

```text
Layer
  content: LayerContent
    Pixel         editable raster data
    Adjustment    supported parameters or preserved undecodable primary block
    Fill          supported FillContent or preserved undecodable fill definition
    Text          parsed text or opaque TySh definition, plus optional proxy
    Shape         fill definition and existing shape appearance/geometry
    SmartObject   source/filter data, plus optional proxy
    Group         ordered children
  attributes      rect, blend, opacity, locks, masks, effects-related data
  preservation    original PSD content representation and opaque data
```

This is a shape sketch, not an additional public type hierarchy. Reuse the existing `LayerStack`, channel planes, text, smart-object, vector-mask, and appearance types. Keep attributes common where current operations require them; in particular raster/vector masks must remain usable on ordinary pixels and other content. Move children into Group content or expose them only through an accessor that cannot create a non-group with active children. Preserve layer-stack identity and copy-on-write sharing.

Expose a borrowed `content()` view, read-only kind/typed accessors derived from it, and validated replacement/content-edit operations. Compatibility getters must derive from that value; there is no public writable mirror of the old options. Mutable payload access must not bypass serialization synchronization: edit through the codec/render authoring operation, stage the new value and affected encoded representation, then install both. Raw preservation access is read-only outside import/serialization machinery. A constructor or replacement validates payload-kind compatibility before mutation.

Keep original PSD channels exactly once with shared plane storage: a Pixel owns editable pixels, other kinds can own raster proxies, and preservation retains raw unmodeled streams and source-depth channels. Channel access used for rendering reads through this ownership instead of retaining a second writable `Layer.channels` mirror.

**Alternative rejected:** Adding `kind` beside the current public options creates a new invalid state: the flag and payloads can disagree. A computed view centralizes precedence but cannot satisfy the user's construction invariant.

### 2. Core owns plain content types, not execution or parsers

Move the `Adjustment` data enum and its plain parameter records to core without adding the fill variants. Move the Match Color statistics/parameters and Replace Color samples/parameters needed by that enum; keep all statistics accumulation, color conversion, and application kernels in adjust. Define `FillContent::{Solid, Gradient, Pattern}` and its plain parameters in core. Preserve defaults and field precision. Re-export moved parameter types from adjust where this reduces caller churn without preserving the removed variants. The core test checks manifests and core-only usability; a separate downstream adjust integration test consumes both core types and adjust re-exports to prove their type identity, without adding even a test-only execution dependency to core.

Keep `AdjustError` in adjust, with its existing errors. Convert validation methods tied to that error into adjust-local functions over the moved records instead of introducing an execution error into core. No dependency from core to adjust, codec, color execution, render, or Qt is needed; no new external dependencies are needed.

Move pure PSD content decoders to a codec `layer_content` concern. Reuse descriptor primitives and the existing profile-free conversions; separate parser helpers currently entangled with renderer modules by pure moves before changing dispatch. A pure parser such as the embedded Color Lookup cube reader moves with decoding when necessary; adjustment execution still consumes its core data. `decode_adjustment` returns only destructive parameters; `decode_fill` returns `FillContent`; vector fill parsing uses the same fill decoders. Preserve existing public render encoder entry points and use render re-exports for unchanged decoder entry points where possible. The fill return-type change is intentionally breaking.

**Alternative rejected:** Putting a typed `Adjustment` in core while leaving its definition in adjust creates a dependency cycle. Adding a generic payload trait or a new intermediate crate is unnecessary for this closed set of content.

### 3. Import normalizes once; route resolution does not redefine identity

Codec assembles a private candidate record while reading, including raw bytes, parsed views, channels/proxies, and tree structure. After the required document-level linked records and text data are available, a single normalization function installs active content. Route priority is preserved exactly:

```text
group
  --> decoded primary adjustment/fill
  --> decoded vscg only if primary block absent
  --> undecodable primary adjustment (no-op)
  --> supported enabled smart filters
  --> applicable stack-mode source / smart source without proxy
  --> parsed type source without proxy
  --> undecodable vscg
  --> pixels
```

Semantic identity is retained when execution uses a proxy. A supported primary fill becomes Fill or Shape using the existing shape-recognition policy: a decodable fill plus an effective raw vector-mask key is Shape. `vmsk` wins over `vsms` by presence, even if malformed; disabling the mask or failing to parse its view does not remove Shape identity. Do not infer or change the saved pixel-data-irrelevant flag. Preserve the distinction between editable shape recognition and that on-disk flag. Text candidates recognize raw `TySh` even if parsing fails; such content can use preserved pixels. SmartObject identity remains SmartObject with a proxy. Do not turn an undecodable primary block into pixels because its parameters are unavailable.

Vector masks are attributes except where the existing fill/shape relationship requires an explicit content transition. Successful mask addition on decodable Fill installs Shape with the same fill and the effective geometry/tags. Successful deletion or rasterization of the final effective mask definition installs Fill with the same fill. Keep Pixel content Pixel. Update content and mask data atomically: deletion reveals the unmasked fill, whereas `rasterize_vector_mask` keeps its current vector-times-raster coverage in the raster mask. Remove the vector-mask definitions according to the existing command, retire the active Shape geometry, and preserve its other attributes, saved flags, and unrelated `vogk`/`vstk`/style/opaque blocks according to current behavior; do not invent extra cleanup. A refused operation, including a malformed mask with no usable parsed view, changes nothing. Save/reopen, row badges/forced-lock facts, command eligibility, and undo/redo must observe the same completed transition.

Raw conflicting candidates remain in preservation. If a legacy smart/text source fallback needs a losing parsed candidate, the content-resolution module can derive that fallback from the retained data; it is not independently editable active content. Do not discard these inputs and accidentally change the established failure/fallback result. The renderer is the only consumer of the resulting fallback route, rather than each caller walking preserved fields.

Refactor `content_route` into the public content-resolution entry point, exhaustively matching the active enum. Keep smart-filter executability and stack applicability in render because they depend on render/filter support. Return separate Adjustment and Fill routes; retain group subroutes, undecodable cases, and GPU rejection provenance. CPU plan construction and GPU eligibility consume this same route. Cache/plan once per resolved layer as now; do not parse descriptors per sample or per band.

**Alternative rejected:** Using the current nine render variants as semantic identity mislabels raster-backed text/smart objects and conflates backdrop transforms with generated sources. Keeping an old fallback precedence chain beside the new function creates two authorities.

#### Admission for new import-time decoding

Thread the existing import `DecodeBudget` into the new normalization/decode path. Its budget-aware entry point returns `Result<_, PsdError>` and carries resource failure separately from the supported/undecodable result; do not run the import through the existing `Option` compatibility helpers or their `.ok()?` error erasure. Public compatibility helpers can keep their current signatures for existing non-import callers. Keep this as explicit parser/normalizer parameters, not a new generic decoder framework.

Charge before every newly introduced heap allocation: any raw-payload copy made to feed the moved parser, DOM object/list/text/raw-data buffers, parser work vectors, and retained typed stores such as LUT points. Borrow original payload slices and share existing planes where possible; aliases do not require a new allocation, but a real copy must be admitted separately. Apply `ImageBudget.max_alloc_bytes` and the same remaining `max_document_bytes` counter used by `DecodeBudget`, including hidden layers. Use its existing conservative cumulative-claim policy for temporary allocations during the read as well as retained allocations; temporary claims are not refunded during normalization. A moved allocation is charged once, not charged again merely because its ownership becomes retained. Many individually valid LUTs therefore still consume the shared document total. The budget-aware descriptor reader must check lengths/counts and admit DOM/raw/text/list storage before allocation, rather than building an unbudgeted DOM and charging only its final typed result. Account for peak reallocation/new capacity before it is requested; no new tiled-channel materialization is permitted.

The cube reader rejects an excess numeric row before growing its point vector past the supported global maximum `64^3`. Keep the existing final declared-size/exact-count test, including currently accepted later `LUT_3D_SIZE` declarations; using the first declared size as an early bound would tighten accepted input unnecessarily. If an overlong cube fits the allocation policy, it keeps the existing malformed lookup result (`ColorLookup` with no parsed lookup) and original bytes. If raw/DOM/work/retained storage would exceed either resource limit first, return the typed resource error; do not reinterpret it as `None`, a harmless opaque adjustment, or an omitted hidden layer. Valid admitted payloads retain their current parameters and serialization. Test the supported maximum, excess rows, malformed-versus-budget distinction, temporary-DOM allocation, and multiple hidden valid LUTs under small explicit budgets.

### 4. Imported bytes remain the unchanged-save source; edits are atomic

Imported known and unknown descriptor bytes remain authoritative for an unchanged write. Normalization never rewrites a descriptor just because decoding succeeded. The preservation object stores the raw primary block separately from its normalized active content and retains original extra blocks, raw channel streams, and source-channel metadata. This preserves unknown descriptor fields and current encounter/encoding order.

An authored edit stages supported parameters plus the exact affected encoded blocks, then commits through the content transition. Keep unrelated opaque blocks. Existing encoders still implement the current descriptor layouts, clamping, omitted defaults, and legacy four-byte solid-fill read compatibility. For text/smart edits, reuse the existing block-editing machinery and include its document-level linked data in the staged document operation.

Rasterization installs Pixel, retains masks/attributes according to that specific command, and deletes the obsolete fill/type/smart/shape definitions that could restore the old kind after reopen. For a conflicting import, cleanup includes the losing content-defining tags that would otherwise become newly active. It does not indiscriminately erase unknown metadata. Existing Type and Smart Object rasterize implementations remain enabled; the older Rasterize subset wording is qualified in its delta rather than used to disable later features.

History snapshots/clones include active content and its preservation together. Retain plane/large-payload sharing; compare semantic and saved content rather than cache metadata. No persisted migration/version marker is needed, since the on-disk format is unchanged.

**Alternative rejected:** Serializing typed parameters for every unchanged layer loses unknown fields and changes goldens. Preserving every retired content tag after rasterization can recreate the retired content on reopen.

### 5. Migrate consumers by responsibility, not by replacing every field read

Replace independent kind/precedence decisions in composite plans, CPU/native paths, GPU eligibility, effects/source preparation, smart filters, rasterization, shape/text editing, layer management, clipboard/filter/paint eligibility, and app row projection. A mask-presence or serialization check is still an attribute/raw-data check and should not be forced through a kind predicate.

All content queries derive from `LayerContent` or its resolved route. Preserve the existing C++ strings and command IDs with a small projection; for example a legacy `adjustment` row label may represent distinct active Adjustment and Fill kinds. Shape and smart-object badges/flags still identify their active kinds. Normalize genuinely conflicting imported row facts to the selected active content rather than retaining a second historical UI precedence chain. Well-formed documents retain their previous affordances and labels.

Update the existing `tools/type-layer-kind` requirement because it hard-codes a separate group/adjustment/background/type chain. Preserve its malformed-TySh recognition, saved bytes, and reader lock policy. Do not expand type editing to malformed descriptors.

The issue's counts guide a bounded workspace sweep of content checks; completion means no duplicate content precedence and no old writable active fields, not matching a stale count. Adding a kind requires a new active enum variant and decoder/constructor, and exhaustive consumers must make explicit rendering/editing decisions; no runtime registry is introduced.

## Risks / Trade-offs

- **Large source break across producers and tests** -> Migrate the whole workspace in one change, leaving no transitional public mirrors. Split new modules along data/decoding/execution concerns to respect file-size limits.
- **Decoded views diverge from saved data** -> Private storage, staged edits, and edit-save-reopen tests including unrelated-block preservation.
- **Conflicts and malformed inputs change appearance** -> A pre-migration route/output corpus covers primary-vscg precedence, proxy/source failure, unknown filters, and conflicting candidates; unknown content remains opaque rather than guessed.
- **Type and smart-object proxies become incorrectly editable pixels** -> Check identity independently from route and test command/row facts on proxy-backed content.
- **Mask edits strand stale Fill/Shape identity** -> Explicit atomic Fill↔Shape transitions, effective-key-presence recognition, and mask-command save/reopen/UI/history regressions; ordinary Pixel masks keep Pixel identity.
- **Moving decoders changes arithmetic or validation** -> Pure moves first, retain precisions/defaults and parser tests, then change the return enum. The destructive valid-variant matrix includes every surviving variant; native-depth refusals remain separate tests.
- **Eager normalization increases decode/open or history cost** -> Admit raw-copy/DOM/parser/retained allocations before allocation under the caller's existing budget, retain the Result-bearing resource-error path, and bound cube growth. Hidden LUT/temporary-DOM tests prove refusal before excess allocation; sufficient-budget controls preserve valid decoding. Reuse parsed/shared data and retain per-plan resolution and tile-backed channels; exercise existing lazy-open, sharing, and composite-plan regressions. No new whole-plane materialization is permitted.
- **Test-plan key parsing strips significant inline-code backticks** -> `scripts/check-test-plan.py::plan_rows` currently uses `strip("`")` on the scenario cell. The existing title "A `vscg` solid fill decodes to `SolidFill`" therefore cannot match its spec key. Fix only balanced outer table-cell quoting, retain internal/trailing inline-code marks, and extend the script's `self_test` before implementation. Existing MODIFIED scenario titles must stay exact for OpenSpec archive compatibility.
- **Test-plan guard expects tests that planning does not create** -> List every future test explicitly in tasks and implement those checks before production logic. The full guard becomes a shipping gate; planning validates row completeness separately and reports the pending implementation coverage honestly.

## Migration Plan

1. Before production edits, add the planned runnable checks and capture deterministic route/output and writer-baseline evidence from the current behavior. Do not overwrite fixture/golden files. Existing pre-migration expected bytes/known values remain the oracle; a comparison between two calls to the new resolver is insufficient.
2. Move plain data into core, split fills from adjustments, and relocate pure decoders. Keep execution and parser behavior unchanged while migrating imports and validation helpers.
3. Introduce private active content/preservation and atomic construction/transition operations, including Fill↔Shape mask edits. Add budget-aware parser admission before wiring eager codec normalization; migrate codec assembly and writer access, followed by all engine/app producers and consumers. Keep the core dependency check independent and the re-export identity check downstream. Finish all code before compiling.
4. Build once with the repository's CPU budget, fix errors and rebuild only after code changes, then run targeted tests and required full gates. Do not weaken existing behavioral checks. Report external oracle skips and GPU availability honestly.
5. Update `ROADMAP.md` for #1344, complete the plan, and archive/sync the change in the shipping PR. This planning request stops before implementation or archive.

Rollback is a source revert of the coordinated model migration; no data-format rollback is needed. Unchanged files and output bytes are covered by the existing preservation/golden contract.
