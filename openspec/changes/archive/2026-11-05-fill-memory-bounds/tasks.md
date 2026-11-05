# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Engine

- [x] 2.1 Flood from a single seed without a seed plane (`mask_ops::flood_from`)
- [x] 2.2 Read the bucket's source planes directly and flood a box with the selection as a wall
- [x] 2.3 Anti-alias the box without softening the selection edge
- [x] 2.4 Fill a box without a mask; build the stroke band in its window; bound the gradient

## 3. App

- [x] 3.1 Give every fill target the selection's box
- [x] 3.2 Estimate each fill's peak and refuse what does not fit the memory available
- [x] 3.3 Report the memory refusal from the tools and the Edit menu
