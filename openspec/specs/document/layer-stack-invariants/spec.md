# document/layer-stack-invariants Specification

## Purpose
Keeps every structural layer-tree edit from leaving a clipped container bottom,
a baseless or Background-based clip, or a misplaced or duplicate Background,
while still loading malformed documents as authored.

## Requirements

### Requirement: Structural edits preserve stack well-formedness

When the pre-edit tree is well-formed, every structural layer-tree edit —
inserting, removing, moving, splicing, replacing, grouping, ungrouping,
reversing, truncating, or filtering nodes, and the merge,
smart-object-conversion, and scratch-document paths built on them — SHALL leave
every affected container well-formed:

- the bottom node of a container SHALL NOT be clipped;
- every clipped node SHALL have a non-clipped, non-Background sibling below it
  in the same container;
- a node carrying the Background flag SHALL be the bottom-most node of its
  container, SHALL NOT be clipped, and SHALL NOT be a group;
- the document SHALL contain at most one Background overall.

An edit that removes a clipping base without replacing it with a new base SHALL
release every layer that was clipped to it and MUST NOT let those layers
re-base to another sibling. Grouping a base, with or without its run, keeps the
run attached to the new group, which becomes its base; grouping a middle member
of a run makes the group the base for the members left above it.

An edit that has no legal landing — a move-to at or below the Background —
SHALL be refused with the document unchanged; a relative move clamps at the
Background floor rather than crossing it. An edit that can be normalized SHALL
normalize instead of refusing: a requested clip with no non-Background base is
dropped, a removed or reversed container bottom is unclipped, and a run stranded
by a removal is released.

#### Scenario: A merge that removes a base releases its run

- **WHEN** Merge Down, Merge Layers, or Merge Visible removes a layer that is
  the base of a clipped run not included in the merge
- **THEN** every layer that was clipped to the removed base is released, and the
  resulting stack has no clipped layer based on a removed node

#### Scenario: Smart-object conversion splices a valid run

- **WHEN** Convert to Layers replaces a smart object with source layers whose
  bottom node is clipped, or with layers that would clip to a Background
- **THEN** the spliced layers are normalized so the container bottom is
  unclipped and no clip is based on a Background

#### Scenario: Insertion and lift never leave a baseless clip

- **WHEN** Layer via Copy/Cut, selection lift, paste, or a new layer inserts a
  node above, below, or inside a clipped run, including at a container bottom
  or at or below a Background
- **THEN** the resulting stack remains well-formed; a clipped insertion with no
  legal base is unclipped; an unclipped insertion inside a run may become the
  base for the run above; and an insertion at or below the Background is
  refused or placed above it

#### Scenario: Grouping a base keeps its run attached

- **WHEN** a base is grouped, with or without its clipped run
- **THEN** the run stays clipped to the new group, which becomes its base, and
  both the group's children and the parent container are normalized

#### Scenario: Grouping a middle member leaves a valid stack

- **WHEN** a middle member of a clipped run is selected for grouping
- **THEN** the members left above the group clip to the group as a legal
  non-Background base, no baseless clip is created, and both containers are
  well-formed

#### Scenario: Ungrouping a base releases its run

- **WHEN** a group that is the clipping base is ungrouped and its children are
  spliced into the parent
- **THEN** the run that was clipped to the group is released, and the spliced
  children and the parent container are normalized

#### Scenario: Reverse keeps the container bottom unclipped

- **WHEN** Reverse moves a clipped layer to the bottom of its container
- **THEN** that layer is released and the container bottom is not clipped

#### Scenario: A move below the Background is refused

- **WHEN** a move-to targets at or below the bottom-most Background in the
  destination container, or a relative move targets past it
- **THEN** the move-to is refused with the document unchanged and the relative
  move stops at the floor directly above the Background

#### Scenario: Scratch filtering releases a clipped base

- **WHEN** a scratch-tree filter removes an adjustment layer that is the base
  of a clipped run
- **THEN** the run is released, and the filtered scratch tree is well-formed

### Requirement: Malformed stacks load as authored

A layer tree installed from a file, a history snapshot, or a scratch document
SHALL be accepted without validation or rewriting, so a PSD created elsewhere
loads exactly as authored. A structural edit on such a tree SHALL normalize the
clipping in the containers it touches and SHALL NOT rewrite containers the edit
does not affect. It SHALL NOT repair a misplaced, grouped, or duplicate
Background: those are reported by the check and left as authored. The engine
MUST NOT panic on a malformed pre-existing tree.

#### Scenario: A malformed file round-trips unchanged

- **WHEN** a PSD containing a clipped container bottom or a clip based on a
  Background is opened and saved without a structural edit
- **THEN** it loads with those flags intact and saves them unchanged

#### Scenario: The first edit normalizes only the touched container

- **WHEN** a document with an invalid clip in one container is structurally
  edited in a different container
- **THEN** the edited container's clipping is normalized, the unrelated invalid
  container is left as loaded, and no panic occurs

#### Scenario: A misplaced Background is not repaired

- **WHEN** a malformed tree has a Background above a sibling, a group carrying
  the flag, or two Backgrounds, and an unrelated structural edit runs
- **THEN** the edit does not move, clear, or choose among the Backgrounds, and
  the check continues to report the malformed placement

### Requirement: Stack well-formedness is checkable

The engine SHALL provide a side-effect-free well-formedness check over a layer
tree that reports the first violating node path and cause, covering a clipped
container bottom, a clip without a legal base, a Background that is not the
bottom-most node of its container, a Background that is a group, and a second
Background. The traversal SHALL be root-first and bottom-first within each
container so the reported violation is deterministic. Every structural edit
SHALL assert the check in debug builds after a pre-edit tree that was itself
well-formed, and the check SHALL be usable by tests without cloning the
document.

#### Scenario: A valid tree passes

- **WHEN** a document with a Background at the bottom, an ordinary clipped run
  above a non-Background base, and nested groups is checked
- **THEN** the check reports no violation

#### Scenario: A clipped container bottom is reported

- **WHEN** the bottom node of the root or a group carries the clipping flag
- **THEN** the check reports that container's path and the clipped-bottom cause

#### Scenario: An illegal clip base is reported

- **WHEN** a clipped node has no non-clipped sibling below it or is clipped to
  the Background
- **THEN** the check reports that node's path and the illegal-base cause

#### Scenario: A misplaced or duplicate Background is reported

- **WHEN** a Background sits above another sibling, is a group, or a second
  Background exists
- **THEN** the check reports the offending node path and the corresponding
  cause
