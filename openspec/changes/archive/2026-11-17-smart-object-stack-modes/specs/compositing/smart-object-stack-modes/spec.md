# smart-object-stack-modes Specification

## Purpose
Per-channel reductions over the layers of a smart object's embedded image stack, rendered non-destructively, with an Extended-only command surface.

## ADDED Requirements

### Requirement: Stack mode is an optional smart-object property

The system SHALL carry a smart object's stack mode as
`SmartObject.stack_mode: Option<StackMode>`, with `None` meaning no stack
render. `StackMode` SHALL have exactly Maximum, Minimum, Mean, Median,
Summation, Range, Variance, Standard Deviation, Skewness, Kurtosis, and
Entropy. A constructed or loaded object with no stored marker SHALL report
`None`, and two objects that differ only in stack mode SHALL compare unequal.

#### Scenario: The mode defaults to none and participates in equality [smst_model]
- **WHEN** a smart object is constructed and then a stack mode is assigned
- **THEN** the default reports `None` and the assigned object compares unequal to the default

### Requirement: Frames and per-pixel samples

Each visible top-level layer of the embedded source document SHALL contribute
one frame, rendered alone over transparency: a group contributes its flattened
result, a nested smart object its rendered source, and the layer's mask,
opacity, and effects apply to its own frame. A hidden layer SHALL NOT
contribute. At each pixel and colour channel the sample set `S` SHALL be the
values of frames whose alpha at that pixel exceeds zero, unweighted by alpha.
The reduction SHALL produce straight-alpha output: where `S` is empty the pixel
SHALL be transparent black, and the output alpha SHALL be the maximum alpha of
the contributing samples.

#### Scenario: Visible top-level layers are the frames [smst_frames]
- **WHEN** the embedded document has a hidden layer and a visible group
- **THEN** the hidden layer's pixels do not affect the reduction and the group contributes one frame

#### Scenario: Transparent pixels contribute no sample [smst_samples]
- **WHEN** some frames are transparent at a pixel
- **THEN** only the frames covering that pixel contribute to its reduction, and a pixel covered by none is transparent

#### Scenario: Output alpha is the maximum sample alpha [smst_output_alpha]
- **WHEN** contributing frames carry different partial alpha values at a pixel
- **THEN** the output alpha equals the largest of those values

### Requirement: Reduction formulas and numeric conventions

The reduction SHALL apply, per pixel and colour channel, to the sample set
`S` with `v` a sample value and `n = |S|`:

| Mode | Definition |
|---|---|
| Maximum | `max(v)` |
| Minimum | `min(v)` |
| Mean | `sum(v) / n` |
| Median | the middle value, or the mean of the two middle values when `n` is even |
| Summation | `sum(v)` |
| Range | `max(v) - min(v)` |
| Variance | `sum((v - mean)^2) / (n - 1)` |
| Standard Deviation | `sqrt(variance)` |
| Skewness | `sum((v - mean)^3) / ((n - 1) * stddev^3)` |
| Kurtosis | `sum((v - mean)^4) / ((n - 1) * stddev^4)` |
| Entropy | `-sum(p(v) * log2(p(v)))` with `p(v) = count(v) / n` |

Results SHALL be rounded to the nearest integer and clamped to `[0, 255]`.
Variance, Standard Deviation, Skewness, and Kurtosis SHALL be 0 when `|S| < 2`
or the standard deviation is 0, and negative Skewness SHALL clamp to 0. Entropy
SHALL be scaled from bits to the 8-bit channel range by `255 / 8` before
clamping. The reduction SHALL run on the 8-bit render channels; a 16- or 32-bit
embedded source is reduced after the existing 8-bit conversion and is therefore
approximate.

#### Scenario: Each mode follows its formula [smst_formulas]
- **WHEN** a three-frame stack is reduced by any mode
- **THEN** each output colour channel equals the rounded, clamped formula result for that pixel's samples

#### Scenario: An even sample set averages the middle samples [smst_even_median]
- **WHEN** a pixel has an even number of non-transparent samples and Median is active
- **THEN** the output equals the rounded mean of the two middle sorted samples

#### Scenario: Single-sample and zero-deviation sets yield zero [smst_moment_guards]
- **WHEN** a pixel has one sample, or all samples are equal, and an order-2 or higher moment mode is active
- **THEN** Variance, Standard Deviation, Skewness, and Kurtosis are 0

#### Scenario: Entropy scales to the channel range [smst_entropy_scale]
- **WHEN** a pixel's samples span the full 8-bit value range with maximum entropy
- **THEN** the entropy result is scaled by `255 / 8` and clamped to 0 through 255

#### Scenario: Summation and moments clamp [smst_clamp]
- **WHEN** Summation exceeds 255 or a moment result falls outside the channel range
- **THEN** the output channel is clamped to 0 through 255

### Requirement: Rendering is non-cumulative

Every stack render SHALL decode the current embedded payload and reduce its
frames. It SHALL NOT reuse a previous stack render or previously reduced pixels
as an input, and the layer's raster proxy SHALL NOT stand in for the payload's
frames. Applying a different mode SHALL produce the same result as applying that
mode to a freshly loaded object.

#### Scenario: Mean then Maximum uses the original frames [smst_non_cumulative]
- **WHEN** a stack is reduced with Mean, then with Maximum
- **THEN** the Maximum render equals a first-time Maximum render over the payload's frames, not a reduction of the Mean output

### Requirement: Applying and clearing a stack mode

The operation `set_smart_object_stack_mode` SHALL set the mode only when the
path resolves to a non-group, non-adjustment layer carrying a smart object
whose embedded payload parses and contains at least two frames. Setting `None`
SHALL be allowed on any resolved smart-object layer and SHALL clear the mode.
Any refusal SHALL return false and SHALL NOT mutate the document. Success SHALL
set or clear the object's mode. Setting the already-active mode SHALL be a
successful no-op (true without mutation), and the application SHALL record no
history state for it.

#### Scenario: A mode is set and cleared [smst_set]
- **WHEN** Median is applied to an eligible object and then None is applied
- **THEN** the object reports Median after the first call and `None` after the second

#### Scenario: Fewer than two frames is refused [smst_refuse_frames]
- **WHEN** a single-frame embedded object is asked to take a mode
- **THEN** the operation returns false and the document is unchanged

#### Scenario: Ineligible targets are refused [smst_refuse_target]
- **WHEN** the target is a group, an adjustment layer, a non-smart layer, an unreadable payload, or an unresolved path
- **THEN** the operation returns false and the document is unchanged

### Requirement: The reduced stack is the render source

When a smart object carries an active stack mode for which the reduced stack
source applies (an embedded payload with at least two visible frames and every
enabled Smart Filter decodes), the renderer SHALL use the reduced stack render
as the object's source, at the embedded document's resolution, before the
existing nearest-neighbour placement into the layer rect, layer properties,
and Smart Filter chain. The reduced source SHALL be cached by the payload and
the active mode. If the payload cannot be decoded,
contains fewer than two frames, or the reduction cannot be built, the renderer
SHALL fall back to the existing embedded-source or proxy path without error.
Clearing the mode SHALL restore the normal source precedence.

When an enabled Smart Filter cannot be decoded and the layer carries a raster
proxy, the stack mode SHALL NOT override the proxy and the existing proxy-path
rule applies.

#### Scenario: A three-frame stack renders its per-pixel median [smst_render_median]
- **WHEN** a smart object whose payload holds three aligned frames is composited with Median active
- **THEN** each pixel equals the per-channel median of the covering frames, placed in the layer rect

#### Scenario: Smart Filters apply to the reduced source [smst_render_filters]
- **WHEN** an object with an active stack mode also carries an enabled Smart Filter whose options decode
- **THEN** the filter is applied once to the reduced stack render, not to the proxy or the unreduced frames

#### Scenario: An unknown Smart Filter keeps the proxy [smst_render_unknown_filter]
- **WHEN** a proxied stack-mode object carries an enabled Smart Filter whose options cannot be decoded
- **THEN** the layer renders from its proxy and the unknown filter is not applied twice

#### Scenario: An unusable stack falls back [smst_render_fallback]
- **WHEN** an object with an active mode has an unreadable payload or fewer than two frames
- **THEN** the compositor renders through the existing proxy or embedded-source path without error

#### Scenario: Clearing restores the proxy render [smst_render_clear]
- **WHEN** an active mode is cleared and the document is composited again
- **THEN** the layer renders from its proxy, or from the unreduced embedded source when it has no proxy

### Requirement: Rasterize bakes the reduced stack

When a smart object has an active stack mode for which the reduced stack source
applies (an embedded payload with at least two visible frames and every enabled
Smart Filter decodes), `rasterize_smart_object` SHALL materialize the reduced
stack render into the layer's channels at the current size, SHALL clear the
object and its mode, and SHALL do so even when the layer already carries a
colour channel. A proxied layer whose stack source does not apply SHALL keep its
raster proxy, and a channel-less layer whose stack source does not apply SHALL
fall back to the embedded source. Rasterizing an object with no active mode
SHALL keep the existing proxy-preserving behaviour.

#### Scenario: Rasterizing bakes the stack render [smst_rasterize]
- **WHEN** a proxied smart object with an active Maximum mode is rasterized
- **THEN** its colour channels hold the maximum reduction and the layer no longer carries a smart object

### Requirement: The Extended-only Stack Mode command surface

The application SHALL expose `Layer > Smart Objects > Stack Mode` as a submenu
of 12 checkable commands in this order: None, Entropy, Kurtosis, Maximum, Mean,
Median, Minimum, Range, Skewness, Standard Deviation, Summation, Variance.
Their stable ids SHALL be `layer.smart.objects.stack.mode.none` and
`layer.smart.objects.stack.mode.<kebab-case label>`, with
`standard.deviation` for Standard Deviation. A mode command SHALL be enabled
only for an object eligible to take that mode, and None SHALL be enabled only
for a smart object with an active mode. The active mode's command SHALL be
checked, and None SHALL be checked when no mode is active. Running a mode
command SHALL apply it, recomposite, and record exactly one history state
labelled `Stack Mode: <label>`; running None SHALL record `Stack Mode: None`.
A refusal or re-selecting the active mode SHALL record no history state. In a
Standard build the 12 commands SHALL be absent.

#### Scenario: The submenu has the twelve leaves in order [smst_menu_rows]
- **WHEN** the Layer menu is built in an Extended build
- **THEN** `Stack Mode` holds None, Entropy, Kurtosis, Maximum, Mean, Median, Minimum, Range, Skewness, Standard Deviation, Summation, and Variance, each with its stable id

#### Scenario: Enabled and checked state follow the layer [smst_menu_state]
- **WHEN** an eligible object has Median active
- **THEN** Median is checked, None is enabled, and no ineligible layer enables a mode command

#### Scenario: Applying records one state [smst_menu_undo]
- **WHEN** a mode command runs on an eligible object
- **THEN** the mode is applied and exactly one state labelled `Stack Mode: <label>` is added

#### Scenario: Refusal and no-change record nothing [smst_menu_refusal]
- **WHEN** a command is refused or the active mode is selected again
- **THEN** the history count is unchanged and the document is unchanged
