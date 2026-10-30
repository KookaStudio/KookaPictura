# Design

## Carry the depth in the data

A thread-local depth counter cannot bound this recursion. `composite_rgba` runs
bands and rows on the rayon pool, so the nested `embedded_source` call can run
on a different worker from the one decoding the enclosing payload, where a
thread-local counter starts at zero again. Work stealing still stacks the
nested work on whichever worker runs it, so the stack grows without the counter
seeing it.

Passing the depth as an argument would thread it through `composite_rgba` and
every compositing entry point. Instead the depth is a field of the decoded data.
`decode_embedded` owns the `Document` it just read, so it walks that document's
layer tree (with an explicit stack, since group nesting is attacker-controlled
too) and sets `nesting = parent + 1` on every smart object. The objects of an
opened file default to `0`.

`nesting` is excluded from `PartialEq` and is never written, like
`payload_digest`: it describes where a decoded copy sits, not document content.

## The limit

`MAX_EMBED_NESTING = 8`: objects at nesting 0 through 7 decode, so a file can
nest eight payloads inside one another. That is far beyond real use; CS6
documents rarely nest more than two or three. It keeps the deepest render to
eight composite frames on a worker stack. An object past the limit makes
`embedded_source` return `None`, and the layer renders as any unusable source
does: its proxy channels if it has them, otherwise nothing.

## The key and the slot owner

Embedded keys hash `payload_digest`, which is FNV-1a and can be made to collide.
Hashing `nesting` into the key separates an enclosing object from its own
descendants. That is a cheap first line of defence, not a proof, because the
outer key hash is 64-bit too. The guarantee comes from the slot: `Building`
records the claiming thread, and a caller outside the pool waits only on a build
some *other* thread owns. A same-thread re-entry builds its own copy; the first
copy published wins, as on the pool. Because the nesting limit bounds the
recursion, the re-entry is bounded too.

## Test shape

`nested_payload(n)` wraps `source_doc` in `n` documents whose only layer is a
no-proxy embedded smart object, cutting off each wrapper's merged composite so
every level renders through its source. Seven wrappers render the innermost
colour; eight stop at the limit. A 200-deep payload rendered on a 2-thread pool
with 512 KiB stacks must finish, under the shared watchdog. The re-entry test
nests a same-key `lookup` inside a build on a plain thread under the watchdog.
