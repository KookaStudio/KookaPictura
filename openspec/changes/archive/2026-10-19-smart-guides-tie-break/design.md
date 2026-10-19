# Design: smart-guides-tie-break

`smart_guides.cpp` `bestAxis` iterates references before document targets and
`consider` only replaces the best candidate when it is strictly closer, so an
exact tie keeps the earlier reference. The first requirement already states that
behaviour; only the geometry requirement and the original design note disagreed.

The new test uses a 1000×1000 document so the only relevant document target is
the centre. A moving box at x `495..515` and a reference at x `490..510` are each
five pixels from the same edges: the reference candidate is found first and must
win with `snap.x == -5` (document-wins would be `+5`). The y axis has no target
within the threshold, so it does not affect the assertion.
