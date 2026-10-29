# Spec Delta

## ADDED Requirements

### Requirement: Sample stores are copy-on-write

The retained typed sample store SHALL be copy-on-write. Cloning a sample store
SHALL share its sample allocation without copying samples. A write through a
mutable sample access SHALL fork the allocation only for the store being
written; a store that is not uniquely held MUST NOT be mutated in place. The
8-bit, 16-bit, and 32-bit variants SHALL share the same semantics, including the
8-bit Lab retained store.

#### Scenario: A cloned sample store shares its allocation

- **WHEN** a sample store of any depth is cloned
- **THEN** the clone and the original report the same sample allocation and equal bytes

#### Scenario: Writing a shared store forks only that store

- **WHEN** one clone of a shared sample store is written through its mutable samples
- **THEN** the written clone holds the new bytes, the other clone keeps the old bytes, and they no longer share an allocation

#### Scenario: A unique store writes in place

- **WHEN** a sample store with no other owner is written
- **THEN** it keeps its allocation and the write does not copy the samples

#### Scenario: Document clone shares retained samples

- **WHEN** a document with a 16-bit or 32-bit retained sample store is cloned
- **THEN** the document and its clone share the sample allocation until one of them is written

## MODIFIED Requirements

### Requirement: PixelBuffer is sample-typed with an 8-bit default

The engine's planar image buffer SHALL be generic over its sample type with `u8`
as the default, so a bare `PixelBuffer` denotes `PixelBuffer<u8>`, its `data`
field holds a refcounted `Plane<u8>`, and existing 8-bit indexing is unchanged.
The buffer SHALL support at least the `u8`, `u16`, and `f32` sample types,
keeping the planar layout `data.len() == width * height * channels`.

#### Scenario: A bare PixelBuffer is 8-bit

- **WHEN** code constructs `PixelBuffer::new(2, 2, 3)` and reads `data`
- **THEN** the value is a `Plane<u8>` of length 12 and the type is `PixelBuffer<u8>`

#### Scenario: A wider sample buffer is expressible

- **WHEN** code constructs a `PixelBuffer<u16>` of width 2, height 2, and 3 channels
- **THEN** its `data` is a `Plane<u16>` of length 12
