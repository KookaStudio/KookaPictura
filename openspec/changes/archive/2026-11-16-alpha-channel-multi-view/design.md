# Design

## Context

The previous change stored one `shownAlpha_` index. The frame built one red
rubylith overlay from `channels_alpha_thumbnail`. CS6 keeps each alpha eye
independent, so the state becomes a set.

## Decisions

### 1. A set of shown indices

`ChannelsPanel` stores `QSet<int> shownAlphas_`. Each alpha eye toggles its own
membership; `updateAlphaEyes` checks each eye against the set. The composite
and colour eyes clear the set. Delete removes the deleted index and shifts
higher indices down.

### 2. Composite the rubylith overlays

The frame fetches each shown channel's preview at the same capped size. Each
channel contributes red at 50 % opacity over its masked (black) area. The frame
multiplies the per-pixel remaining transparency across the shown channels, so
one channel is 50 % red, two are 75 %, three are 87.5 %, and so on. The overlay
is RGBA, so the composite stays visible beneath it.

### 3. Cache key includes the set

The overlay cache key lists the shown indices in set order plus the canvas
revision, so adding/removing a channel rebuilds the overlay.

## Risks / Trade-offs

- [Set iteration order] -> `QSet` values are not sorted; the key and composition
  order may vary, but multiplying transparency is commutative and the key
  changes only on toggles.
- [Large documents] -> each preview is capped at 4096 px per side; the combined
  buffer is one capped RGBA plane.
- [No per-channel options] -> the overlay is fixed red at 50 % with the Masked
  Areas sense; channel options are out of scope.
