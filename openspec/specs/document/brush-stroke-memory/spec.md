# document/brush-stroke-memory Specification

## Purpose
Bounds what a brush stroke holds and what its start is charged by the layer's
real tiles, so a sparse layer is not refused for memory it never uses.

## Requirements

### Requirement: A stroke's undo copy holds only the tiles it reached

A stroke SHALL keep the pre-stroke pixels of its target layer on a grid of
64 × 64 tiles, each allocated when a dab first reaches it and holding only its
part inside the layer at that moment. Tiles no dab reached SHALL hold no
memory, in the 8-bit planes and in a tone stroke's 16-bit store alike. Cancelling
the stroke SHALL write every reached tile back, so the layer equals its
pre-stroke state.

#### Scenario: One dab saves only the tiles under it [bsm_undo_tiles]

- **WHEN** one dab is painted on a large layer
- **THEN** the undo copy holds exactly the grid tiles the dab's box crosses, and
  its bytes are those tiles' bytes, not the layer's

#### Scenario: A cancelled stroke restores the layer [bsm_cancel]

- **WHEN** an 8-bit stroke or a 16-bit tone stroke is painted and then cancelled
- **THEN** every pixel of the layer equals its pre-stroke value

### Requirement: Growing the layer moves no saved pixels

When the target layer grows under a stroke, the undo copy SHALL keep the tiles it
already holds and SHALL allocate none for the grown area or for the old layer's
unreached pixels. A pixel in the grown area SHALL read as transparent before the
stroke.

#### Scenario: A grow keeps the saved tiles [bsm_grow]

- **WHEN** a stroke on a layer smaller than the canvas paints outside the layer,
  so the layer grows
- **THEN** the undo copy holds the same tiles as before the grow plus only those
  the new dabs reached, and cancelling restores the original layer and bounds

### Requirement: The stroke-start gate charges the layer's held bytes

The brush stroke's start SHALL be charged the bytes the target layer's planes
hold, not their logical size, plus one byte per canvas pixel for the stroke's
own tiles. A uniform tile SHALL cost nothing. The charge SHALL be at least the
heap peak the memory benchmark measures for a stroke on an empty layer (M6) and
on a dense layer.

#### Scenario: A sparse layer on a large canvas paints [bsm_gate_sparse]

- **WHEN** the governor's room is pinned near 1.9 GiB and a stroke starts on a
  sparse 15,000 × 17,500 layer of a 21,000² canvas
- **THEN** the stroke is admitted

#### Scenario: A dense layer beyond the room is refused [bsm_gate_dense]

- **WHEN** the governor's room is pinned below a dense layer's held bytes and a
  stroke starts on it
- **THEN** the stroke is refused with the memory reason

#### Scenario: The estimate covers the dense-layer stroke [bsm_dense_peak]

- **WHEN** the memory benchmark's brush stroke runs on a dense 4250² layer alone
  in its process
- **THEN** its measured heap peak is at most the stroke's estimate, and is far
  below the layer's size
