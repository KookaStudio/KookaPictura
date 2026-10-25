# Spec Delta

## ADDED Requirements

### Requirement: Selection-tool canvas menu

When a selection tool is active — rectangular marquee, elliptical marquee, lasso,
polygonal lasso, magnetic lasso, Magic Wand, or Quick Selection — and the clicked
document has a non-empty selection, a canvas right-click SHALL open a selection
menu at the pointer acting on the clicked document. The menu SHALL offer
Deselect, Inverse, Feather…, and Refine Edge…, reusing the registered
Select commands. With no active selection the tool SHALL open no selection menu,
and a click on the pasteboard outside the document SHALL open the
canvas-background menu instead. Transform Selection SHALL NOT be offered until the
selection-border transform exists.

#### Scenario: A selection opens the menu [sel_menu_opens]

- **WHEN** a selection tool is active with a non-empty selection and the user
  right-clicks over the document
- **THEN** the selection menu opens at the pointer with Deselect, Inverse,
  Feather…, and Refine Edge…

#### Scenario: The entries act on the clicked document [sel_menu_commands]

- **WHEN** the user chooses Deselect, Inverse, Feather…, or Refine Edge… from
  the selection menu
- **THEN** that Select command runs on the document that was clicked

#### Scenario: No selection opens the background menu on the pasteboard [sel_menu_no_selection]

- **WHEN** a selection tool is active with no selection and the user right-clicks
  on the pasteboard outside the document
- **THEN** no selection menu opens and the canvas-background menu opens instead
