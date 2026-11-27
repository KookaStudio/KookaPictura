## Purpose

Deform raster content with a triangulated mesh and interactive pins while keeping the document intact until the user applies the result.

## ADDED Requirements

### Requirement: Target a supported raster surface
Edit > Puppet Warp SHALL open for one unlocked, visible supported 8-bit RGB or Grayscale raster layer or its targeted user mask. It SHALL reject Background, unsupported targets, active pixel selections, and surfaces outside the workspace limits.

#### Scenario: Warp an imported 8-bit raster or its mask [puppet-native]
- **WHEN** an eligible 8-bit RGB or Grayscale layer has retained native samples
- **THEN** the layer or its mask is editable, and applying preserves authoritative color and mask data without stale retained samples

#### Scenario: Warp a layer with a linked raster mask [puppet-linked]
- **WHEN** a supported layer with a linked raster mask is warped
- **THEN** its color and mask deform with the same mesh and one undo restores both, including when their bounds differ

#### Scenario: Enable the command for the selected raster [puppet-target]
- **WHEN** the user selects a supported raster, then a locked or unsupported surface
- **THEN** the menu enables the raster operation and disables it for the unsupported surface

### Requirement: Deform a triangulated mesh with pins
The Puppet Warp editor SHALL let the user place, select, drag, and remove pins. Pin positions SHALL be hard constraints on the deformation. Density SHALL change mesh resolution, Expansion SHALL pad the mesh, and Mode SHALL change how nearby pins influence unpinned vertices. Show Mesh SHALL control mesh display independently of pins; holding H SHALL hide pins until release. The active pin SHALL support moving forward and backward in depth, Auto and manual rotation, and deletion by Delete or Alt-click on the pin. Alt-drag on its nearby rotation ring SHALL rotate it manually. Shift-click SHALL select multiple pins and Shift-drag SHALL move the selected pins together; Ctrl+A SHALL select all pins, Ctrl+D SHALL deselect them, and Ctrl+Z SHALL undo the last pin adjustment inside the editor.

#### Scenario: Drag one pin while another anchors the mesh [puppet-anchor]
- **WHEN** one pin is dragged while a second stays fixed
- **THEN** the moved and fixed vertices reach their exact requested positions and the rest of the mesh deforms

#### Scenario: Preserve the area around a stationary anchor [puppet-anchor-area]
- **WHEN** two pins are 100 pixels apart, both use manual zero rotation, and one moves 10 pixels while the other stays fixed in Normal mode
- **THEN** points within one pixel of the fixed pin move less than one pixel

#### Scenario: Change the visible mesh and pin options [puppet-controls]
- **WHEN** the user changes the mesh options, selects or rotates a pin with its Alt-drag ring, changes its depth, holds and releases H, selects and moves multiple pins, undoes a pin adjustment, and Alt-clicks a pin
- **THEN** the editor updates the preview without changing the document, H hides only pins until release, selection and local undo work, and Alt-click removes the pin

#### Scenario: Continue from automatic rotation [puppet-auto-gesture]
- **WHEN** the user Alt-drags the rotation ring of a pin with a nonzero Auto angle
- **THEN** manual rotation begins at the solved angle without a jump

#### Scenario: Solve automatic and manual rotation [puppet-rotation]
- **WHEN** pins move with Auto rotation or a pin is set to a manual angle
- **THEN** the pin-local rotations follow the documented solve and pin positions remain exact

#### Scenario: Render a folded mesh [puppet-fold]
- **WHEN** folded triangles overlap and the user changes their owners' pin depth
- **THEN** the front triangle wins with deterministic ties and preview and commit use the same ordering

#### Scenario: Sample a warped raster and mask [puppet-sampling]
- **WHEN** a mesh maps pixel centers across a translucent raster edge or outside a mask
- **THEN** the result uses premultiplied bilinear raster sampling, the mask's default exterior, and the transformed bounds consistently in preview and commit

### Requirement: Publish a single raster edit
The editor SHALL leave source pixels and history untouched until Apply. A changed result SHALL become one undoable edit of the targeted surface. Cancel, stale targets, identity, and refused output SHALL leave history and pixels untouched.

#### Scenario: Cancel, apply, and undo a mesh edit [puppet-transaction]
- **WHEN** the user cancels a preview, applies a changed mesh, and undoes the operation
- **THEN** cancel leaves the original intact, apply commits one history entry, and undo restores the original

#### Scenario: Warp a raster mask without modifying layer color [puppet-mask]
- **WHEN** the user applies a deformation to a targeted ordinary raster user mask
- **THEN** the mask alone changes and undo restores its original geometry and samples

#### Scenario: Refuse oversized or identity output [puppet-bound]
- **WHEN** the source or output exceeds the workspace limits, or the user applies an identity mesh
- **THEN** no partial edit or history entry is created

#### Scenario: Refuse a stale target [puppet-stale]
- **WHEN** the source document, target binding, revision, or eligibility changes before Apply
- **THEN** Apply leaves pixels, geometry, and history untouched
