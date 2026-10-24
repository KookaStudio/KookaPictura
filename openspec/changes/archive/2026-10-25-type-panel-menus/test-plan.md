# Test plan

Qt suites run offscreen through CTest.

## ui/type-panel-menus

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Character menu lists the CS6 rows | Open the live Character panel menu on a type layer and sync it. | The top-level rows are in CS6 order and the OpenType submenu has all eight features. | `tst_character_paragraph_panels::characterMenuMatchesCs6` |
| Character menu check states follow the layer | Select a layer with Faux Bold and Fractional Widths on and an OpenType feature on; sync the menu. | Faux Bold and Fractional Widths are checked, System Layout is unchecked, and the OpenType row matches. | `tst_character_paragraph_panels::characterMenuCheckStates` |
| A style toggle applies from the menu | Trigger Faux Bold, then undo. | The layer flips, one state is recorded, and the button follows. | `tst_character_paragraph_panels::characterMenuToggles` |
| Fractional Widths and System Layout are inverses | Trigger Fractional Widths off, trigger System Layout on, undo each. | The layer and check states follow the inverse, one state per trigger. | `tst_character_paragraph_panels::characterMenuToggles` |
| Reset Character restores defaults | Style the layer, trigger Reset Character, then undo. | Resettable attributes return to defaults; family/style/size/colour/text stay; the undo restores the style. | `tst_character_paragraph_panels::characterMenuReset` |
| Paragraph menu lists the CS6 rows | Open the live Paragraph panel menu on a type layer and sync it. | The rows are in CS6 order with the grouping separators. | `tst_character_paragraph_panels::paragraphMenuMatchesCs6` |
| Paragraph menu check states follow the layer | Select a layer with hanging off and the single-line composer; sync the menu. | Hanging is unchecked and Single-line Composer is checked. | `tst_character_paragraph_panels::paragraphMenuCheckStates` |
| A paragraph toggle applies from the menu | Trigger Roman Hanging Punctuation, then undo. | The layer's flag flips, one state is recorded, and the checkbox follows. | `tst_character_paragraph_panels::paragraphMenuToggles` |
| Composer rows are exclusive | Trigger Every-line Composer while Single-line is on, then undo. | The layer and menu show every-line; one undo restores single-line. | `tst_character_paragraph_panels::paragraphMenuToggles` |
| Reset Paragraph restores defaults | Style the paragraph, trigger Reset Paragraph, then undo. | Paragraph attributes return to defaults; character/text stay; the undo restores. | `tst_character_paragraph_panels::paragraphMenuReset` |
| The Justification item opens the dialog | Trigger Justification… and accept the dialog. | The dialog opens and accepting records one state. | `tst_character_paragraph_panels::justificationDialogRoundTrips` |
| Unimplemented rows are disabled | Open either live menu. | No Break, Hyphenation…, Middle Eastern Features, and the layout/feature-set rows are disabled with the not-implemented tooltip. | `tst_character_paragraph_panels::menuDisabledRows` |
| Close entries are last | Open either live menu. | Close and Close Tab Group are the final two non-separator entries. | `tst_character_paragraph_panels::menuCloseEntriesLast` |
