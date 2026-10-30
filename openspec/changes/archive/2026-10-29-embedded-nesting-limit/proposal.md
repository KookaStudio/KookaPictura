# Proposal

Tracking issue: #1044 (review finding on #1046).

## Why

An embedded smart object's payload is a PSD from the opened file, and it can
hold another no-proxy smart object, and so on. Each level renders on the stack
of the level that encloses it, with no depth limit. Once #1044 stopped pool
workers from waiting on an in-flight build, a crafted file nested hundreds of
levels deep recurses until a worker's stack overflows, which aborts the
process. Embedded payloads already come from user-opened files, so the
untrusted case exists today.

The cache digest (`content_digest`, FNV-1a) is not collision-resistant. A
payload crafted to collide with an enclosing one re-enters its own cache key on
the building thread. Outside the pool, that thread waits on its own build
forever.

## What Changes

- `SmartObject` gains `nesting`, render metadata excluded from equality and
  never written. It counts the embedded documents that enclose the object, and
  `decode_embedded` sets it on every smart object in a decoded payload. The
  depth travels in the data, so it holds when a nested composite runs on another
  pool thread.
- `embedded_source` refuses an object at `nesting >= MAX_EMBED_NESTING` (8).
  The layer then renders as any unusable source does.
- The embedded cache key includes `nesting`, so an enclosing object and its
  descendant never share a key through equal payload digests.
- A cache slot records the thread that claimed it, and no thread waits on a
  build it owns. A same-thread re-entry builds its own copy, which the nesting
  limit bounds.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/smart-object-rendering`: embedded nesting is bounded.
- `compositing/layer-source-cache`: a build never waits on itself.

## Impact

- `crates/pictura-core/src/lib.rs`: the `SmartObject::nesting` field.
- `crates/pictura-render/src/layer_source/mod.rs`: the nesting guard, the
  propagation in `decode_embedded`, and the slot owner.
- `crates/pictura-render/src/layer_source/key.rs`: nesting in the embedded key.
- `crates/pictura-render/src/layer_source/tests.rs`: four tests.
