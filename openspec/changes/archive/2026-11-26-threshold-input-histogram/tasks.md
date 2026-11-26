# Tasks

## 1. Regression coverage

- [x] 1.1 Write the Rust selection-weighting and bounded-sampling tests and the dialog and Properties Qt cases listed in the test plan, including both mask targets, refresh, live selection moves, and group boundaries.

## 2. Threshold histogram

- [x] 2.1 Derive luminance bins from the pre-adjustment targets and selection using deterministic bounded tiles and cached mask bounds; use Quick Mask working_value in its current polarity.
- [x] 2.2 Share the histogram widget between the Threshold pages, place it between the value row and slider, and connect each host's source and Properties refresh, including coalesced selection-move previews.

## 3. Deferred execution

- [ ] 3.1 Build and run the new cases and repository gates when the user requests execution.
