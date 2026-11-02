# Design

## Context

- `docs/06-filters/filters-overview.md` records the only behavioural fact: the
  filter removes interlacing from video frames by eliminating or interpolating
  **Even** or **Odd** fields. It lists De-Interlace for 8-, 16- and 32-bpc
  documents. It does not describe the dialog, the field parity, the edge rule,
  or the duplication source.
- The filter engine is planar `u8` (`pictura_core::PixelBuffer`, 3 or 4
  channels). No filter has 16/32-bpc plumbing, so this change follows the
  existing engine depth, exactly as NTSC Colors (#345) did.
- One `FilterSpec` row drives the menu row, the shared dialog and the kind
  mapping. `filter_map.rs` is a thin seam over `catalogue::build`, so the app
  needs no per-kind table.
- Filter kinds are recorded in three places that the existing guards keep in
  step: the engine's `Filter`/`FilterKind` tables, the catalogue, and the
  exhaustive test tables (`tests/common/instances.rs`,
  `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`).
- `apply_op_region` crops the layer to a clip-sized buffer and does not pass the
  clip's origin to the kernel, so a kernel that depends on an absolute row index
  cannot be previewed from a section.

## Goals / Non-Goals

**Goals:**

- A deterministic, documented approximation of De-Interlace that reconstructs
  one field from the retained field, for both field choices and both methods.
- The kernel in `pictura-filters::video`, with its own tests.
- A descriptor-driven menu entry that opens the shared dialog, previews live and
  commits one history state.
- Engine tests that pin parity, both methods, the edge rule, alpha
  reconstruction and refusal, plus a Qt Test for the dialog and the menu wiring.

**Non-Goals:**

- Reproducing Adobe's closed kernel bit for bit. There is no oracle, so the spec
  records the conventions as the project's approximation.
- 16- and 32-bpc filter support. That is an engine-wide gap shared with every
  other filter.
- Changing image dimensions. Photoshop's De-Interlace reconstructs the discarded
  field in place; it does not halve the picture.
- A dedicated dialog. The two radio groups fit the shared parameter dialog.

## Decisions

### Odd and Even are 0-based row indices

"Odd Fields" eliminates rows whose 0-based index is odd (`1, 3, 5, …`); "Even
Fields" eliminates rows whose index is even (`0, 2, 4, …`), so the first row of
the layer belongs to the even field. This is the usual deinterlace convention
(the top field is the even field) and the only reading that makes "Even" the
option that can eliminate the first row.

Adobe documents neither convention, and the docs corpus is silent, so the choice
is pinned by a unit test and stated in the requirement. The alternative, a
1-based scanline number, is the exact inverse and would flip every output.

### Duplication copies the retained row above

Each eliminated row `y` copies row `y - 1`, which is always retained because
adjacent rows alternate parity. At the top edge — reachable only when Even is
eliminated — there is no row above, so row `0` copies row `1`. The same
`above`/`below` neighbour helper serves Interpolation, so the two methods agree
on which rows are retained and differ only in how the new value is formed.

Copying the row below instead would shift the reconstructed field one line
down. Rejected because it is not the scanline-doubling behaviour the dialog's
name describes.

### Interpolation is the rounded mean of the two retained neighbours

Each eliminated row is the mean of the retained row above and the retained row
below, rounded half-up in integer arithmetic (`(a + b + 1) / 2`) so the result
is deterministic and never leaves `0..=255`. At an edge only one retained
neighbour exists, so the mean degenerates to that neighbour and Interpolation
and Duplication agree there. This is the standard linear deinterlace.

### The whole plane is reconstructed, alpha included

The kernel runs over every channel plane of the buffer, so a fourth (alpha)
plane is reconstructed the same way as colour. A layer whose transparency was
interlaced keeps its coverage aligned with its colour on the eliminated rows.
The catalogue row records `AlphaPolicy::Always`.

The alternative, `AlphaPolicy::Never`, leaves an interlaced mask edge untouched
and makes colour and coverage disagree on every eliminated row.

### The preview resolves against the whole layer

Row parity is a property of the layer's absolute row index. `apply_op_region`
crops a section-sized buffer without telling the kernel where the crop starts,
so a section preview beginning on an odd row would eliminate the wrong field and
then jump when the dialog committed. The catalogue row records
`whole_layer: true`, which is the existing answer for every other filter whose
result depends on absolute position (Diffuse, Lens Flare, the Blur Gallery).

Ceiling (`ponytail:`): a preview on a very large document recomputes the whole
layer on every tick instead of the visible section. Teaching `apply_op_region`
to pass the clip origin would let De-Interlace preview a section and is the
better long-term fix, but it changes the shared filter plumbing for every kind.

### No depth gate

The issue says the filter works on 8/16/32-bit documents. The engine has no
native-depth filter path, and NTSC Colors took the same position: the entry
stays enabled on every document depth and runs on the engine's 8-bit working
planes. The requirement says so explicitly and does not claim native-depth
precision.

## Risks / Trade-offs

- **Approximation without an oracle.** The spec records the parity, edge and
  duplication conventions; the tests pin them. It cannot prove Adobe parity
  (rule 5).
- **Whole-layer preview cost.** Accepted above, and recorded as a `ponytail:`
  ceiling.
- **Convention lock-in.** Changing the parity or duplication convention later is
  a spec change with test updates, not a silent tuning knob.
- **Command-count churn.** The golden catalogue, the unwired list and the
  implemented count must change together or `pictura-script`'s catalogue tests
  fail.
