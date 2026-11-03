# Tasks

## 1. Tests

- [x] 1.1 Add the tests in `test-plan.md`

## 2. Paint engine

- [x] 2.1 Test the flood's pixels in parallel and return a uniform surface whole
- [x] 2.2 Run the anti-alias box in parallel from column sums
- [x] 2.3 Reuse the last blend across a run of equal pixels in a fill

## 3. Compositor

- [x] 3.1 Start a band at the topmost plain opaque Normal layer over a provably opaque stack
- [x] 3.2 Copy an opaque layer's bytes for a band with nothing shown above it

## 4. History

- [x] 4.1 Share the live plane on a capture with no hint
- [x] 4.2 Share the restored plane with the live document on an in-place undo
- [x] 4.3 Swap band-aligned tiles one band per core

## 5. Evidence

- [x] 5.1 Time the click and its undo in `bucket_fill_profile`
