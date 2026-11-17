# layer-content Specification

## Purpose

Provides one authoritative editable content identity for each layer while preserving imported PSD data and existing rendering and saving behavior.

## Requirements

### Requirement: Exactly one active content kind

Every constructed, imported, cloned, or history-restored layer SHALL own exactly
one active content value: Pixel, Adjustment, Fill, Text, Shape, SmartObject, or
Group. Callers SHALL NOT be able to construct independently active group,
adjustment, text, or smart-object payloads on the same layer. An unsupported or
malformed imported payload SHALL be representable without inventing supported
parameters or dropping its original bytes. Raster proxies, raster masks,
effects, and passive preserved blocks SHALL NOT independently change the active
kind. Vector-mask edits SHALL preserve identity except for the explicit
Fill-to-Shape and Shape-to-Fill transitions below; a vector mask alone SHALL
NOT turn Pixel content into Shape content.

#### Scenario: Constructors cover every kind [lc_construct]

- **WHEN** a layer of each supported content kind is constructed through the public model interface
- **THEN** each layer reports exactly that one kind and exposes only its compatible active payload

#### Scenario: Incompatible active content is refused [lc_invariant]

- **WHEN** a caller attempts to install a second incompatible active payload or an invalid content transition
- **THEN** the public model prevents the conflicting state by construction or rejects the transition without changing the layer

#### Scenario: Proxies and masks preserve identity [lc_associated_data]

- **WHEN** Text or SmartObject content has a pixel proxy, a Pixel has a vector mask, or a Fill has a raster mask
- **THEN** each layer retains its active kind and the associated data remains usable

### Requirement: One normalization policy preserves rendering precedence

All raw imported content candidates SHALL be resolved by one normalization
policy. The policy SHALL preserve the existing effective compositing order:
group; decodable primary adjustment/fill or, only when no primary adjustment
block exists, decodable vector fill; undecodable primary adjustment; supported
enabled smart filters; smart source; type source; undecodable vector fill;
pixels. A decodable primary block SHALL win over `vscg`; an undecodable primary
block SHALL remain a no-op and SHALL NOT fall through to vector fill or pixels.
Conflicting candidates SHALL be normalized into one active value while their
unchanged raw bytes remain available for saving. Proxy selection and supported
filter/stack-mode decisions SHALL retain the existing conditions.

#### Scenario: Conflicting imported fields have one winner [lc_conflict]

- **WHEN** imported candidates combine a group, primary content, smart-object data, text data, vector fill, and proxy pixels
- **THEN** the normalized content and rendered result follow the precedence policy, and the losing raw candidates remain preserved

#### Scenario: A primary undecodable adjustment blocks fallthrough [lc_no_fallthrough]

- **WHEN** an undecodable primary adjustment coexists with a decodable vector fill and colored proxy pixels
- **THEN** the layer contributes no content and preserves all three data sources for an unchanged save

#### Scenario: Decodable vector fill wins before smart and text sources [lc_vector_precedence]

- **WHEN** there is no primary adjustment block and a decodable `vscg` fill coexists with smart-object or type data
- **THEN** the fill supplies the rendered content as before

### Requirement: Render routes are derived from active content

The compositor and GPU eligibility SHALL consume the same resolved content
route. Rendering SHALL distinguish pass-through and isolated groups,
destructive adjustments and generative fills, undecodable adjustments,
smart-filtered and smart-source content, type-source content, undecodable
vector fills, and pixels without redefining content identity. Raster proxies
SHALL retain their established priority except when supported smart filters or
an applicable stack mode select the source. An unknown enabled smart filter
SHALL retain the existing proxy fallback. The renderer SHALL preserve existing
fallbacks when an embedded source or text raster cannot be produced.

#### Scenario: Every route preserves its previous output [lc_routes]

- **WHEN** a deterministic corpus exercises every content route, all three fill kinds, group masks and opacity, proxy presence, supported and unknown smart filters, and active stack modes
- **THEN** the selected route and complete CPU output match the established pre-migration results

#### Scenario: A proxy changes the route without changing identity [lc_proxy_route]

- **WHEN** the same Text or SmartObject content is resolved with and without its raster proxy
- **THEN** the route follows the existing proxy policy and the active content kind is unchanged

#### Scenario: GPU eligibility uses the resolved route [lc_gpu_route]

- **WHEN** the corpus is checked for GPU eligibility, including unsupported source and adjustment cases
- **THEN** the same content route drives the decision and the existing fallback categories are retained

### Requirement: Generative fill content is distinct from adjustments

Generative fill data SHALL be represented by `FillContent`: Solid with a
straight-alpha RGBA color, Gradient with stops, reverse, kind, angle, and scale,
or Pattern with pattern id, scale, layer anchoring, and origin. Gradient kinds
SHALL remain Linear, Radial, Angle, Reflected, and Diamond. Stops SHALL retain
their locations in `0..=4096` and RGB colors; reverse SHALL default to false and
scale to 100. Pattern anchoring SHALL default to linked and origin to `(0, 0)`.
These fills SHALL NOT be valid destructive adjustment arguments. Existing
descriptor acceptance, approximations, generated pixels, and PSD bytes SHALL
remain unchanged.

#### Scenario: Each fill retains its parameters [lc_fill_models]

- **WHEN** Solid, Gradient, and Pattern values are constructed with non-default parameters
- **THEN** each exposes its own parameter data without conversion into a destructive adjustment

#### Scenario: Fill encoding and rendering remain unchanged [lc_fill_parity]

- **WHEN** existing solid, five-kind gradient, and pattern fixtures are decoded, rendered, and saved through the new fill model
- **THEN** their parameters, rendered CPU samples, and saved bytes match the existing baselines

### Requirement: Vector-mask edits transition fill and shape content atomically

A successful vector-mask addition to decodable Fill content that installs an
effective vector-mask definition SHALL atomically install Shape content with
the same fill. Removing or rasterizing the final effective vector-mask
definition from Shape content SHALL atomically install Fill content with the
same fill. Pixel content SHALL remain Pixel when its vector mask changes.
Recognition SHALL retain the existing effective-key policy: `vmsk` wins over
`vsms` by presence, even when malformed; an effective key plus decodable fill
identifies Shape regardless of the mask's enabled state, parsed-view
availability, or saved pixel-data-irrelevant flag. A refused mask operation
SHALL leave the active content and preserved data unchanged.

The transition SHALL update the active geometry and affected mask tags together
with the mask operation. Removing a mask SHALL retain the command's existing
unmasked-fill behavior; rasterizing it SHALL retain the existing combination
of vector coverage with the layer raster mask. The fill, layer attributes,
saved flags, and unrelated style/live-shape/opaque blocks SHALL retain the
existing command behavior; the identity transition SHALL NOT add a new cleanup
policy for those blocks. Save/reopen, layer-row facts, command eligibility,
and undo/redo SHALL reflect the completed transition.

#### Scenario: Adding a vector mask turns fill into shape [lc_fill_shape_add]

- **WHEN** a vector-mask addition succeeds on a decodable Fill layer without an effective vector-mask definition
- **THEN** its active content becomes Shape with the same fill and the installed effective geometry/tags, while its other attributes and saved flags retain the existing command behavior

#### Scenario: Removing or rasterizing the final vector mask restores fill [lc_shape_fill_remove]

- **WHEN** a Shape layer's final effective vector-mask definition is successfully removed or rasterized
- **THEN** its active content becomes Fill with the same fill, obsolete vector-mask geometry/tags are removed according to the command, deletion reveals the unmasked fill, and rasterization preserves the covered appearance through the existing raster-mask combination

#### Scenario: Effective-key presence keeps existing shape recognition [lc_shape_mask_presence]

- **WHEN** decodable fill data has a disabled or malformed effective `vmsk`, including a malformed `vmsk` beside a valid `vsms`, or a `vsms` alone
- **THEN** Shape identity follows effective-key presence with `vmsk` priority, independently of the saved pixel-data-irrelevant flag, and a mask operation refused for an unavailable parsed view leaves the state unchanged

#### Scenario: Mask transitions remain consistent across UI history and files [lc_mask_transition_state]

- **WHEN** vector-mask addition, deletion, and rasterization on fill/shape content are followed by save/reopen and undo/redo through their supported commands
- **THEN** active content, geometry/tags, row facts, content-command eligibility, saved attributes, and existing history counts agree with each restored state, and vector-masked Pixel controls remain Pixel

### Requirement: Derived import allocations are bounded and admitted

Import-time content normalization SHALL apply the caller's existing allocation
policy to newly introduced temporary and retained derived-content allocations,
including content on hidden layers. Before allocation, each new raw-payload
copy, descriptor DOM/string/list buffer, parser work buffer, and retained
parameter store SHALL fit the per-allocation and remaining document limits.
Retained stores across all layers SHALL contribute to the document total;
sharing already admitted storage SHALL NOT introduce an uncharged copy.
Resource-budget exhaustion SHALL fail import with a typed resource error
rather than being interpreted as malformed/unsupported content or silently
replacing a valid source with a no-op. Malformed content that fits the resource
policy SHALL retain its original opaque bytes and its established undecodable
or no-op representation.

Cube parsing SHALL bound point accumulation before exceeding the supported
maximum of `64^3` points, then retain the existing final declared-size and
exact-count validation. It SHALL NOT allocate excess point storage and check
the count only after consuming an arbitrarily long numeric payload. Content
that fits the policy SHALL retain its established valid decoding, rendering,
and unchanged-save bytes. No new channel-plane materialization SHALL be
introduced by this admission work.

#### Scenario: An oversized malformed cube is bounded before accumulation [lc_cube_bound]

- **WHEN** an imported Color Lookup cube contains more than `64^3` numeric points under a budget sufficient for the supported maximum and its input representation
- **THEN** point storage never grows beyond the supported maximum, the existing malformed-lookup no-op representation and original payload bytes are retained, and no allocation is requested for the excess points

#### Scenario: Hidden retained lookup tables count toward the document budget [lc_hidden_lut_budget]

- **WHEN** multiple hidden layers contain individually valid lookup tables whose per-allocation claims fit but whose retained stores together exceed a small explicit document budget
- **THEN** import fails with a typed resource error before the allocation that would exceed the total, rather than opening with silently omitted lookups

#### Scenario: Temporary content decoding respects the import budget [lc_decode_temporary_budget]

- **WHEN** a content payload would require a new raw copy, descriptor DOM/string/list buffer, or parser work allocation beyond an explicit per-allocation or remaining document limit, even though its final typed parameters could be small
- **THEN** import fails with a typed resource error before that allocation, including for hidden content, and does not classify the resource failure as malformed content

#### Scenario: Admitted valid content preserves existing behavior [lc_budget_valid]

- **WHEN** supported adjustment/fill payloads, including valid lookup tables and currently accepted later cube-size declarations, are imported within a sufficient explicit budget
- **THEN** decoded parameters, rendered samples, and unchanged-save bytes match the existing contract, and shared storage introduces no duplicate unaccounted allocation

### Requirement: Preservation is independent of active classification

An unchanged import-save SHALL preserve the original adjustment key/payload,
additional-layer-info blocks, decoded channels, and raw channel streams using
the existing encoding policy, including unsupported and conflicting content.
Normalization SHALL NOT re-encode supported descriptors solely because a typed
view was created. Existing serialized goldens SHALL remain byte-identical;
this requirement SHALL NOT imply byte-for-byte whole-file identity where the
existing writer already normalizes a container. Preservation access SHALL NOT
offer an independent way to mutate active content behind the typed model.

#### Scenario: Conflicting and opaque data survive PSD and PSB [lc_preserve]

- **WHEN** PSD and PSB data with conflicting or undecodable content, extra blocks, and raw channels are imported, saved unchanged, and reopened
- **THEN** active normalization is stable and every retained key, payload, block, and channel matches the original under the existing encoding policy

#### Scenario: Existing writer goldens do not change [lc_golden]

- **WHEN** the existing deterministic serialization corpus is written through the new model
- **THEN** its serialized bytes equal the established pre-migration golden bytes without regenerating baselines

### Requirement: Content transitions update saving and history consistently

An authoring edit that changes content SHALL atomically update active content
and its affected PSD representation or leave the document unchanged on refusal.
Unrelated preserved data SHALL remain intact. Rasterization SHALL install Pixel
content and remove the obsolete definition and conflicting content candidates
that could recreate the retired kind on reopen, while retaining attributes and
masks according to the existing rasterize command. Undo and redo SHALL restore
both the active value and corresponding preserved data with the existing
history-count and copy-on-write behavior.

#### Scenario: An edited fill reopens with the edited value [lc_edit_save]

- **WHEN** a preserved fill is edited, saved, and reopened
- **THEN** the active fill and rendered result reflect the edit, and unrelated preserved blocks are unchanged

#### Scenario: Rasterized content does not reappear on reopen [lc_rasterize_reopen]

- **WHEN** an imported Fill, Shape, Text, or SmartObject is rasterized through its supported command, saved, and reopened
- **THEN** it remains Pixel, matches the rasterized appearance, and obsolete content tags cannot restore its previous identity

#### Scenario: History restores content and preservation together [lc_history]

- **WHEN** a content edit or rasterization is undone and redone
- **THEN** the previous and new active values, saved representation, rendered samples, and existing history counts are restored together

### Requirement: Consumers share the authoritative classification

Rendering, effects, rasterization, content-edit eligibility, and layer-row facts
SHALL obtain content identity from the public authoritative model, rather than
reconstructing precedence from unrelated optional fields. Serialization,
preservation, and mask/lock attribute handling SHALL still inspect their own
data as needed. Existing C++ bridge strings, command IDs, and row affordances
for non-conflicting documents SHALL remain compatible; multiple content kinds
can map to one legacy bridge label without becoming the same active kind.

#### Scenario: Row facts and commands agree with active content [lc_consumer_agreement]

- **WHEN** documents containing each content kind, including raster-backed text and smart objects and masked fills, are projected into layer rows and queried for content commands
- **THEN** the row facts and command eligibility match the active content and retain the existing bridge protocol and affordances

#### Scenario: Unrelated masks do not create shape identity [lc_pixel_mask]

- **WHEN** a Pixel layer gains or loses a vector mask without receiving shape fill content
- **THEN** it remains Pixel and is not offered shape-content editing or Rasterize Shape

### Requirement: Core content data has no execution dependency

The public content model SHALL be usable by core and codec without depending
on adjustment execution, compositing, or Qt. Plain content parameters SHALL
have one definition shared with the execution crates. Invalid parameters and
unsupported buffer/depth combinations SHALL retain their existing validation
behavior; removing fill variants SHALL NOT expand native-depth or GPU support.

#### Scenario: Core content can be consumed without execution crates [lc_dependencies]

- **WHEN** the workspace dependency manifests and public core content types are checked
- **THEN** core has no dependency on codec, adjust, render, or Qt, and consumers refer to the same content parameter types

#### Scenario: Native-depth refusals are unchanged [lc_native_limits]

- **WHEN** an adjustment outside the existing native-depth support set is applied to a native-depth store
- **THEN** it is still refused without modifying samples, while valid supported 8-bit adjustments remain executable
