# document/presentation Specification

## Purpose

One change-report value and one presenter for every document change producer, so the bookkeeping, region-before-notification ordering, rebuild, and notification rules are defined once instead of copied per producer.

## Requirements

### Requirement: The five document change producers report one Presentation

The five document change producers SHALL report their result through one presentation value: the committed edit transaction, session cancellation, history restore, filter commit, and image-adjust preview poll. That value SHALL carry the document rectangles whose pixels the producer already folded into the displayed level 0 and pyramid, one rebuild kind, one notify flag, the dirty value to apply, and one content-revision flag. For the committed edit transaction the value SHALL also carry the normalized edit declaration, so the caller can report whether and how the document changed. A producer MUST NOT emit a region-damaged notification or a change notification itself.

#### Scenario: All five producers report the same value [pr_one_value]

- **WHEN** a committed edit, a cancelled session, a history restore, a filter commit, or an image-adjust preview poll completes
- **THEN** each returns the same presentation value shape, and no producer has emitted a region-damaged or change notification

#### Scenario: A producer does not present directly [pr_no_direct_signal]

- **WHEN** any of the five change producers completes
- **THEN** the shell receives no region-damaged or change notification until the presenter consumes the returned value


### Requirement: The presenter applies bookkeeping, then regions, then rebuild, then notifies

One presenter SHALL consume the presentation value in a fixed order: apply the dirty and content-revision bookkeeping, then emit one region-damaged notification per reported region in order, then perform at most one rebuild and at most one change notification. The presenter MUST NOT emit a change notification before every reported region has been emitted. It MUST emit at most one change notification per presentation. A presentation with no rebuild and no notify flag MUST emit no change notification.

#### Scenario: Bookkeeping precedes the region blits [pr_bookkeeping_first]

- **WHEN** a presentation carries a dirty or content-revision change and one or more regions
- **THEN** the dirty and content-revision bookkeeping is applied before the first region-damaged notification, and every region-damaged notification still precedes the change notification

#### Scenario: Regions precede the change notification [pr_regions_first]

- **WHEN** a presentation carries one or more regions and requests a change notification
- **THEN** every region-damaged notification is emitted before the single change notification

#### Scenario: At most one change notification [pr_one_notify]

- **WHEN** a presentation requests a change notification
- **THEN** exactly one change notification is emitted for that presentation

#### Scenario: A silent region presentation [pr_silent_region]

- **WHEN** a presentation carries regions and does not request a notification or rebuild
- **THEN** the regions are emitted and no change notification is emitted

#### Scenario: A notification without regions [pr_notify_only]

- **WHEN** a presentation carries no regions and requests a change notification without a rebuild
- **THEN** no region-damaged notification is emitted and exactly one change notification is emitted


### Requirement: One rebuild kind

The presentation value SHALL carry exactly one rebuild kind: none, synchronous display rebuild, or deferred display rebuild. A producer whose change cannot be bounded to the reported regions SHALL leave the document composite exact and request a rebuild; it MUST NOT reset the display itself. The presenter SHALL perform a synchronous rebuild from the exact document composite, or a deferred rebuild for a large history restore, before notifying. A presentation that requests a rebuild SHALL also notify.

#### Scenario: An unbounded edit rebuilds once [pr_rebuild_once]

- **WHEN** a presentation requests a synchronous rebuild
- **THEN** the display is rebuilt once from the exact document composite and exactly one change notification follows

#### Scenario: A large restore defers its rebuild [pr_deferred_rebuild]

- **WHEN** a history restore requests a deferred rebuild
- **THEN** the presenter starts the deferred rebuild and notifies, and the exact pyramid lands when the worker finishes

#### Scenario: A bounded edit does not rebuild [pr_no_rebuild]

- **WHEN** a presentation carries bounded regions and requests no rebuild
- **THEN** the display is patched only over those regions and no full rebuild runs

#### Scenario: A rebuild always notifies [pr_rebuild_notifies]

- **WHEN** a presentation requests a synchronous or deferred rebuild, regardless of its notify flag
- **THEN** exactly one change notification is emitted after the rebuild


### Requirement: The presenter owns dirty and content-revision bookkeeping

The presenter SHALL apply the presentation's dirty value and content-revision flag before emitting any region and before the change notification. A producer MUST NOT write the dirty flag or advance the content revision for a presented change; it SHALL report the dirty value it derived and whether the revision advances. History capture SHALL record the state without also setting dirty or advancing the content revision; the presenter is the only writer of those two values. A move commit of the topmost pixel layer SHALL report no content-revision advance. A history restore SHALL derive dirty from the saved identity without writing it, and report that value with a revision advance.

#### Scenario: An edit sets dirty and advances the revision [pr_bookkeeping]

- **WHEN** a committed edit presentation is applied
- **THEN** the document is dirty and its content revision advanced by one, both before the first region-damaged notification and the change notification

#### Scenario: A move commit keeps the content revision [pr_move_revision]

- **WHEN** the topmost pixel layer is moved and committed
- **THEN** the document is dirty and the content revision is unchanged

#### Scenario: A restore derives dirty [pr_restore_dirty]

- **WHEN** a history restore presentation is applied
- **THEN** the dirty flag reflects the saved identity and the content revision advances once


### Requirement: The display-preview fast path uses the same presenter

A live opacity, fill, or adjustment preview that patches the displayed level SHALL report through the same presentation value and presenter as a committed change. It MUST NOT emit a region-damaged notification or a change notification directly.

#### Scenario: A display preview presents through the presenter [pr_preview_presenter]

- **WHEN** a live opacity, fill, or adjustment preview patches the displayed level
- **THEN** it returns a presentation value, the presenter emits its region, and the preview path emits no signal of its own


### Requirement: Layer opacity and fill share one preview path

The layer opacity preview and the layer fill preview SHALL share one preview path and one commit path, parameterized by the property being changed. The two paths MUST NOT duplicate the display fast path, the exact fallback, the dirty and revision bookkeeping, or the pending-preview flag handling.

#### Scenario: Opacity and fill previews take the same path [pr_preview_shared]

- **WHEN** a layer opacity preview and a layer fill preview run on the same document
- **THEN** both execute the same preview path and the same commit path, differing only in the property setter, the preview kind, the pending flag, and the history label


### Requirement: Presentation preserves observable canvas behaviour

For every producer, the regions presented, the rebuild kind, the number of change notifications, the dirty value, and the content-revision value SHALL equal the behaviour before this capability existed. The region path SHALL remain byte-identical to a full recomposite over the same document.

#### Scenario: A bounded edit is unchanged [pr_bounded_equivalent]

- **WHEN** a bounded edit commits before and after this capability
- **THEN** both emit the same region rectangle and the same change-notification count, and the canvas is byte-identical

#### Scenario: An unbounded edit is unchanged [pr_unbounded_equivalent]

- **WHEN** an unbounded edit commits before and after this capability
- **THEN** both run one full recomposite, emit no region, and emit one change notification

