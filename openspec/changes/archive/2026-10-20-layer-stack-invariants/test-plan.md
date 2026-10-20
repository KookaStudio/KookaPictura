# Test Plan

Every scenario in `specs/document/layer-stack-invariants/spec.md` maps to an
automated Rust test. The new tests are written before the implementation that
makes them pass; the existing `clipping.rs`, `tests_background.rs`, and
`merge_tests.rs` scenarios remain the semantic oracle.

Coverage uses the repository's two-segment test reference form
(`<crate>::<fn>`), which `scripts/check-test-plan.py` validates.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| A merge that removes a base releases its run | Build `[A, B(clipped to A), C]`, select `A` and `C` for Merge Layers so `B` is left in place, run the merge | `B.clipping == false`; the result stack is well-formed; no layer bases on a removed node | `pictura-render::merge_removing_a_base_releases_its_run` |
| Smart-object conversion splices a valid run | Convert a smart object whose embedded source has a clipped bottom layer, and one whose source would clip to a Background | The spliced bottom is unclipped; no clip bases on a Background; the stack validates | `pictura-render::smart_object_conversion_splices_a_valid_run` |
| Insertion and lift never leave a baseless clip | Insert via Copy/Cut, lift a selection, paste, and add a clipped layer at a container bottom, above a Background, and inside a run | Every result validates; a clipped insertion with no legal base is unclipped; an unclipped insertion inside a run may become its base; insertion at or below the Background is refused or moved above it | `pictura-render::insertion_and_lift_never_leave_a_baseless_clip`; `pictura-render::every_structural_op_leaves_a_well_formed_stack` |
| Grouping a base keeps its run attached | Group `[A(base), B(clipped to A)]` together; then group `A` alone | `B` stays clipped to the new group; the group's children and the parent validate | `pictura-render::grouping_a_base_keeps_its_run_attached`; `pictura-render::grouping_a_clipped_layer_clears_the_new_bottom` |
| Grouping a middle member leaves a valid stack | Group only `B` from `[A(base), B(clipped), C(clipped)]` | `C` clips to the unclipped group as a legal base; no baseless clip; both containers validate | `pictura-render::grouping_a_middle_member_leaves_a_valid_stack` |
| Ungrouping a base releases its run | Ungroup a group that is a base with a clipped run above it | The run is released; the spliced children and the parent validate | `pictura-render::ungrouping_a_base_releases_its_run`; `pictura-render::ungrouping_a_base_group_releases_its_run` |
| Reverse keeps the container bottom unclipped | Reverse a container whose top layer is clipped so it becomes the bottom | The layer is released; the container bottom is unclipped; the stack validates | `pictura-render::reverse_keeps_the_container_bottom_unclipped`; `pictura-render::reverse_clears_a_clipped_bottom` |
| A move below the Background is refused | Move a normal layer to the index below a Background; separately, apply a negative delta to the layer directly above it | Move-to is refused and the document is unchanged; the delta move clamps to the floor and changes the order; each dry run agrees with its mutation | `pictura-render::move_path_to_refuses_below_background`; `pictura-render::move_path_clamps_at_background_floor` |
| Scratch filtering releases a clipped base | Build a scratch tree where an adjustment layer is the base of a clipped run; run the adjustment filter | The run is released; the filtered scratch tree is well-formed | `pictura-render::scratch_filtering_releases_a_clipped_base` |
| A malformed file round-trips unchanged | Build a tree with a clipped bottom and a clip based on a Background, write it, read it back, write it again | The flags survive the read and the two writes are byte-identical | `pictura-codec::malformed_clip_round_trips_unchanged`; `pictura-codec::clip_on_a_background_round_trips_unchanged` |
| The first edit normalizes only the touched container | Build a tree with an invalid clip in group `B`; apply a structural edit in group `A` | `A`'s clipping is normalized; `B` keeps its loaded flags; no panic and the check reports `B` as still invalid | `pictura-core::first_edit_normalizes_only_the_touched_container` |
| A misplaced Background is not repaired | Build a tree with a Background above a sibling, then a group carrying the flag, then two Backgrounds; apply an unrelated structural edit to each | The edit does not move, clear, or choose among the Backgrounds; the check still reports the malformed placement | `pictura-core::malformed_background_is_not_repaired` |
| A valid tree passes | Check a Background at the root bottom, an ordinary clipped run above a non-Background base, and nested groups | `well_formed` returns `Ok(())` | `pictura-core::valid_tree_passes` |
| A clipped container bottom is reported | Set `clipping` on the root bottom, then on a group's bottom | The violation names the container path and the clipped-bottom cause | `pictura-core::clipped_bottom_is_reported` |
| An illegal clip base is reported | Clip a node with no sibling below it; clip a node directly above a Background | The violation names the node path and the illegal-base cause | `pictura-core::illegal_clip_base_is_reported` |
| A misplaced or duplicate Background is reported | Use three separate trees: a Background above a sibling, a group carrying the flag, and two Backgrounds | Each call reports the offending node path and the corresponding cause; no first-violation ambiguity | `pictura-core::misplaced_or_duplicate_background_is_reported` |

## Additional coverage (not spec scenarios)

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Every structural op leaves a valid stack | Apply the full op matrix to a canonical tree and compare base identity before and after | The postcondition holds; dry runs do not mutate; released runs are not silently re-based | `pictura-render::every_structural_op_leaves_a_well_formed_stack` |
| Primitive property test | Apply random primitives to generated small trees with a fixed seed, including malformed inputs | A refused edit leaves the tree unchanged; a normalized edit yields a well-formed tree and never panics | `pictura-core::property_structural_primitives_keep_the_tree_well_formed` |
| Raw install is permissive | Install a malformed tree with `from_layers` | The tree is unchanged; the check reports the violation | `pictura-core::from_layers_accepts_a_malformed_tree_unchanged` |
| Stamp placement still joins a run | Insert a stamp above a clipped run | The stamp joins the run; the existing placement behavior is preserved | `pictura-render::stamp_above_a_clip_run_keeps_its_placement` |

| Anchor-shift and refusal atomicity | Remove a sibling before the anchor's ancestor; refuse a floor violation | The merge lands in the shifted ancestor; a refusal leaves the tree identical | `pictura-core::splice_maps_the_anchor_parent_after_a_removal`; `pictura-core::splice_refusal_leaves_the_tree_unchanged`; `pictura-core::refusals_leave_the_tree_unchanged` |
| Nested Background insertion | Insert a group carrying a Background into a tree that already has one | Refused with the tree unchanged | `pictura-core::insert_refuses_a_second_nested_background` |
| Retain-tree isolation and release | No-op filter on a malformed tree; filter a base out of a run | The untouched container keeps its flags; every run member is released | `pictura-core::retain_tree_noop_does_not_normalize_untouched_containers`; `pictura-core::retain_tree_releases_every_member_of_a_run` |
| Move out of a malformed source | Move a layer out of a container below its Background | The cross-container move succeeds and the result validates | `pictura-core::move_node_can_leave_a_source_below_the_background`; `pictura-render::move_path_to_can_leave_a_source_below_the_background` |
| Background placement on replace | Replace a bottom group with its Background child; replace a non-bottom group | The bottom replacement succeeds; the non-bottom one is refused unchanged | `pictura-core::replace_allows_a_background_only_at_the_container_bottom` |
| Merge across a shifting ancestor | Select a root layer and a layer inside a group below it | The merge lands in the shifted group, not a sibling | `pictura-render::merge_selected_maps_a_shifting_ancestor` |
| Lift/via refusal preserves pixels | Lift or via-cut from a source below a Background | The command returns failure and the source alpha is unchanged | `pictura-render::lift_refusal_keeps_source_pixels`; `pictura-render::via_cut_refusal_keeps_source_pixels` |
| Smart object over a Background | Replace a smart object on a Background host with a clipped source | The replacement is unclipped and the host validates | `pictura-render::smart_object_conversion_with_a_background_host` |
| Merge lifted keeps the run | Lift a base, then merge the floating layer back | The run stays clipped to the source | `pictura-render::merge_lifted_keeps_a_run_clipped_to_the_source` |
| Nested Background ungroup | Ungroup a bottom group with a nested Background; ungroup a non-bottom one | The bottom splice succeeds; the non-bottom replace is refused unchanged | `pictura-render::ungrouping_a_bottom_group_with_a_nested_background_is_allowed`; `pictura-render::ungrouping_a_non_bottom_group_with_a_nested_background_is_refused` |
| Empty container and unresolved path | Query a move in an empty group; insert against an unresolved nested selection | The query refuses without underflow; the insert falls back to the top level | `pictura-render::empty_container_queries_do_not_underflow`; `pictura-render::unresolved_selection_path_falls_back_to_the_top` |

